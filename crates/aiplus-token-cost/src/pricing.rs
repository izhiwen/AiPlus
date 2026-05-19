use crate::embedded::{EMBEDDED_PRICES, EMBEDDED_SOURCE};
use crate::error::Result;
use anyhow::{anyhow, Context};
use serde::Deserialize;
use serde_json::Value;
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{Duration as StdDuration, SystemTime};

pub const LITELLM_PRICING_URL: &str =
    "https://raw.githubusercontent.com/BerriAI/litellm/main/model_prices_and_context_window.json";
const CACHE_MAX_AGE: StdDuration = StdDuration::from_secs(24 * 60 * 60);
type PricingEntries = HashMap<(String, String), PricePerToken>;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PricePerToken {
    pub input_usd: f64,
    pub output_usd: f64,
}

#[derive(Debug, Clone)]
pub struct PricingTable {
    entries: PricingEntries,
    source: String,
    warnings: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct PricingLoadOptions {
    pub fetch_enabled: bool,
    pub cache_dir: Option<PathBuf>,
    pub pricing_url: Option<String>,
}

impl Default for PricingLoadOptions {
    fn default() -> Self {
        Self {
            fetch_enabled: true,
            cache_dir: None,
            pricing_url: None,
        }
    }
}

impl PricingTable {
    pub fn load(project_root: &Path) -> Self {
        Self::load_with_options(project_root, PricingLoadOptions::default())
    }

    pub fn load_with_options(project_root: &Path, options: PricingLoadOptions) -> Self {
        let mut warnings = Vec::new();
        let (mut table, source) =
            load_base_table(&options, &mut warnings).unwrap_or_else(|error| {
                warnings.push(format!("pricing fallback to embedded: {error}"));
                (embedded_entries(), EMBEDDED_SOURCE.to_string())
            });

        if let Err(error) = apply_project_override(project_root, &mut table) {
            warnings.push(format!("pricing override ignored: {error}"));
        }

        Self {
            entries: table,
            source,
            warnings,
        }
    }

    pub fn embedded() -> Self {
        Self {
            entries: embedded_entries(),
            source: EMBEDDED_SOURCE.to_string(),
            warnings: Vec::new(),
        }
    }

    pub fn lookup(&self, provider: &str, model: &str) -> Option<&PricePerToken> {
        let provider = normalize(provider);
        let model = normalize_model(model);
        self.entries
            .get(&(provider.clone(), model.clone()))
            .or_else(|| {
                self.entries
                    .get(&(provider.clone(), strip_model_prefix(&model)))
            })
            .or_else(|| {
                infer_provider(&model)
                    .and_then(|inferred| self.entries.get(&(inferred, model.clone())))
            })
    }

    pub fn source(&self) -> &str {
        &self.source
    }

