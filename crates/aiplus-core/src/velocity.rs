use crate::redaction::sensitive_findings;
use anyhow::{anyhow, Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};

pub const VELOCITY_SCHEMA_VERSION: &str = "1";
pub const VELOCITY_DIR_REL: &str = ".aiplus/velocity";
pub const ESTIMATES_JSONL: &str = "estimates.jsonl";
pub const RUNS_JSONL: &str = "runs.jsonl";
pub const ANCHOR_SIGNALS_JSONL: &str = "anchor-signals.jsonl";
pub const RARE_CASES_JSONL: &str = "rare-cases.jsonl";
pub const MULTIPLIERS_JSON: &str = "multipliers.json";
pub const AGGREGATES_JSON: &str = "aggregates.json";
pub const ROTATION_STATE_JSON: &str = "rotation-state.json";
pub const CONFIG_JSON: &str = "config.json";

pub const DEFAULT_MAX_RECORDS: usize = 200;
pub const DEFAULT_RARE_CASE_MAX_RECORDS: usize = 20;
pub const DEFAULT_MAX_BYTES_PER_JSONL: usize = 1_048_576;
pub const DEFAULT_RETAIN_DAYS: u64 = 90;
pub const DEFAULT_MIN_BUCKET_SAMPLES: usize = 8;

// ---------------------------------------------------------------------------
// Config
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase", default)]
pub struct VelocityConfig {
    pub schema_version: String,
    pub max_records: usize,
    pub rare_case_max_records: usize,
    pub max_bytes_per_jsonl: usize,
    pub retain_days: u64,
    pub min_bucket_samples: usize,
    pub raw_content_allowed: bool,
    pub memory_integration: String,
}

impl Default for VelocityConfig {
    fn default() -> Self {
        Self {
            schema_version: VELOCITY_SCHEMA_VERSION.to_string(),
            max_records: DEFAULT_MAX_RECORDS,
            rare_case_max_records: DEFAULT_RARE_CASE_MAX_RECORDS,
            max_bytes_per_jsonl: DEFAULT_MAX_BYTES_PER_JSONL,
            retain_days: DEFAULT_RETAIN_DAYS,
            min_bucket_samples: DEFAULT_MIN_BUCKET_SAMPLES,
            raw_content_allowed: false,
            memory_integration: "disabled".to_string(),
        }
    }
}

// ---------------------------------------------------------------------------
// Record structs
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase", default)]
pub struct EstimateRecord {
    pub schema_version: String,
    pub id: String,
    pub task_id: String,
    pub created_at: String,
    pub project_id: String,
    pub task_type: String,
    pub repo_area: String,
    pub agent_role: String,
    pub runtime: String,
    pub model: String,
    pub workflow_level: String,
    pub estimate_basis: String,
    pub human_baseline_minutes: u32,
    pub human_baseline_source: String,
    pub human_estimate_minutes: u32,
    pub ai_native_estimate_p50_minutes: u32,
    pub ai_native_estimate_p90_minutes: u32,
    pub confidence: String,
    pub matched_records: usize,
    pub human_anchor_signals: Vec<String>,
    pub stop_when_done: bool,
}

