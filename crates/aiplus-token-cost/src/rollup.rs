use crate::error::Result;
use crate::pricing::{infer_provider, PricingTable};
use anyhow::{anyhow, Context};
use chrono::{DateTime, Duration, TimeZone, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::{BTreeMap, HashMap, HashSet};
use std::fs;
use std::path::Path;

#[derive(Debug, Clone, PartialEq)]
pub struct WindowSpec {
    pub label: String,
    pub duration: Duration,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct RollupResult {
    pub window_label: String,
    pub total_tokens: u64,
    pub total_usd: f64,
    pub top_tasks: Vec<TaskCost>,
    pub by_role: BTreeMap<String, RoleCost>,
    pub warnings: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct TaskCost {
    pub key: String,
    pub task_excerpt: String,
    pub role: String,
    pub provider: String,
    pub model: String,
    pub input_tokens: u64,
    pub output_tokens: u64,
    pub total_tokens: u64,
    pub usd: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
pub struct RoleCost {
    pub input_tokens: u64,
    pub output_tokens: u64,
    pub total_tokens: u64,
    pub usd: f64,
}

#[derive(Debug, Clone)]
struct ParsedEntry {
    timestamp: DateTime<Utc>,
    key: String,
    task_excerpt: String,
    role: String,
    provider: String,
    model: String,
    input_tokens: u64,
    output_tokens: u64,
    total_tokens: u64,
}

pub fn default_windows() -> Vec<WindowSpec> {
    vec![
        WindowSpec {
            label: "1h".to_string(),
            duration: Duration::hours(1),
        },
        WindowSpec {
            label: "8h".to_string(),
            duration: Duration::hours(8),
        },
        WindowSpec {
            label: "24h".to_string(),
            duration: Duration::hours(24),
        },
    ]
}

pub fn parse_window(label: &str) -> Result<WindowSpec> {
    let duration = match label {
        "1h" => Duration::hours(1),
        "8h" => Duration::hours(8),
        "24h" => Duration::hours(24),
        _ => return Err(anyhow!("unsupported token-cost window: {label}")),
    };
    Ok(WindowSpec {
        label: label.to_string(),
        duration,
    })
}

pub fn rollup_from_dispatch_log(
    log_path: &Path,
    pricing: &PricingTable,
    now: DateTime<Utc>,
    windows: &[WindowSpec],
    top_n: usize,
) -> Result<Vec<RollupResult>> {
    if !log_path.exists() {
        return Ok(windows
            .iter()
            .map(|window| empty_rollup(&window.label))
            .collect());
    }

    let body =
        fs::read_to_string(log_path).with_context(|| format!("read {}", log_path.display()))?;
    let mut parsed_entries = Vec::new();
    let mut parse_warnings = Vec::new();
    for (idx, line) in body.lines().enumerate() {
        if line.trim().is_empty() {
            continue;
        }
        match serde_json::from_str::<Value>(line)
            .map_err(anyhow::Error::from)
            .and_then(|value| parse_entry(idx + 1, &value))
        {
            Ok(entry) => parsed_entries.push(entry),
            Err(error) => parse_warnings.push(format!("line {} skipped: {error}", idx + 1)),
        }
    }

    let mut results = Vec::new();
    for window in windows {
        let min_timestamp = now - window.duration;
        let mut tasks: HashMap<String, TaskCost> = HashMap::new();
        let mut by_role: BTreeMap<String, RoleCost> = BTreeMap::new();
        let mut warnings = parse_warnings.clone();
        let mut unknown_prices = HashSet::new();
        let mut total_tokens = 0_u64;
        let mut total_usd = 0.0_f64;

        for entry in parsed_entries
            .iter()
            .filter(|entry| entry.timestamp >= min_timestamp && entry.timestamp <= now)
        {
            let usd = pricing
                .lookup(&entry.provider, &entry.model)
                .map(|price| {
                    entry.input_tokens as f64 * price.input_usd
                        + entry.output_tokens as f64 * price.output_usd
                })
                .unwrap_or_else(|| {
                    if entry.total_tokens > 0 {
                        unknown_prices.insert(format!("{}/{}", entry.provider, entry.model));
                    }
                    0.0
                });
            total_tokens += entry.total_tokens;
            total_usd += usd;

            let task = tasks.entry(entry.key.clone()).or_insert_with(|| TaskCost {
                key: entry.key.clone(),
                task_excerpt: entry.task_excerpt.clone(),
                role: entry.role.clone(),
                provider: entry.provider.clone(),
                model: entry.model.clone(),
                input_tokens: 0,
                output_tokens: 0,
                total_tokens: 0,
                usd: 0.0,
            });
            if task.role != entry.role {
                task.role = "multiple".to_string();
            }
            task.input_tokens += entry.input_tokens;
            task.output_tokens += entry.output_tokens;
            task.total_tokens += entry.total_tokens;
            task.usd += usd;

            let role = by_role.entry(entry.role.clone()).or_default();
            role.input_tokens += entry.input_tokens;
            role.output_tokens += entry.output_tokens;
            role.total_tokens += entry.total_tokens;
            role.usd += usd;
        }

        for unknown in unknown_prices {
            warnings.push(format!("unknown pricing for {unknown}; usd counted as 0"));
        }

        let mut top_tasks: Vec<TaskCost> = tasks.into_values().collect();
        top_tasks.sort_by(|a, b| {
            b.usd
                .partial_cmp(&a.usd)
                .unwrap_or(std::cmp::Ordering::Equal)
                .then_with(|| b.total_tokens.cmp(&a.total_tokens))
                .then_with(|| a.key.cmp(&b.key))
        });
        top_tasks.truncate(top_n);

        results.push(RollupResult {
            window_label: window.label.clone(),
            total_tokens,
            total_usd,
            top_tasks,
            by_role,
            warnings,
        });
    }

    Ok(results)
}

fn empty_rollup(label: &str) -> RollupResult {
    RollupResult {
        window_label: label.to_string(),
        total_tokens: 0,
        total_usd: 0.0,
        top_tasks: Vec::new(),
        by_role: BTreeMap::new(),
        warnings: Vec::new(),
    }
}

fn parse_entry(line_number: usize, value: &Value) -> Result<ParsedEntry> {
    let timestamp = string_field(value, &["timestamp", "createdAt", "created_at"])
        .ok_or_else(|| anyhow!("missing timestamp"))?;
    let timestamp = parse_timestamp(timestamp).ok_or_else(|| anyhow!("invalid timestamp"))?;
    let usage = value
        .get("usage_tokens")
        .or_else(|| value.get("usageTokens"))
        .or_else(|| value.get("usage"));
    let (input_tokens, output_tokens, total_tokens) = parse_usage(usage);
    let model = string_field(value, &["model", "modelName", "model_name", "pricingModel"])
        .or_else(|| {
            usage.and_then(|usage| string_field(usage, &["model", "modelName", "model_name"]))
        })
        .unwrap_or("unknown")
        .to_string();
    let provider = string_field(
        value,
        &["provider", "providerName", "provider_name", "runtime"],
    )
    .or_else(|| {
        usage.and_then(|usage| string_field(usage, &["provider", "providerName", "provider_name"]))
    })
    .map(str::to_string)
    .or_else(|| infer_provider(&model))
    .unwrap_or_else(|| "unknown".to_string());
    let task_excerpt = string_field(value, &["taskExcerpt", "task_excerpt", "task"])
        .unwrap_or("(no task excerpt)")
        .to_string();
    let key = string_field(
        value,
        &["decisionId", "decision_id", "dispatchId", "dispatch_id"],
    )
    .map(str::to_string)
    .unwrap_or_else(|| format!("line-{line_number}"));
    let role = string_field(value, &["role"])
        .unwrap_or_else(|| {
            if value.get("event").and_then(Value::as_str) == Some("coordinator_decision") {
                "coordinator"
            } else {
                "unknown"
            }
        })
        .to_string();

    Ok(ParsedEntry {
        timestamp,
        key,
        task_excerpt,
        role,
        provider,
        model,
        input_tokens,
        output_tokens,
        total_tokens,
    })
}

fn string_field<'a>(value: &'a Value, keys: &[&str]) -> Option<&'a str> {
    keys.iter()
        .find_map(|key| value.get(*key).and_then(Value::as_str))
        .filter(|value| !value.trim().is_empty())
}

fn parse_usage(usage: Option<&Value>) -> (u64, u64, u64) {
    let Some(usage) = usage.filter(|value| !value.is_null()) else {
        return (0, 0, 0);
    };
    let input = number_field(
        usage,
        &[
            "input_tokens",
            "inputTokens",
            "prompt_tokens",
            "promptTokens",
            "input",
        ],
    );
    let output = number_field(
        usage,
        &[
            "output_tokens",
            "outputTokens",
            "completion_tokens",
            "completionTokens",
            "output",
        ],
    );
    let total = number_field(usage, &["total_tokens", "totalTokens", "total"])
        .unwrap_or_else(|| input.unwrap_or(0) + output.unwrap_or(0));
    (input.unwrap_or(0), output.unwrap_or(0), total)
}

fn number_field(value: &Value, keys: &[&str]) -> Option<u64> {
    keys.iter().find_map(|key| {
        value.get(*key).and_then(|value| {
            value
                .as_u64()
                .or_else(|| value.as_i64().and_then(|v| u64::try_from(v).ok()))
                .or_else(|| value.as_f64().map(|v| v.max(0.0).round() as u64))
        })
    })
}

pub fn parse_timestamp(value: &str) -> Option<DateTime<Utc>> {
    if let Ok(parsed) = DateTime::parse_from_rfc3339(value) {
        return Some(parsed.with_timezone(&Utc));
    }
    if let Some(raw) = value
        .strip_prefix("unix-")
        .and_then(|v| v.strip_suffix("ms"))
    {
        if let Ok(ms) = raw.parse::<i64>() {
            return Utc.timestamp_millis_opt(ms).single();
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    fn write_log(path: &Path, now: DateTime<Utc>) {
        let line1 = serde_json::json!({
            "timestamp": now.to_rfc3339(),
            "dispatchId": "dispatch-1-engineer-a",
            "role": "engineer-a",
            "task": "implement payment flow",
            "provider": "anthropic",
            "model": "claude-sonnet-4-6",
            "usage_tokens": {"input_tokens": 1000, "output_tokens": 100}
        });
        let line2 = serde_json::json!({
            "timestamp": now.to_rfc3339(),
            "decisionId": "coord-1",
            "event": "coordinator_decision",
            "taskExcerpt": "missing usage",
            "usage_tokens": null
        });
        let line3 = serde_json::json!({
            "timestamp": (now - Duration::hours(2)).to_rfc3339(),
            "dispatchId": "dispatch-2-tech-writer",
            "role": "tech-writer",
            "task": "write docs",
            "provider": "openai",
            "model": "gpt-5",
            "usageTokens": {"inputTokens": 500, "outputTokens": 50}
        });
        fs::write(path, format!("{line1}\n{line2}\n{line3}\n")).unwrap();
    }

    #[test]
    fn rollup_counts_windows_top_tasks_and_roles() {
        let temp = tempfile::tempdir().unwrap();
        let log = temp.path().join("dispatch-log.jsonl");
        let now = Utc::now();
        write_log(&log, now);
        let pricing = PricingTable::embedded();
        let results = rollup_from_dispatch_log(&log, &pricing, now, &default_windows(), 5).unwrap();

        let one_hour = results.iter().find(|r| r.window_label == "1h").unwrap();
        assert_eq!(one_hour.total_tokens, 1100);
        assert_eq!(one_hour.by_role["engineer-a"].total_tokens, 1100);
        assert_eq!(one_hour.top_tasks[0].key, "dispatch-1-engineer-a");

        let eight_hour = results.iter().find(|r| r.window_label == "8h").unwrap();
        assert_eq!(eight_hour.total_tokens, 1650);
        assert!(eight_hour.by_role.contains_key("tech-writer"));
    }

    #[test]
    fn unknown_model_counts_tokens_with_zero_usd_warning() {
        let temp = tempfile::tempdir().unwrap();
        let log = temp.path().join("dispatch-log.jsonl");
        let now = Utc::now();
        let line = serde_json::json!({
            "timestamp": now.to_rfc3339(),
            "dispatchId": "dispatch-unknown",
            "role": "engineer-a",
            "provider": "unknown",
            "model": "future-model",
            "usage_tokens": {"input_tokens": 7, "output_tokens": 3}
        });
        fs::write(&log, format!("{line}\n")).unwrap();
        let results =
            rollup_from_dispatch_log(&log, &PricingTable::embedded(), now, &default_windows(), 5)
                .unwrap();
        assert_eq!(results[0].total_tokens, 10);
        assert_eq!(results[0].total_usd, 0.0);
        assert!(results[0].warnings[0].contains("unknown pricing"));
    }
}