    pub fn warnings(&self) -> &[String] {
        &self.warnings
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }
}

fn load_base_table(
    options: &PricingLoadOptions,
    warnings: &mut Vec<String>,
) -> Result<(PricingEntries, String)> {
    let cache = cache_file(options)?;
    if is_cache_fresh(&cache) {
        let text =
            fs::read_to_string(&cache).with_context(|| format!("read {}", cache.display()))?;
        return parse_litellm_json(&text).map(|table| (table, "litellm_cache".to_string()));
    }

    if options.fetch_enabled {
        match fetch_litellm_json(options) {
            Ok(text) => {
                let table = parse_litellm_json(&text)?;
                if let Some(parent) = cache.parent() {
                    let _ = fs::create_dir_all(parent);
                }
                let _ = fs::write(&cache, text);
                return Ok((table, "litellm_fetch".to_string()));
            }
            Err(error) => warnings.push(format!("litellm fetch failed: {error}")),
        }
    }

    if cache.exists() {
        let text =
            fs::read_to_string(&cache).with_context(|| format!("read {}", cache.display()))?;
        return parse_litellm_json(&text).map(|table| (table, "litellm_stale_cache".to_string()));
    }

    Ok((embedded_entries(), EMBEDDED_SOURCE.to_string()))
}

fn parse_litellm_json(text: &str) -> Result<PricingEntries> {
    let value: Value = serde_json::from_str(text)?;
    let object = value
        .as_object()
        .ok_or_else(|| anyhow!("LiteLLM pricing JSON root is not an object"))?;
    let mut entries = HashMap::new();
    for (model_key, details) in object {
        if model_key == "sample_spec" {
            continue;
        }
        let Some(details) = details.as_object() else {
            continue;
        };
        let Some(input_usd) =
            price_field(details, "input_cost_per_token", "input_cost_per_1m_tokens")
        else {
            continue;
        };
        let output_usd = price_field(
            details,
            "output_cost_per_token",
            "output_cost_per_1m_tokens",
        )
        .unwrap_or(0.0);
        let provider = details
            .get("litellm_provider")
            .or_else(|| details.get("provider"))
            .and_then(Value::as_str)
            .map(str::to_string)
            .or_else(|| infer_provider(model_key))
            .unwrap_or_else(|| "unknown".to_string());
        insert_price(&mut entries, &provider, model_key, input_usd, output_usd);
        if let Some(model) = details.get("model").and_then(Value::as_str) {
            insert_price(&mut entries, &provider, model, input_usd, output_usd);
        }
    }
    if entries.is_empty() {
        return Err(anyhow!(
            "LiteLLM pricing JSON contained no usable token prices"
        ));
    }
    Ok(entries)
}

fn price_field(
    details: &serde_json::Map<String, Value>,
    per_token_key: &str,
    per_1m_key: &str,
) -> Option<f64> {
    details
        .get(per_token_key)
        .and_then(Value::as_f64)
        .or_else(|| {
            details
                .get(per_1m_key)
                .and_then(Value::as_f64)
                .map(|v| v / 1_000_000.0)
        })
}

fn insert_price(
    entries: &mut PricingEntries,
    provider: &str,
    model: &str,
    input_usd: f64,
    output_usd: f64,
) {
    if provider == "unknown" || model.trim().is_empty() {
        return;
    }
    let price = PricePerToken {
        input_usd,
        output_usd,
    };
    let provider = normalize(provider);
    let model = normalize_model(model);
    entries.insert((provider.clone(), model.clone()), price);
    entries.insert((provider, strip_model_prefix(&model)), price);
}

fn embedded_entries() -> PricingEntries {
    let mut entries = HashMap::new();
    for (provider, model, price) in EMBEDDED_PRICES {
        insert_price(
            &mut entries,
            provider,
            model,
            price.input_usd,
            price.output_usd,
        );
    }
    entries
}

#[derive(Debug, Deserialize)]
struct OverrideFile {
    #[serde(default)]
    price: Vec<OverridePrice>,
}

#[derive(Debug, Deserialize)]
struct OverridePrice {
    provider: String,
    model: String,
    input_usd_per_token: Option<f64>,
    output_usd_per_token: Option<f64>,
    input_usd_per_1m_tokens: Option<f64>,
    output_usd_per_1m_tokens: Option<f64>,
}

fn apply_project_override(project_root: &Path, entries: &mut PricingEntries) -> Result<()> {
    let path = project_root.join(".aiplus/pricing.toml");
    if !path.exists() {
        return Ok(());
    }
    let text = fs::read_to_string(&path).with_context(|| format!("read {}", path.display()))?;
    let overrides: OverrideFile = toml::from_str(&text)?;
    for price in overrides.price {
        let input = price
            .input_usd_per_token
            .or_else(|| price.input_usd_per_1m_tokens.map(|v| v / 1_000_000.0))
            .ok_or_else(|| anyhow!("override missing input price for {}", price.model))?;
        let output = price
            .output_usd_per_token
            .or_else(|| price.output_usd_per_1m_tokens.map(|v| v / 1_000_000.0))
            .ok_or_else(|| anyhow!("override missing output price for {}", price.model))?;
        insert_price(entries, &price.provider, &price.model, input, output);
    }
    Ok(())
}

fn fetch_litellm_json(options: &PricingLoadOptions) -> Result<String> {
    let url = options
        .pricing_url
        .clone()
        .or_else(|| std::env::var("AIPLUS_TOKEN_COST_PRICING_URL").ok())
        .unwrap_or_else(|| LITELLM_PRICING_URL.to_string());
    if let Some(path) = url.strip_prefix("file://") {
        return Ok(fs::read_to_string(path)?);
    }
    if command_exists("curl") {
        let output = Command::new("curl").args(["-fsSL", &url]).output()?;
        if !output.status.success() {
            return Err(anyhow!("curl failed for LiteLLM pricing URL"));
        }
        return Ok(String::from_utf8(output.stdout)?);
    }
    if command_exists("wget") {
        let output = Command::new("wget")
            .args(["-q", "-O", "-", &url])
            .output()?;
        if !output.status.success() {
            return Err(anyhow!("wget failed for LiteLLM pricing URL"));
        }
        return Ok(String::from_utf8(output.stdout)?);
    }
    Err(anyhow!("curl or wget is required to fetch LiteLLM pricing"))
}

fn command_exists(name: &str) -> bool {
    Command::new(name).arg("--version").output().is_ok()
}

fn cache_file(options: &PricingLoadOptions) -> Result<PathBuf> {
    if let Some(dir) = &options.cache_dir {
        return Ok(dir.join("pricing.json"));
    }
    let base = if let Ok(xdg) = std::env::var("XDG_CACHE_HOME") {
        PathBuf::from(xdg)
    } else if let Ok(home) = std::env::var("HOME") {
        PathBuf::from(home).join(".cache")
    } else {
        std::env::temp_dir()
    };
    Ok(base.join("aiplus-token-cost").join("pricing.json"))
}

fn is_cache_fresh(path: &Path) -> bool {
    let Ok(metadata) = fs::metadata(path) else {
        return false;
    };
    let Ok(modified) = metadata.modified() else {
        return false;
    };
    SystemTime::now()
        .duration_since(modified)
        .is_ok_and(|age| age <= CACHE_MAX_AGE)
}

fn normalize(value: &str) -> String {
    value.trim().to_ascii_lowercase()
}

pub fn normalize_model(value: &str) -> String {
    normalize(value)
}

fn strip_model_prefix(model: &str) -> String {
    model.rsplit(['/', '.']).next().unwrap_or(model).to_string()
}

pub fn infer_provider(model: &str) -> Option<String> {
    let model = normalize_model(model);
    if model.contains("claude") {
        Some("anthropic".to_string())
    } else if model.starts_with("gpt-")
        || model.starts_with("o1")
        || model.starts_with("o3")
        || model.starts_with("o4")
    {
        Some("openai".to_string())
    } else if model.contains("kimi") || model.contains("moonshot") {
        Some("moonshot".to_string())
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn embedded_lookup_covers_required_models() {
        let pricing = PricingTable::embedded();
        assert!(pricing.lookup("anthropic", "claude-sonnet-4-6").is_some());
        assert!(pricing.lookup("openai", "gpt-5").is_some());
        assert!(pricing.lookup("", "gpt-4o-mini").is_some());
    }

    #[test]
    fn project_override_wins_for_specific_model() {
        let temp = tempfile::tempdir().unwrap();
        fs::create_dir_all(temp.path().join(".aiplus")).unwrap();
        fs::write(
            temp.path().join(".aiplus/pricing.toml"),
            r#"
[[price]]
provider = "anthropic"
model = "claude-sonnet-4-6"
input_usd_per_1m_tokens = 10.0
output_usd_per_1m_tokens = 20.0
"#,
        )
        .unwrap();
        let pricing = PricingTable::load_with_options(
            temp.path(),
            PricingLoadOptions {
                fetch_enabled: false,
                cache_dir: Some(temp.path().join("cache")),
                pricing_url: None,
            },
        );
        let price = pricing.lookup("anthropic", "claude-sonnet-4-6").unwrap();
        assert_eq!(price.input_usd, 0.00001);
        assert_eq!(price.output_usd, 0.00002);
    }
}