impl Default for EstimateRecord {
    fn default() -> Self {
        Self {
            schema_version: VELOCITY_SCHEMA_VERSION.to_string(),
            id: String::new(),
            task_id: String::new(),
            created_at: String::new(),
            project_id: String::new(),
            task_type: String::new(),
            repo_area: String::new(),
            agent_role: String::new(),
            runtime: String::new(),
            model: String::new(),
            workflow_level: String::new(),
            estimate_basis: String::new(),
            human_baseline_minutes: 0,
            human_baseline_source: String::new(),
            human_estimate_minutes: 0,
            ai_native_estimate_p50_minutes: 0,
            ai_native_estimate_p90_minutes: 0,
            confidence: String::new(),
            matched_records: 0,
            human_anchor_signals: Vec::new(),
            stop_when_done: false,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase", default)]
pub struct RunRecord {
    pub schema_version: String,
    pub id: String,
    pub estimate_id: String,
    pub task_id: String,
    pub created_at: String,
    pub project_id: String,
    pub task_type: String,
    pub repo_area: String,
    pub agent_role: String,
    pub runtime: String,
    pub model: String,
    pub workflow_level: String,
    pub original_estimate_minutes: u32,
    pub human_baseline_minutes: u32,
    pub actual_active_minutes: u32,
    pub actual_time_source: String,
    pub wall_clock_minutes: u32,
    pub tool_wait_minutes: u32,
    pub blocked_minutes: u32,
    pub outcome: String,
    pub verification_depth: String,
    pub quality_verdict: String,
    pub rework_count: u32,
    pub owner_gate_hit: bool,
    pub overestimate_ratio: f64,
    pub human_time_bias: bool,
    pub slow_reason: String,
    pub redaction_status: String,
    pub raw_content_stored: bool,
    pub secret_values_stored: bool,
    pub memory_integration: String,
}

impl Default for RunRecord {
    fn default() -> Self {
        Self {
            schema_version: VELOCITY_SCHEMA_VERSION.to_string(),
            id: String::new(),
            estimate_id: String::new(),
            task_id: String::new(),
            created_at: String::new(),
            project_id: String::new(),
            task_type: String::new(),
            repo_area: String::new(),
            agent_role: String::new(),
            runtime: String::new(),
            model: String::new(),
            workflow_level: String::new(),
            original_estimate_minutes: 0,
            human_baseline_minutes: 0,
            actual_active_minutes: 0,
            actual_time_source: String::new(),
            wall_clock_minutes: 0,
            tool_wait_minutes: 0,
            blocked_minutes: 0,
            outcome: String::new(),
            verification_depth: String::new(),
            quality_verdict: String::new(),
            rework_count: 0,
            owner_gate_hit: false,
            overestimate_ratio: 0.0,
            human_time_bias: false,
            slow_reason: String::new(),
            redaction_status: String::new(),
            raw_content_stored: false,
            secret_values_stored: false,
            memory_integration: "disabled".to_string(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase", default)]
pub struct AnchorSignalRecord {
    pub schema_version: String,
    pub id: String,
    pub task_id: String,
    pub created_at: String,
    pub signal_type: String,
    pub description: String,
    pub human_estimate_minutes: u32,
    pub ai_native_prior_minutes: u32,
    pub confidence: String,
}

impl Default for AnchorSignalRecord {
    fn default() -> Self {
        Self {
            schema_version: VELOCITY_SCHEMA_VERSION.to_string(),
            id: String::new(),
            task_id: String::new(),
            created_at: String::new(),
            signal_type: String::new(),
            description: String::new(),
            human_estimate_minutes: 0,
            ai_native_prior_minutes: 0,
            confidence: String::new(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase", default)]
pub struct RareCaseRecord {
    pub schema_version: String,
    pub id: String,
    pub task_id: String,
    pub created_at: String,
    pub case_type: String,
    pub description: String,
    pub overestimate_ratio: f64,
    pub actual_active_minutes: u32,
    pub original_estimate_minutes: u32,
    pub outcome: String,
}

impl Default for RareCaseRecord {
    fn default() -> Self {
        Self {
            schema_version: VELOCITY_SCHEMA_VERSION.to_string(),
            id: String::new(),
            task_id: String::new(),
            created_at: String::new(),
            case_type: String::new(),
            description: String::new(),
            overestimate_ratio: 0.0,
            actual_active_minutes: 0,
            original_estimate_minutes: 0,
            outcome: String::new(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase", default)]
pub struct BucketData {
    pub model_key: String,
    pub model_family: String,
    pub sample_count: usize,
    pub actual_ai_p50_minutes: u32,
    pub actual_ai_p80_minutes: u32,
    pub overestimate_ratio_p50: f64,
    pub observed_speedup_p50: f64,
    pub human_bias_rate: f64,
    pub confidence: String,
    pub stale_for_current_model: bool,
    pub last_updated_at: String,
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub recent_actual_minutes: Vec<u32>,
}

impl Default for BucketData {
    fn default() -> Self {
        Self {
            model_key: String::new(),
            model_family: String::new(),
            sample_count: 0,
            actual_ai_p50_minutes: 0,
            actual_ai_p80_minutes: 0,
            overestimate_ratio_p50: 0.0,
            observed_speedup_p50: 0.0,
            human_bias_rate: 0.0,
            confidence: String::new(),
            stale_for_current_model: false,
            last_updated_at: String::new(),
            recent_actual_minutes: Vec::new(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
#[serde(rename_all = "camelCase", default)]
pub struct MultiplierBucket {
    pub schema_version: String,
    pub updated_at: String,
    pub buckets: BTreeMap<String, BucketData>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
#[serde(rename_all = "camelCase", default)]
pub struct Aggregates {
    pub schema_version: String,
    pub updated_at: String,
    pub total_estimates: usize,
    pub total_runs: usize,
    pub total_anchor_signals: usize,
    pub total_rare_cases: usize,
    pub median_overestimate_ratio: f64,
    pub human_time_bias_rate: f64,
    pub last_rotation_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
#[serde(rename_all = "camelCase", default)]
pub struct RotationState {
    pub schema_version: String,
    pub last_rotation_at: String,
    pub records_pruned: usize,
    pub rare_cases_pruned: usize,
    pub rotation_runs: usize,
}

// ---------------------------------------------------------------------------
// Time parser
// ---------------------------------------------------------------------------

pub fn parse_duration(input: &str) -> Result<u32> {
    let trimmed = input.trim();
    if trimmed.is_empty() {
        return Err(anyhow!("duration is empty"));
    }

    // Try integer minutes first: "90m"
    if let Some(min_str) = trimmed.strip_suffix('m') {
        let min_str = min_str.trim();
        if let Ok(minutes) = min_str.parse::<f64>() {
            if minutes < 0.0 {
                return Err(anyhow!("negative duration not allowed"));
            }
            return Ok(minutes.round() as u32);
        }
    }

    // Try hours: "1h", "1.5h", "5h"
    if let Some(h_str) = trimmed.strip_suffix('h') {
        let h_str = h_str.trim();
        if let Ok(hours) = h_str.parse::<f64>() {
            if hours < 0.0 {
                return Err(anyhow!("negative duration not allowed"));
            }
            return Ok((hours * 60.0).round() as u32);
        }
    }

    // Try plain integer as minutes (fallback)
    if let Ok(minutes) = trimmed.parse::<f64>() {
        if minutes < 0.0 {
            return Err(anyhow!("negative duration not allowed"));
        }
        return Ok(minutes.round() as u32);
    }

    Err(anyhow!(
        "invalid duration format: expected like 20m, 1h, 1.5h, 5h, 90m"
    ))
}

// ---------------------------------------------------------------------------
// Storage helpers
// ---------------------------------------------------------------------------

pub fn velocity_dir(root: &Path) -> PathBuf {
    root.join(VELOCITY_DIR_REL)
}

pub fn init_velocity(root: &Path) -> Result<()> {
    let dir = velocity_dir(root);
    fs::create_dir_all(&dir)?;

    let config_path = dir.join(CONFIG_JSON);
    if !config_path.exists() {
        let config = VelocityConfig::default();
        let json = serde_json::to_string_pretty(&config)?;
        crate::write_file_atomic(&config_path, json.as_bytes())?;
    }

    for file in [
        ESTIMATES_JSONL,
        RUNS_JSONL,
        ANCHOR_SIGNALS_JSONL,
        RARE_CASES_JSONL,
    ] {
        let path = dir.join(file);
        if !path.exists() {
            fs::write(&path, b"")?;
        }
    }

    for file in [MULTIPLIERS_JSON, AGGREGATES_JSON, ROTATION_STATE_JSON] {
        let path = dir.join(file);
        if !path.as_path().exists() {
            let empty = match file {
                MULTIPLIERS_JSON => serde_json::to_string_pretty(&MultiplierBucket::default())?,
                AGGREGATES_JSON => serde_json::to_string_pretty(&Aggregates::default())?,
                ROTATION_STATE_JSON => serde_json::to_string_pretty(&RotationState::default())?,
                _ => String::new(),
            };
            crate::write_file_atomic(&path, empty.as_bytes())?;
        }
    }

    Ok(())
}

pub fn read_config(root: &Path) -> Result<VelocityConfig> {
    let path = velocity_dir(root).join(CONFIG_JSON);
    if !path.exists() {
        return Ok(VelocityConfig::default());
    }
    let text =
        fs::read_to_string(&path).with_context(|| format!("read config {}", path.display()))?;
    let config: VelocityConfig =
        serde_json::from_str(&text).with_context(|| format!("parse config {}", path.display()))?;
    Ok(config)
}

pub fn write_config(root: &Path, config: &VelocityConfig) -> Result<()> {
    let path = velocity_dir(root).join(CONFIG_JSON);
    let json = serde_json::to_string_pretty(config)?;
    crate::write_file_atomic(&path, json.as_bytes())?;
    Ok(())
}

pub fn append_jsonl<T: Serialize>(root: &Path, filename: &str, record: &T) -> Result<()> {
    let path = velocity_dir(root).join(filename);
    let line = serde_json::to_string(record)?;
    crate::append_jsonl_atomic(&path, &line)?;
    Ok(())
}

pub fn read_jsonl<T: for<'de> Deserialize<'de>>(root: &Path, filename: &str) -> Result<Vec<T>> {
    let path = velocity_dir(root).join(filename);
    if !path.exists() {
        return Ok(Vec::new());
    }
    let text = fs::read_to_string(&path).with_context(|| format!("read {}", path.display()))?;
    let mut records = Vec::new();
    for (idx, line) in text.lines().enumerate() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        let record: T = serde_json::from_str(line)
            .with_context(|| format!("parse {} line {}", path.display(), idx + 1))?;
        records.push(record);
    }
    Ok(records)
}

pub fn rewrite_jsonl<T: Serialize>(root: &Path, filename: &str, records: &[T]) -> Result<()> {
    let path = velocity_dir(root).join(filename);
    crate::rewrite_jsonl_atomic(&path, records)?;
    Ok(())
}

pub fn write_json<T: Serialize>(root: &Path, filename: &str, value: &T) -> Result<()> {
    let path = velocity_dir(root).join(filename);
    let json = serde_json::to_string_pretty(value)?;
    crate::write_file_atomic(&path, json.as_bytes())?;
    Ok(())
}

pub fn read_json<T: for<'de> Deserialize<'de>>(root: &Path, filename: &str) -> Result<T> {
    let path = velocity_dir(root).join(filename);
    let text = fs::read_to_string(&path).with_context(|| format!("read {}", path.display()))?;
    let value: T =
        serde_json::from_str(&text).with_context(|| format!("parse {}", path.display()))?;
    Ok(value)
}

// ---------------------------------------------------------------------------
// ID helpers
// ---------------------------------------------------------------------------

pub fn generate_estimate_id() -> String {
    format!("est_{}", crate::epoch_millis())
}

pub fn generate_run_id() -> String {
    format!("run_{}", crate::epoch_millis())
}

pub fn generate_signal_id() -> String {
    format!("sig_{}", crate::epoch_millis())
}

pub fn generate_rare_case_id() -> String {
    format!("rare_{}", crate::epoch_millis())
}

pub fn now_iso() -> String {
    let now = std::time::SystemTime::now();
    let duration = now
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default();
    let secs = duration.as_secs();
    let nanos = duration.subsec_nanos();
    let millis = nanos / 1_000_000;
    let dt = time_from_epoch(secs);
    format!(
        "{:04}-{:02}-{:02}T{:02}:{:02}:{:02}.{:03}Z",
        dt.year, dt.month, dt.day, dt.hour, dt.minute, dt.second, millis
    )
}

struct SimpleDateTime {
    year: i32,
    month: u8,
    day: u8,
    hour: u8,
    minute: u8,
    second: u8,
}

fn time_from_epoch(mut secs: u64) -> SimpleDateTime {
    const DAYS_IN_MONTH: [u8; 12] = [31, 28, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31];
    let mut year = 1970i32;
    loop {
        let is_leap = (year % 4 == 0 && year % 100 != 0) || (year % 400 == 0);
        let secs_in_year = if is_leap { 31_622_400 } else { 31_536_000 };
        if secs >= secs_in_year as u64 {
            secs -= secs_in_year as u64;
            year += 1;
        } else {
            break;
        }
    }
    let is_leap = (year % 4 == 0 && year % 100 != 0) || (year % 400 == 0);
    let mut month = 0usize;
    loop {
        let days = if month == 1 && is_leap {
            29
        } else {
            DAYS_IN_MONTH[month] as u64
        };
        let secs_in_month = days * 86_400;
        if secs >= secs_in_month {
            secs -= secs_in_month;
            month += 1;
        } else {
            break;
        }
    }
    let day = (secs / 86_400) + 1;
    secs %= 86_400;
    let hour = secs / 3600;
    secs %= 3600;
    let minute = secs / 60;
    let second = secs % 60;
    SimpleDateTime {
        year,
        month: (month + 1) as u8,
        day: day as u8,
        hour: hour as u8,
        minute: minute as u8,
        second: second as u8,
    }
}

// ---------------------------------------------------------------------------
// Bias detection
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq)]
pub struct BiasResult {
    pub bias_type: String,
    pub overestimate_ratio: f64,
    pub human_time_bias_found: bool,
    pub human_time_bias_confidence: String,
    pub human_baseline_status: String,
    pub next_estimate_adjustment: String,
    pub fast_finish_triggered: bool,
    pub fast_finish_calibration: Option<FastFinishCalibration>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct FastFinishCalibration {
    pub original_estimate_minutes: u32,
    pub actual_time_minutes: u32,
    pub acceptance_status: String,
    pub error_ratio: f64,
    pub cause: String,
    pub next_estimate_adjustment: String,
}

pub fn detect_bias(estimate: &EstimateRecord, run: &RunRecord) -> BiasResult {
    let overestimate_ratio = if run.actual_active_minutes > 0 {
        estimate.human_estimate_minutes as f64 / run.actual_active_minutes as f64
    } else {
        0.0
    };

    let has_human_baseline = estimate.human_baseline_minutes > 0;

    // Human-time bias detection rules
    let ai_native_prior = estimate.ai_native_estimate_p50_minutes;
    let no_major_slow_reason = run.slow_reason == "none" || run.slow_reason.is_empty();
    let quality_pass = run.quality_verdict == "pass" || run.outcome == "pass";

    let human_time_bias_found = if run.actual_active_minutes > 0 {
        let rule1 = estimate.human_estimate_minutes >= 2 * ai_native_prior;
        let rule2 = run.actual_active_minutes <= ai_native_prior + (ai_native_prior / 4); // p75-ish
        let rule3 = no_major_slow_reason;
        let rule4 = quality_pass;
        rule1 && rule2 && rule3 && rule4
    } else {
        false
    };

    let human_time_bias_confidence = if human_time_bias_found {
        if has_human_baseline && overestimate_ratio >= 5.0 {
            "high"
        } else if has_human_baseline {
            "medium"
        } else {
            "low"
        }
    } else {
        "none"
    };

    let bias_type = if overestimate_ratio >= 2.0 {
        "OVERESTIMATE"
    } else if overestimate_ratio <= 0.5 {
        "UNDERESTIMATE"
    } else {
        "NEUTRAL"
    };

    let next_estimate_adjustment = if human_time_bias_found {
        "shorten_same_type"
    } else if bias_type == "UNDERESTIMATE" {
        "lengthen_same_type"
    } else {
        "none"
    };

    // Fast-finish calibration
    let fast_finish_triggered = run.actual_active_minutes > 0
        && (run.actual_active_minutes as f64)
            < (estimate.ai_native_estimate_p50_minutes as f64 * 0.5);

    let fast_finish_calibration = if fast_finish_triggered {
        let cause = if human_time_bias_found {
            "human_time_anchoring"
        } else {
            "scope_smaller_than_expected"
        };
        Some(FastFinishCalibration {
            original_estimate_minutes: estimate.human_estimate_minutes,
            actual_time_minutes: run.actual_active_minutes,
            acceptance_status: if quality_pass {
                "pass".to_string()
            } else {
                "needs_fix".to_string()
            },
            error_ratio: overestimate_ratio,
            cause: cause.to_string(),
            next_estimate_adjustment: next_estimate_adjustment.to_string(),
        })
    } else {
        None
    };

    BiasResult {
        bias_type: bias_type.to_string(),
        overestimate_ratio,
        human_time_bias_found,
        human_time_bias_confidence: human_time_bias_confidence.to_string(),
        human_baseline_status: if has_human_baseline {
            "present"
        } else {
            "missing"
        }
        .to_string(),
        next_estimate_adjustment: next_estimate_adjustment.to_string(),
        fast_finish_triggered,
        fast_finish_calibration,
    }
}

pub fn has_meaningful_slow_reason(reason: &str) -> bool {
    let normalized = reason.trim().to_ascii_lowercase();
    !normalized.is_empty() && normalized != "none"
}

pub fn is_rare_case(run: &RunRecord) -> bool {
    run.overestimate_ratio >= 5.0
        || run.owner_gate_hit
        || run.rework_count > 0
        || has_meaningful_slow_reason(&run.slow_reason)
        || run.outcome == "blocked"
}

pub fn classify_rare_case(run: &RunRecord) -> Option<RareCaseRecord> {
    if !is_rare_case(run) {
        return None;
    }

    let case_type = if run.overestimate_ratio >= 5.0 {
        "large_overestimate"
    } else if run.owner_gate_hit {
        "owner_gate"
    } else if run.rework_count > 0 {
        "rework"
    } else if has_meaningful_slow_reason(&run.slow_reason) {
        "slow_reason"
    } else if run.outcome == "blocked" {
        "blocked"
    } else {
        "unusual"
    };

    Some(RareCaseRecord {
        schema_version: VELOCITY_SCHEMA_VERSION.to_string(),
        id: generate_rare_case_id(),
        task_id: run.task_id.clone(),
        created_at: now_iso(),
        case_type: case_type.to_string(),
        description: format!(
            "overestimate={:.1}x outcome={}",
            run.overestimate_ratio, run.outcome
        ),
        overestimate_ratio: run.overestimate_ratio,
        actual_active_minutes: run.actual_active_minutes,
        original_estimate_minutes: run.original_estimate_minutes,
        outcome: run.outcome.clone(),
    })
}

// ---------------------------------------------------------------------------
// Estimation algorithm
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq)]
pub struct EstimateResult {
    pub human_estimate_minutes: u32,
    pub ai_native_estimate_p50_minutes: u32,
    pub ai_native_estimate_p90_minutes: u32,
    pub expected_speedup_range: String,
    pub matched_records: usize,
    pub confidence: String,
    pub human_anchor_detected: bool,
    pub stop_when_done: bool,
    pub bucket_key: String,
}

pub fn compute_ai_native_estimate(
    root: &Path,
    task_type: &str,
    model: &str,
    workflow: &str,
    human_estimate_minutes: u32,
) -> Result<EstimateResult> {
    let config = read_config(root)?;
    let runs = read_jsonl::<RunRecord>(root, RUNS_JSONL)?;

    let bucket_keys = vec![
        format!("model:{model}|project:aiplus|task:{task_type}|workflow:{workflow}"),
        format!("model:{model}|task:{task_type}|workflow:{workflow}"),
        format!("task:{task_type}|workflow:{workflow}"),
        format!("task:{task_type}"),
        "global".to_string(),
    ];

    let _model_family = model.split('-').next().unwrap_or(model);

    let mut best_bucket: Option<(String, Vec<u32>)> = None;
    for key in &bucket_keys {
        let samples: Vec<u32> = runs
            .iter()
            .filter(|r| {
                if key == "global" {
                    return true;
                }
                let parts: Vec<&str> = key.split('|').collect();
                parts.iter().all(|part| {
                    let kv: Vec<&str> = part.splitn(2, ':').collect();
                    if kv.len() != 2 {
                        return true;
                    }
                    match kv[0] {
                        "model" => r.model == kv[1],
                        "task" => r.task_type == kv[1],
                        "workflow" => r.workflow_level == kv[1],
                        "project" => r.project_id == kv[1] || kv[1] == "aiplus",
                        _ => true,
                    }
                })
            })
            .map(|r| r.actual_active_minutes)
            .collect();

        if samples.len() >= config.min_bucket_samples {
            best_bucket = Some((key.clone(), samples));
            break;
        }

        // Fallback: if no exact match, try model family
        if key.starts_with("model:") && samples.len() >= 3 {
            best_bucket = Some((key.clone(), samples));
            break;
        }
    }

    if let Some((bucket_key, samples)) = best_bucket {
        let mut sorted = samples.clone();
        sorted.sort_unstable();
        let p50 = percentile(&sorted, 0.5);
        let p90 = percentile(&sorted, 0.9);
        let matched = samples.len();

        let confidence = if matched >= 20 {
            "high"
        } else if matched >= config.min_bucket_samples {
            "medium"
        } else {
            "low"
        };

        let human_anchor_detected = human_estimate_minutes >= 2 * p50;

        let expected_speedup_range = if p50 > 0 {
            let low = (human_estimate_minutes as f64 / (p50 * 2) as f64).max(1.0);
            let high = (human_estimate_minutes as f64 / (p50 / 2).max(1) as f64).max(low);
            format!("{low:.0}x-{high:.0}x")
        } else {
            "unknown".to_string()
        };

        return Ok(EstimateResult {
            human_estimate_minutes,
            ai_native_estimate_p50_minutes: p50,
            ai_native_estimate_p90_minutes: p90,
            expected_speedup_range,
            matched_records: matched,
            confidence: confidence.to_string(),
            human_anchor_detected,
            stop_when_done: true,
            bucket_key,
        });
    }

    // No bucket: broad fallback
    let fallback_p50 = (human_estimate_minutes / 4).max(5);
    let fallback_p90 = (human_estimate_minutes / 2).max(10);
    let human_anchor_detected = human_estimate_minutes >= 2 * fallback_p50;

    Ok(EstimateResult {
        human_estimate_minutes,
        ai_native_estimate_p50_minutes: fallback_p50,
        ai_native_estimate_p90_minutes: fallback_p90,
        expected_speedup_range: "2x-8x".to_string(),
        matched_records: 0,
        confidence: "low".to_string(),
        human_anchor_detected,
        stop_when_done: true,
        bucket_key: "fallback".to_string(),
    })
}

fn percentile(sorted: &[u32], p: f64) -> u32 {
    if sorted.is_empty() {
        return 0;
    }
    let idx = ((sorted.len() - 1) as f64 * p).round() as usize;
    sorted[idx.min(sorted.len() - 1)]
}

// ---------------------------------------------------------------------------
// Multiplier / aggregate update
// ---------------------------------------------------------------------------

pub fn update_multipliers(root: &Path) -> Result<()> {
    let runs = read_jsonl::<RunRecord>(root, RUNS_JSONL)?;
    let mut buckets: BTreeMap<String, Vec<u32>> = BTreeMap::new();

    for run in &runs {
        let key = format!(
            "model:{}|project:{}|task:{}|workflow:{}",
            run.model, run.project_id, run.task_type, run.workflow_level
        );
        buckets
            .entry(key)
            .or_default()
            .push(run.actual_active_minutes);
    }

    let mut multipliers = MultiplierBucket {
        schema_version: VELOCITY_SCHEMA_VERSION.to_string(),
        updated_at: now_iso(),
        ..MultiplierBucket::default()
    };

    for (key, samples) in buckets {
        let mut sorted = samples.clone();
        sorted.sort_unstable();
        let p50 = percentile(&sorted, 0.5);
        let p80 = percentile(&sorted, 0.8);

        let overestimate_ratios: Vec<f64> = runs
            .iter()
            .filter(|r| {
                format!(
                    "model:{}|project:{}|task:{}|workflow:{}",
                    r.model, r.project_id, r.task_type, r.workflow_level
                ) == key
            })
            .map(|r| r.overestimate_ratio)
            .collect();
        let mut sorted_ratios = overestimate_ratios.clone();
        sorted_ratios.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
        let ratio_p50 = if !sorted_ratios.is_empty() {
            sorted_ratios[sorted_ratios.len() / 2]
        } else {
            0.0
        };

        let bias_count = runs
            .iter()
            .filter(|r| {
                format!(
                    "model:{}|project:{}|task:{}|workflow:{}",
                    r.model, r.project_id, r.task_type, r.workflow_level
                ) == key
                    && r.human_time_bias
            })
            .count();
        let bias_rate = if !samples.is_empty() {
            bias_count as f64 / samples.len() as f64
        } else {
            0.0
        };

        let model_key = runs
            .iter()
            .find(|r| {
                format!(
                    "model:{}|project:{}|task:{}|workflow:{}",
                    r.model, r.project_id, r.task_type, r.workflow_level
                ) == key
            })
            .map(|r| r.model.clone())
            .unwrap_or_default();
        let model_family = model_key
            .split('-')
            .next()
            .unwrap_or(&model_key)
            .to_string();

        let confidence = if samples.len() >= 20 {
            "high"
        } else if samples.len() >= DEFAULT_MIN_BUCKET_SAMPLES {
            "medium"
        } else {
            "low"
        };

        multipliers.buckets.insert(
            key,
            BucketData {
                model_key,
                model_family,
                sample_count: samples.len(),
                actual_ai_p50_minutes: p50,
                actual_ai_p80_minutes: p80,
                overestimate_ratio_p50: ratio_p50,
                observed_speedup_p50: ratio_p50,
                human_bias_rate: bias_rate,
                confidence: confidence.to_string(),
                stale_for_current_model: false,
                last_updated_at: now_iso(),
                recent_actual_minutes: samples.into_iter().rev().take(5).collect(),
            },
        );
    }

    write_json(root, MULTIPLIERS_JSON, &multipliers)?;
    Ok(())
}

pub fn update_aggregates(root: &Path) -> Result<()> {
    let estimates = read_jsonl::<EstimateRecord>(root, ESTIMATES_JSONL)?;
    let runs = read_jsonl::<RunRecord>(root, RUNS_JSONL)?;
    let signals = read_jsonl::<AnchorSignalRecord>(root, ANCHOR_SIGNALS_JSONL)?;
    let rare_cases = read_jsonl::<RareCaseRecord>(root, RARE_CASES_JSONL)?;

    let mut sorted_ratios: Vec<f64> = runs
        .iter()
        .map(|r| r.overestimate_ratio)
        .filter(|r| r.is_finite() && *r > 0.0)
        .collect();
    sorted_ratios.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    let median_ratio = if !sorted_ratios.is_empty() {
        sorted_ratios[sorted_ratios.len() / 2]
    } else {
        0.0
    };

    let bias_count = runs.iter().filter(|r| r.human_time_bias).count();
    let bias_rate = if !runs.is_empty() {
        bias_count as f64 / runs.len() as f64
    } else {
        0.0
    };

    let agg = Aggregates {
        schema_version: VELOCITY_SCHEMA_VERSION.to_string(),
        updated_at: now_iso(),
        total_estimates: estimates.len(),
        total_runs: runs.len(),
        total_anchor_signals: signals.len(),
        total_rare_cases: rare_cases.len(),
        median_overestimate_ratio: median_ratio,
        human_time_bias_rate: bias_rate,
        last_rotation_at: read_rotation_state(root)?.last_rotation_at,
    };

    write_json(root, AGGREGATES_JSON, &agg)?;
    Ok(())
}

fn read_rotation_state(root: &Path) -> Result<RotationState> {
    let path = velocity_dir(root).join(ROTATION_STATE_JSON);
    if !path.exists() {
        return Ok(RotationState::default());
    }
    let text = fs::read_to_string(&path)?;
    Ok(serde_json::from_str(&text)?)
}

// ---------------------------------------------------------------------------
// Retention
// ---------------------------------------------------------------------------

pub fn apply_retention(root: &Path) -> Result<()> {
    let config = read_config(root)?;

    let estimates: Vec<EstimateRecord> = read_jsonl(root, ESTIMATES_JSONL)?;
    let runs: Vec<RunRecord> = read_jsonl(root, RUNS_JSONL)?;
    let signals: Vec<AnchorSignalRecord> = read_jsonl(root, ANCHOR_SIGNALS_JSONL)?;
    let rare_cases: Vec<RareCaseRecord> = read_jsonl(root, RARE_CASES_JSONL)?;

    let estimates_len = estimates.len();
    let rare_cases_len = rare_cases.len();

    let trimmed_estimates = trim_latest(estimates, config.max_records);
    let trimmed_runs = trim_latest(runs, config.max_records);
    let trimmed_signals = trim_latest(signals, config.max_records);
    let trimmed_rare = trim_latest(rare_cases, config.rare_case_max_records);

    rewrite_jsonl(root, ESTIMATES_JSONL, &trimmed_estimates)?;
    rewrite_jsonl(root, RUNS_JSONL, &trimmed_runs)?;
    rewrite_jsonl(root, ANCHOR_SIGNALS_JSONL, &trimmed_signals)?;
    rewrite_jsonl(root, RARE_CASES_JSONL, &trimmed_rare)?;

    let mut state = read_rotation_state(root)?;
    state.schema_version = VELOCITY_SCHEMA_VERSION.to_string();
    state.last_rotation_at = now_iso();
    state.rotation_runs += 1;
    state.records_pruned += estimates_len - trimmed_estimates.len();
    state.rare_cases_pruned += rare_cases_len - trimmed_rare.len();

    write_json(root, ROTATION_STATE_JSON, &state)?;
    update_multipliers(root)?;
    update_aggregates(root)?;

    Ok(())
}

fn trim_latest<T>(mut records: Vec<T>, limit: usize) -> Vec<T> {
    if records.len() > limit {
        let skip = records.len() - limit;
        records.drain(..skip);
    }
    records
}

// ---------------------------------------------------------------------------
// Redaction / validation
// ---------------------------------------------------------------------------

pub fn reject_sensitive_velocity_text(text: &str) -> Result<()> {
    let findings: Vec<&str> = sensitive_findings(text)
        .into_iter()
        .filter_map(|(label, found)| found.then_some(label))
        .collect();
    if !findings.is_empty() {
        return Err(anyhow!(
            "VELOCITY_REDACTION_STATUS=BLOCKED reason=sensitive_pattern labels=[{}]",
            findings.join(",")
        ));
    }
    Ok(())
}

pub fn validate_run_record(run: &RunRecord) -> Result<()> {
    if run.actual_active_minutes > run.wall_clock_minutes && run.wall_clock_minutes > 0 {
        return Err(anyhow!(
            "VELOCITY_VALIDATION_STATUS=BLOCKED reason=actual_active_minutes > wall_clock_minutes"
        ));
    }
    if !run.overestimate_ratio.is_finite() {
        return Err(anyhow!(
            "VELOCITY_VALIDATION_STATUS=BLOCKED reason=overestimate_ratio is not finite"
        ));
    }
    if run.secret_values_stored {
        return Err(anyhow!(
            "VELOCITY_VALIDATION_STATUS=BLOCKED reason=secret_values_stored=true"
        ));
    }
    if run.raw_content_stored {
        return Err(anyhow!(
            "VELOCITY_VALIDATION_STATUS=BLOCKED reason=raw_content_stored=true"
        ));
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// Doctor
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq)]
pub struct DoctorReport {
    pub status: String,
    pub records_count: usize,
    pub rotation_needed: bool,
    pub bad_jsonl_lines: usize,
    pub secret_values: String,
    pub raw_content_found: String,
    pub global_agent_config_edits: String,
    pub duplicate_ids: usize,
    pub missing_required_fields: usize,
    pub negative_time_records: usize,
    pub actual_exceeds_wallclock: usize,
    pub nan_multipliers: usize,
    pub over_threshold_files: Vec<String>,
    pub sqlite_found: bool,
}

pub fn doctor(root: &Path) -> Result<DoctorReport> {
    let dir = velocity_dir(root);
    let mut report = DoctorReport {
        status: "PASS".to_string(),
        records_count: 0,
        rotation_needed: false,
        bad_jsonl_lines: 0,
        secret_values: "none".to_string(),
        raw_content_found: "no".to_string(),
        global_agent_config_edits: "none".to_string(),
        duplicate_ids: 0,
        missing_required_fields: 0,
        negative_time_records: 0,
        actual_exceeds_wallclock: 0,
        nan_multipliers: 0,
        over_threshold_files: Vec::new(),
        sqlite_found: false,
    };

    if !dir.exists() {
        report.status = "NEEDS_FIX".to_string();
        return Ok(report);
    }

    // Check JSONL files
    for (filename, _) in [
        (ESTIMATES_JSONL, "estimate"),
        (RUNS_JSONL, "run"),
        (ANCHOR_SIGNALS_JSONL, "signal"),
        (RARE_CASES_JSONL, "rare_case"),
    ] {
        let path = dir.join(filename);
        if !path.exists() {
            continue;
        }

        let meta = fs::metadata(&path)?;
        let size = meta.len() as usize;
        if size > DEFAULT_MAX_BYTES_PER_JSONL {
            report.over_threshold_files.push(format!(
                "{} ({} bytes > {})",
                filename, size, DEFAULT_MAX_BYTES_PER_JSONL
            ));
        }

        let text = match fs::read_to_string(&path) {
            Ok(t) => t,
            Err(_) => {
                report.bad_jsonl_lines += 1;
                continue;
            }
        };

        let mut ids: HashSet<String> = HashSet::new();
        for line in text.lines() {
            let line = line.trim();
            if line.is_empty() {
                continue;
            }
            report.records_count += 1;

            // Try parse as generic JSON to validate structure
            if serde_json::from_str::<serde_json::Value>(line).is_err() {
                report.bad_jsonl_lines += 1;
                continue;
            }

            // Specific checks per file type
            if filename == RUNS_JSONL {
                if let Ok(run) = serde_json::from_str::<RunRecord>(line) {
                    if !ids.insert(run.id.clone()) {
                        report.duplicate_ids += 1;
                    }
                    if run.actual_active_minutes == 0
                        && run.wall_clock_minutes == 0
                        && run.original_estimate_minutes == 0
                    {
                        report.missing_required_fields += 1;
                    }
                    if run.actual_active_minutes as i64 > run.wall_clock_minutes as i64 {
                        report.actual_exceeds_wallclock += 1;
                    }
                    if !run.overestimate_ratio.is_finite() {
                        report.nan_multipliers += 1;
                    }
                    if run.secret_values_stored {
                        report.secret_values = "found".to_string();
                    }
                    if run.raw_content_stored {
                        report.raw_content_found = "yes".to_string();
                    }
                }
            } else if filename == ESTIMATES_JSONL {
                if let Ok(est) = serde_json::from_str::<EstimateRecord>(line) {
                    if !ids.insert(est.id.clone()) {
                        report.duplicate_ids += 1;
                    }
                    if est.human_estimate_minutes == 0 && est.ai_native_estimate_p50_minutes == 0 {
                        report.missing_required_fields += 1;
                    }
                }
            }

            // Sensitive pattern scan
            if reject_sensitive_velocity_text(line).is_err() {
                report.secret_values = "found".to_string();
            }
        }
    }

    // Check multipliers
    let multipliers_path = dir.join(MULTIPLIERS_JSON);
    if multipliers_path.exists() {
        if let Ok(text) = fs::read_to_string(&multipliers_path) {
            if let Ok(multipliers) = serde_json::from_str::<MultiplierBucket>(&text) {
                for bucket in multipliers.buckets.values() {
                    if !bucket.overestimate_ratio_p50.is_finite()
                        || !bucket.observed_speedup_p50.is_finite()
                        || !bucket.human_bias_rate.is_finite()
                    {
                        report.nan_multipliers += 1;
                    }
                }
            }
        }
    }

    // Check for SQLite files
    if dir.join("velocity.sqlite").exists() || dir.join("velocity.db").exists() {
        report.sqlite_found = true;
    }

    // Check rotation state
    let state = read_rotation_state(root)?;
    if state.rotation_runs == 0 && report.records_count > DEFAULT_MAX_RECORDS {
        report.rotation_needed = true;
    }

    if report.bad_jsonl_lines > 0
        || report.duplicate_ids > 0
        || report.secret_values == "found"
        || report.raw_content_found == "yes"
        || report.sqlite_found
        || !report.over_threshold_files.is_empty()
    {
        report.status = "NEEDS_FIX".to_string();
    } else if report.rotation_needed {
        report.status = "NEEDS_ROTATION".to_string();
    }

    Ok(report)
}

// ---------------------------------------------------------------------------
// Purge
// ---------------------------------------------------------------------------

pub fn purge_velocity(root: &Path) -> Result<()> {
    let dir = velocity_dir(root);
    if dir.exists() {
        fs::remove_dir_all(&dir)?;
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_duration_valid() {
        assert_eq!(parse_duration("20m").unwrap(), 20);
        assert_eq!(parse_duration("1h").unwrap(), 60);
        assert_eq!(parse_duration("1.5h").unwrap(), 90);
        assert_eq!(parse_duration("5h").unwrap(), 300);
        assert_eq!(parse_duration("90m").unwrap(), 90);
    }

    #[test]
    fn parse_duration_invalid() {
        assert!(parse_duration("abc").is_err());
        assert!(parse_duration("").is_err());
        assert!(parse_duration("1x").is_err());
    }

    #[test]
    fn parse_duration_negative() {
        assert!(parse_duration("-20m").is_err());
        assert!(parse_duration("-1h").is_err());
    }

    #[test]
    fn bias_fixture_5h_to_20m() {
        let estimate = EstimateRecord {
            human_estimate_minutes: 300,
            human_baseline_minutes: 300,
            ai_native_estimate_p50_minutes: 45,
            ai_native_estimate_p90_minutes: 90,
            ..EstimateRecord::default()
        };
        let run = RunRecord {
            actual_active_minutes: 20,
            wall_clock_minutes: 24,
            original_estimate_minutes: 300,
            outcome: "pass".to_string(),
            quality_verdict: "pass".to_string(),
            slow_reason: "none".to_string(),
            ..RunRecord::default()
        };
        let bias = detect_bias(&estimate, &run);
        assert!(bias.human_time_bias_found);
        assert_eq!(bias.overestimate_ratio, 15.0);
        assert_eq!(bias.human_time_bias_confidence, "high");
        assert_eq!(bias.bias_type, "OVERESTIMATE");
    }

    #[test]
    fn bias_missing_baseline() {
        let estimate = EstimateRecord {
            human_estimate_minutes: 300,
            human_baseline_minutes: 0,
            ai_native_estimate_p50_minutes: 45,
            ..EstimateRecord::default()
        };
        let run = RunRecord {
            actual_active_minutes: 20,
            wall_clock_minutes: 24,
            original_estimate_minutes: 300,
            outcome: "pass".to_string(),
            quality_verdict: "pass".to_string(),
            slow_reason: "none".to_string(),
            ..RunRecord::default()
        };
        let bias = detect_bias(&estimate, &run);
        assert!(bias.human_time_bias_found);
        assert_eq!(bias.human_baseline_status, "missing");
        assert_eq!(bias.human_time_bias_confidence, "low");
    }

    #[test]
    fn blocked_not_counted_as_bias() {
        let estimate = EstimateRecord {
            human_estimate_minutes: 300,
            human_baseline_minutes: 300,
            ai_native_estimate_p50_minutes: 45,
            ..EstimateRecord::default()
        };
        let run = RunRecord {
            actual_active_minutes: 20,
            wall_clock_minutes: 24,
            original_estimate_minutes: 300,
            outcome: "blocked".to_string(),
            quality_verdict: "blocked".to_string(),
            slow_reason: "none".to_string(),
            ..RunRecord::default()
        };
        let bias = detect_bias(&estimate, &run);
        assert!(!bias.human_time_bias_found);
    }

    #[test]
    fn retention_trims_normal_records() {
        let temp = tempfile::tempdir().unwrap();
        let root = temp.path();
        init_velocity(root).unwrap();

        let mut records = Vec::new();
        for i in 0..250 {
            records.push(EstimateRecord {
                id: format!("est_{i}"),
                human_estimate_minutes: 10,
                ..EstimateRecord::default()
            });
        }
        rewrite_jsonl(root, ESTIMATES_JSONL, &records).unwrap();
        apply_retention(root).unwrap();

        let kept: Vec<EstimateRecord> = read_jsonl(root, ESTIMATES_JSONL).unwrap();
        assert_eq!(kept.len(), DEFAULT_MAX_RECORDS);
    }

    #[test]
    fn retention_trims_rare_cases() {
        let temp = tempfile::tempdir().unwrap();
        let root = temp.path();
        init_velocity(root).unwrap();

        let mut records = Vec::new();
        for i in 0..30 {
            records.push(RareCaseRecord {
                id: format!("rare_{i}"),
                ..RareCaseRecord::default()
            });
        }
        rewrite_jsonl(root, RARE_CASES_JSONL, &records).unwrap();
        apply_retention(root).unwrap();

        let kept: Vec<RareCaseRecord> = read_jsonl(root, RARE_CASES_JSONL).unwrap();
        assert_eq!(kept.len(), DEFAULT_RARE_CASE_MAX_RECORDS);
    }

    #[test]
    fn doctor_clean_store_pass() {
        let temp = tempfile::tempdir().unwrap();
        let root = temp.path();
        init_velocity(root).unwrap();

        let report = doctor(root).unwrap();
        assert_eq!(report.status, "PASS");
        assert_eq!(report.secret_values, "none");
        assert!(!report.sqlite_found);
    }

    #[test]
    fn doctor_malformed_jsonl_needs_fix() {
        let temp = tempfile::tempdir().unwrap();
        let root = temp.path();
        init_velocity(root).unwrap();

        let path = velocity_dir(root).join(RUNS_JSONL);
        fs::write(&path, b"{not valid json\n").unwrap();

        let report = doctor(root).unwrap();
        assert_eq!(report.status, "NEEDS_FIX");
        assert!(report.bad_jsonl_lines > 0);
    }

    #[test]
    fn doctor_duplicate_ids_needs_fix() {
        let temp = tempfile::tempdir().unwrap();
        let root = temp.path();
        init_velocity(root).unwrap();

        let records = vec![
            RunRecord {
                id: "run_dup".to_string(),
                actual_active_minutes: 10,
                wall_clock_minutes: 15,
                original_estimate_minutes: 20,
                overestimate_ratio: 2.0,
                ..RunRecord::default()
            },
            RunRecord {
                id: "run_dup".to_string(),
                actual_active_minutes: 10,
                wall_clock_minutes: 15,
                original_estimate_minutes: 20,
                overestimate_ratio: 2.0,
                ..RunRecord::default()
            },
        ];
        rewrite_jsonl(root, RUNS_JSONL, &records).unwrap();

        let report = doctor(root).unwrap();
        assert_eq!(report.status, "NEEDS_FIX");
        assert!(report.duplicate_ids > 0);
    }

    #[test]
    fn doctor_secret_pattern_needs_fix() {
        let temp = tempfile::tempdir().unwrap();
        let root = temp.path();
        init_velocity(root).unwrap();

        let records = vec![RunRecord {
            id: "run_1".to_string(),
            actual_active_minutes: 10,
            wall_clock_minutes: 15,
            original_estimate_minutes: 20,
            overestimate_ratio: 2.0,
            secret_values_stored: true,
            ..RunRecord::default()
        }];
        rewrite_jsonl(root, RUNS_JSONL, &records).unwrap();

        let report = doctor(root).unwrap();
        assert_eq!(report.status, "NEEDS_FIX");
        assert_eq!(report.secret_values, "found");
    }

    #[test]
    fn no_sqlite_dependency_for_velocity() {
        // This test is a structural check: velocity module uses only JSONL.
        // The Cargo.toml check is done externally in CI.
        let temp = tempfile::tempdir().unwrap();
        let root = temp.path();
        init_velocity(root).unwrap();
        assert!(!velocity_dir(root).join("velocity.sqlite").exists());
        assert!(!velocity_dir(root).join("velocity.db").exists());
    }

    fn baseline_normal_run() -> RunRecord {
        RunRecord {
            actual_active_minutes: 20,
            wall_clock_minutes: 24,
            original_estimate_minutes: 30,
            overestimate_ratio: 1.5,
            outcome: "pass".to_string(),
            quality_verdict: "pass".to_string(),
            ..RunRecord::default()
        }
    }

    #[test]
    fn velocity_slow_reason_none_is_not_rare() {
        for reason in ["", "none", "None", " NONE "] {
            let run = RunRecord {
                slow_reason: reason.to_string(),
                ..baseline_normal_run()
            };
            assert!(
                !is_rare_case(&run),
                "slow_reason={reason:?} should not mark a normal pass as rare"
            );
            assert!(
                classify_rare_case(&run).is_none(),
                "slow_reason={reason:?} should not classify as rare"
            );
        }
    }

    #[test]
    fn velocity_meaningful_slow_reason_is_rare() {
        for reason in ["test_failure", "owner_gate", "infra_outage"] {
            let run = RunRecord {
                slow_reason: reason.to_string(),
                ..baseline_normal_run()
            };
            assert!(
                is_rare_case(&run),
                "slow_reason={reason:?} should mark run as rare"
            );
            let rare = classify_rare_case(&run).expect("rare classification");
            assert_eq!(rare.case_type, "slow_reason");
        }
    }

    #[test]
    fn velocity_large_overestimate_remains_rare() {
        let run = RunRecord {
            slow_reason: "none".to_string(),
            overestimate_ratio: 15.0,
            ..baseline_normal_run()
        };
        assert!(is_rare_case(&run));
        let rare = classify_rare_case(&run).expect("rare classification");
        assert_eq!(rare.case_type, "large_overestimate");
    }
}
