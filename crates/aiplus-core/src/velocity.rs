use crate::redaction::sensitive_findings;
use anyhow::{anyhow, Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, HashSet};
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};

pub const VELOCITY_SCHEMA_VERSION: &str = "2";
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
/// Spec v2 Q2: global retention is 5× project. Caps `runs.jsonl` and
/// `estimates.jsonl` at 1000, `rare-cases.jsonl` at 100.
/// At ~900 B/record this puts global ledger lifetime under 2 MB.
pub const DEFAULT_GLOBAL_MAX_RECORDS: usize = 1000;
pub const DEFAULT_GLOBAL_RARE_CASE_MAX_RECORDS: usize = 100;
/// Spec v2 Q1: merge rule "project-recent-heavy" — when reading the
/// union of local + global for estimate calibration, take the most
/// recent N from project and most recent M from global, dedupe by id.
pub const MERGE_LOCAL_TAKE: usize = 50;
pub const MERGE_GLOBAL_TAKE: usize = 150;

/// W6: research-native unit types that AEL ships with. The bucket
/// lookup in `compute_ai_native_estimate` keys off `task_type`, so
/// adding these as a documented set is enough to make them
/// queryable. The strings here are the values the CLI expects for
/// `--task-type`.
pub const AEL_VELOCITY_UNIT_TYPES: &[&str] = &[
    "regression-spec",
    "table",
    "figure",
    "replication-package",
    "referee-response",
];

/// W6: synthetic per-unit-type seed distribution. Each tuple is
/// `(task_type, [actual_active_minutes; 5])`. The minute values are
/// rough rules-of-thumb for an AEL-driven agent workflow (model =
/// claude-opus class, workflow = MEDIUM); they get flagged as
/// `seed = true` so doctor knows they are "not calibrated" and the
/// CLI can mark the resulting p50/p90 as advisory.
///
/// The numbers are deliberately conservative — better to over-
/// estimate at the start than to anchor users on optimistic seeds.
/// Once the user logs 5+ real runs per type, those displace the
/// seeds in the bucket lookup (real runs sort by recency).
pub const AEL_VELOCITY_SEED_RUNS: &[(&str, [u32; 5])] = &[
    // One specification = one estimator + clustering choice + the
    // robustness column. Coffee-and-think 12 minutes; AI-assisted
    // typical 18-30 min.
    ("regression-spec", [18, 22, 25, 28, 32]),
    // One paper table = N specifications wired together, formatted
    // for LaTeX, captioned. Typical 45-90 min including a Replicator
    // pass.
    ("table", [45, 55, 65, 75, 90]),
    // One paper figure = one econometric output + ggplot/matplotlib
    // styling + caption. Typical 35-75 min.
    ("figure", [35, 45, 55, 65, 75]),
    // One AEA-Data-Editor-grade replication package: Makefile,
    // env pin, README, clean-machine verification pass. Multi-day.
    ("replication-package", [240, 320, 480, 600, 960]),
    // One R&R response point: read the referee comment, write the
    // pointed rebuttal, link to the supporting analysis, route to
    // co-authors. Typical 25-70 min.
    ("referee-response", [25, 35, 45, 55, 70]),
];
pub const DEFAULT_MAX_BYTES_PER_JSONL: usize = 1_048_576;
pub const DEFAULT_RETAIN_DAYS: u64 = 90;
pub const DEFAULT_MIN_BUCKET_SAMPLES: usize = 8;

// ---------------------------------------------------------------------------
// Config
// ---------------------------------------------------------------------------

/// Spec v2 Q5: three-state explicit opt-out semantics. Default
/// `ReadWrite`. `None` is full isolation; `ReadOnly` is "learn from
/// others but don't share." A `bool` flag would conflate the two and
/// is rejected by spec.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ShareToGlobalMode {
    /// Read from + write to the global ledger. Default for new projects.
    ReadWrite,
    /// Read from the global ledger; do not contribute writes.
    ReadOnly,
    /// Full isolation. Project does not read or write global ledger.
    None,
}

impl Default for ShareToGlobalMode {
    fn default() -> Self {
        Self::ReadWrite
    }
}

impl ShareToGlobalMode {
    pub fn reads_global(self) -> bool {
        matches!(self, Self::ReadWrite | Self::ReadOnly)
    }
    pub fn writes_global(self) -> bool {
        matches!(self, Self::ReadWrite)
    }
    pub fn as_str(self) -> &'static str {
        match self {
            Self::ReadWrite => "read_write",
            Self::ReadOnly => "read_only",
            Self::None => "none",
        }
    }
}

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
    /// Spec v2 Q5: per-project opt-out for the global ledger. Default
    /// `ReadWrite`. Skipped on serialize when default so old configs
    /// stay byte-identical until the user explicitly opts out.
    #[serde(default, skip_serializing_if = "is_share_default")]
    pub share_to_global_mode: ShareToGlobalMode,
}

fn is_share_default(mode: &ShareToGlobalMode) -> bool {
    *mode == ShareToGlobalMode::ReadWrite
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
            share_to_global_mode: ShareToGlobalMode::ReadWrite,
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
    /// W6: true when this is a synthetic seed run installed by an
    /// `*_init` hook (e.g., aieconlab_init) so brand-new projects
    /// have *some* p50/p90 distribution before the first real
    /// measurement. Seed runs participate in the bucket lookup, but
    /// `doctor` counts them separately and reports
    /// "estimates not yet calibrated" when the bucket is < 5
    /// non-seed records.
    #[serde(default, skip_serializing_if = "is_default_bool")]
    pub seed: bool,
}

fn is_default_bool(b: &bool) -> bool {
    !*b
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
            seed: false,
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

/// W6: seed the velocity store with synthetic AEL unit-type runs.
/// Called from `aieconlab_init` on the AiPlus CLI side. Each seed is
/// flagged `seed=true` so doctor knows it does not count toward the
/// 5-record calibration threshold, and `actual_time_source` is set
/// to `"synthetic-seed-w6"` so a Replicator-style audit can identify
/// them later.
///
/// Idempotent: re-running `aieconlab_init` will not append duplicate
/// seeds (we check for existing `seed=true` rows per task_type before
/// writing).
pub fn init_aieconlab_velocity_seeds(root: &Path) -> Result<()> {
    init_velocity(root)?;
    // Lower the bucket-sample threshold to 5 so the per-task-type
    // seed cohort (5 rows) actually fires when the user queries
    // `velocity estimate --task-type table` etc. The default of 8
    // would otherwise force fallback to the global bucket and
    // average across all 5 unit types — which gives meaningless
    // numbers for AEL.
    let mut config = read_config(root).unwrap_or_default();
    if config.min_bucket_samples > 5 {
        config.min_bucket_samples = 5;
        write_config(root, &config)?;
    }
    let existing = read_jsonl::<RunRecord>(root, RUNS_JSONL).unwrap_or_default();
    let already_seeded: std::collections::BTreeSet<String> = existing
        .iter()
        .filter(|r| r.seed)
        .map(|r| r.task_type.clone())
        .collect();
    let now = now_iso();
    for (task_type, samples) in AEL_VELOCITY_SEED_RUNS {
        if already_seeded.contains(*task_type) {
            continue;
        }
        for (i, minutes) in samples.iter().enumerate() {
            let run = RunRecord {
                schema_version: VELOCITY_SCHEMA_VERSION.to_string(),
                id: format!("seed-w6-{task_type}-{i}"),
                estimate_id: String::new(),
                task_id: format!("seed-w6-{task_type}-{i}"),
                created_at: now.clone(),
                project_id: "aieconlab-seed".to_string(),
                task_type: (*task_type).to_string(),
                repo_area: String::new(),
                agent_role: "ra-stata".to_string(),
                runtime: "synthetic".to_string(),
                model: "synthetic-seed".to_string(),
                workflow_level: "MEDIUM".to_string(),
                original_estimate_minutes: *minutes,
                human_baseline_minutes: minutes * 3,
                actual_active_minutes: *minutes,
                actual_time_source: "synthetic-seed-w6".to_string(),
                wall_clock_minutes: *minutes,
                tool_wait_minutes: 0,
                blocked_minutes: 0,
                outcome: "pass".to_string(),
                verification_depth: "standard".to_string(),
                quality_verdict: "pass".to_string(),
                rework_count: 0,
                owner_gate_hit: false,
                overestimate_ratio: 1.0,
                human_time_bias: false,
                slow_reason: String::new(),
                redaction_status: "clean".to_string(),
                raw_content_stored: false,
                secret_values_stored: false,
                memory_integration: "disabled".to_string(),
                seed: true,
            };
            let line = serde_json::to_string(&run)?;
            let path = velocity_dir(root).join(RUNS_JSONL);
            crate::append_jsonl_atomic(&path, &line)?;
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
//
// Spec v2 Q4 commits to ULID-shaped IDs (Crockford base32, sortable,
// 80 random bits) so a future multi-machine sync of `~/.config/aiplus/
// velocity/` cannot produce duplicates from concurrent writers on
// different machines. Old `est_{unix_ms}` IDs are still read fine —
// they're just strings — so this is forward-compat without breaking
// existing ledgers.

/// Crockford base32 alphabet, ULID-spec compatible.
const ULID_ALPHABET: &[u8; 32] = b"0123456789ABCDEFGHJKMNPQRSTVWXYZ";

/// Generate a 26-char ULID. First 10 chars = 48-bit UNIX-ms timestamp,
/// last 16 chars = 80 bits of randomness. Hand-rolled to avoid adding
/// a third-party `ulid` dep per spec ("no new third-party deps").
fn generate_ulid() -> String {
    let mut buf = [0u8; 26];
    let ts_ms = crate::epoch_millis();
    // Encode 48-bit timestamp into the first 10 chars (5 bits per char).
    for i in 0..10 {
        let shift = 5 * (9 - i);
        let idx = ((ts_ms >> shift) & 0x1F) as usize;
        buf[i] = ULID_ALPHABET[idx];
    }
    // 16 random chars (80 bits). Pull from a SystemRandom-style mix
    // of nanos + thread id + a per-call counter so concurrent calls
    // on the same machine never share entropy bits.
    let mut rnd = ulid_entropy();
    for i in 10..26 {
        let idx = (rnd & 0x1F) as usize;
        buf[i] = ULID_ALPHABET[idx];
        rnd = ulid_mix(rnd);
    }
    String::from_utf8(buf.to_vec()).expect("ULID alphabet is ASCII")
}

fn ulid_entropy() -> u64 {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};
    use std::sync::atomic::{AtomicU64, Ordering};
    static COUNTER: AtomicU64 = AtomicU64::new(0);
    let mut hasher = DefaultHasher::new();
    let now = std::time::SystemTime::now();
    let nanos = now
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.subsec_nanos() as u64 ^ d.as_secs())
        .unwrap_or(0);
    nanos.hash(&mut hasher);
    std::thread::current().id().hash(&mut hasher);
    COUNTER.fetch_add(1, Ordering::Relaxed).hash(&mut hasher);
    std::process::id().hash(&mut hasher);
    hasher.finish()
}

fn ulid_mix(x: u64) -> u64 {
    // xorshift64* — keeps successive 5-bit windows decorrelated.
    let mut y = x.wrapping_add(0x9E3779B97F4A7C15);
    y ^= y >> 30;
    y = y.wrapping_mul(0xBF58476D1CE4E5B9);
    y ^= y >> 27;
    y = y.wrapping_mul(0x94D049BB133111EB);
    y ^= y >> 31;
    y
}

pub fn generate_estimate_id() -> String {
    format!("est_{}", generate_ulid())
}

pub fn generate_run_id() -> String {
    format!("run_{}", generate_ulid())
}

pub fn generate_signal_id() -> String {
    format!("sig_{}", generate_ulid())
}

pub fn generate_rare_case_id() -> String {
    format!("rare_{}", generate_ulid())
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
    // Spec v2: read the merged ledger (project + global) unless the
    // project's config says no. `read_write` and `read_only` both
    // pull from global; only `none` is local-only.
    let runs: Vec<RunRecord> = if config.share_to_global_mode.reads_global() {
        merge_runs_for_estimate(root, true)
            .unwrap_or_else(|_| read_jsonl::<RunRecord>(root, RUNS_JSONL).unwrap_or_default())
    } else {
        read_jsonl::<RunRecord>(root, RUNS_JSONL)?
    };

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
    /// W6: task_types whose bucket has fewer than 5 non-seed records.
    /// `aiplus velocity report` and the CLI doctor surface this so a
    /// user knows "your p50/p90 for table tasks is still on synthetic
    /// seeds, treat the estimate as advisory."
    pub uncalibrated_buckets: Vec<String>,
    /// Spec v2 §9: global-ledger telemetry. Counts and the project's
    /// effective share mode let the doctor explain the merged state
    /// without ever reading sensitive fields.
    pub local_records_count: usize,
    pub global_records_count: usize,
    pub synced_records_count: usize,
    pub local_only_records_count: usize,
    pub share_to_global_mode: String,
    pub global_ledger_health: String,
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
        uncalibrated_buckets: Vec::new(),
        local_records_count: 0,
        global_records_count: 0,
        synced_records_count: 0,
        local_only_records_count: 0,
        share_to_global_mode: ShareToGlobalMode::ReadWrite.as_str().to_string(),
        global_ledger_health: "PASS".to_string(),
    };

    if !dir.exists() {
        report.status = "NEEDS_FIX".to_string();
        return Ok(report);
    }
    if let Ok(cfg) = read_config(root) {
        report.share_to_global_mode = cfg.share_to_global_mode.as_str().to_string();
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

    // W6: count non-seed runs per AEL task_type. A bucket with fewer
    // than 5 calibrated (non-seed) records means the user's `velocity
    // estimate` for that type is still riding on synthetic seeds.
    // Surface the names so the CLI can tell the user where to spend
    // their next "complete a real task" action.
    {
        let mut counts: std::collections::BTreeMap<&'static str, usize> =
            AEL_VELOCITY_UNIT_TYPES.iter().map(|t| (*t, 0)).collect();
        if let Ok(runs) = read_jsonl::<RunRecord>(root, RUNS_JSONL) {
            for run in &runs {
                if run.seed {
                    continue;
                }
                if let Some(slot) = counts.get_mut(run.task_type.as_str()) {
                    *slot += 1;
                }
            }
        }
        for (t, n) in &counts {
            if *n < 5 {
                report.uncalibrated_buckets.push(format!("{t}={n}"));
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

    // --- Spec v2 §9: global ledger telemetry ---
    let local_runs: Vec<RunRecord> = read_jsonl(root, RUNS_JSONL).unwrap_or_default();
    let global_runs: Vec<RunRecord> = read_global_jsonl(RUNS_JSONL).unwrap_or_default();
    report.local_records_count = local_runs.len();
    report.global_records_count = global_runs.len();
    let global_ids: std::collections::HashSet<&str> =
        global_runs.iter().map(|r| r.id.as_str()).collect();
    let synced = local_runs
        .iter()
        .filter(|r| !r.id.is_empty() && global_ids.contains(r.id.as_str()))
        .count();
    report.synced_records_count = synced;
    report.local_only_records_count = report.local_records_count.saturating_sub(synced);

    // Three-tier health classifier per spec §9.
    report.global_ledger_health = classify_global_ledger_health();

    Ok(report)
}

/// Spec §9: classify global ledger health into PASS / NEEDS_FIX / FAIL.
/// FAIL = directory unreadable or known-corrupt; NEEDS_FIX = missing
/// file or unparseable line(s); PASS = directory exists, all 8 expected
/// files present, every JSONL line parses, no duplicate ids.
fn classify_global_ledger_health() -> String {
    let dir = match global_velocity_dir() {
        Ok(d) => d,
        Err(_) => return "FAIL".to_string(),
    };
    if !dir.exists() {
        return "PASS".to_string(); // not initialised yet ≠ broken
    }
    let expected_files = [
        CONFIG_JSON,
        ESTIMATES_JSONL,
        RUNS_JSONL,
        ANCHOR_SIGNALS_JSONL,
        RARE_CASES_JSONL,
    ];
    let mut needs_fix = false;
    for f in expected_files {
        let p = dir.join(f);
        if !p.exists() {
            needs_fix = true;
            continue;
        }
        let Ok(text) = fs::read_to_string(&p) else {
            return "FAIL".to_string();
        };
        if f.ends_with(".jsonl") {
            for line in text.lines() {
                let line = line.trim();
                if line.is_empty() {
                    continue;
                }
                if serde_json::from_str::<serde_json::Value>(line).is_err() {
                    needs_fix = true;
                }
            }
        } else if !text.is_empty() {
            if serde_json::from_str::<serde_json::Value>(&text).is_err() {
                needs_fix = true;
            }
        }
    }
    // iCloud / Dropbox warning: flock is unreliable on sync mounts.
    // Surface as NEEDS_FIX so the user knows to move ~/.config out
    // of a sync folder. Detection is conservative — only catches
    // canonical Apple iCloud and Dropbox paths after symlink resolution.
    if let Ok(canon) = dir.canonicalize() {
        let s = canon.to_string_lossy();
        if s.contains("/Mobile Documents/")
            || s.contains("/Dropbox/")
            || s.contains("/CloudStorage/")
        {
            needs_fix = true;
        }
    }
    if needs_fix {
        "NEEDS_FIX".to_string()
    } else {
        "PASS".to_string()
    }
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
// Global ledger (spec v2)
// ---------------------------------------------------------------------------
//
// Read/write helpers parallel to the project-local ones above, but
// keyed on `~/.config/aiplus/velocity/` instead of a project root.
// Concurrency strategy per spec §11:
//   * JSONL append uses O_APPEND (already what append_jsonl_atomic does)
//     and asserts each record < 4096 bytes (PIPE_BUF) in tests so the
//     append is atomic across concurrent writers without an explicit
//     lock.
//   * Whole-document JSON (`config.json`, `aggregates.json`, …) uses
//     write_file_atomic which is rename(2)-based; rename is atomic on
//     the same filesystem.
//   * Dedup is enforced at write time per spec Q3: if `id` already
//     exists in the file, we skip the write and emit a warning to
//     stderr. Idempotent migrations + no double-counting from
//     concurrent writers.
//   * Permissions: dir 0700, files 0600 (spec §12).

pub use crate::paths::global_velocity_dir;

fn ensure_dir_mode(path: &Path, mode: u32) -> Result<()> {
    fs::create_dir_all(path).with_context(|| format!("mkdir {}", path.display()))?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let perms = std::fs::Permissions::from_mode(mode);
        std::fs::set_permissions(path, perms)
            .with_context(|| format!("chmod {} {:o}", path.display(), mode))?;
    }
    #[cfg(not(unix))]
    {
        let _ = mode;
    }
    Ok(())
}

fn ensure_file_mode(path: &Path, mode: u32) -> Result<()> {
    if !path.exists() {
        return Ok(());
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let perms = std::fs::Permissions::from_mode(mode);
        std::fs::set_permissions(path, perms)
            .with_context(|| format!("chmod {} {:o}", path.display(), mode))?;
    }
    #[cfg(not(unix))]
    {
        let _ = mode;
    }
    Ok(())
}

/// Initialise `~/.config/aiplus/velocity/` if it does not exist.
/// Creates the directory at 0700, writes default config at 0600,
/// touches the four JSONL files (empty) at 0600 so first append
/// inherits 0600. Idempotent — re-running is a no-op when state is
/// already consistent.
pub fn init_global_velocity() -> Result<()> {
    let dir = global_velocity_dir()?;
    ensure_dir_mode(&dir, 0o700)?;

    let config_path = dir.join(CONFIG_JSON);
    if !config_path.exists() {
        // Global config has bigger retention than project config. The
        // share_to_global_mode field is meaningless at the global tier
        // (it's a per-project knob) and is intentionally absent here.
        let config = VelocityConfig {
            schema_version: VELOCITY_SCHEMA_VERSION.to_string(),
            max_records: DEFAULT_GLOBAL_MAX_RECORDS,
            rare_case_max_records: DEFAULT_GLOBAL_RARE_CASE_MAX_RECORDS,
            max_bytes_per_jsonl: DEFAULT_MAX_BYTES_PER_JSONL,
            retain_days: DEFAULT_RETAIN_DAYS,
            min_bucket_samples: DEFAULT_MIN_BUCKET_SAMPLES,
            raw_content_allowed: false,
            memory_integration: "disabled".to_string(),
            share_to_global_mode: ShareToGlobalMode::ReadWrite,
        };
        let json = serde_json::to_string_pretty(&config)?;
        // Race-tolerant write: two concurrent initialisers must not
        // share a tmp filename (`write_file_atomic` derives it from
        // epoch_millis, so two writers in the same ms collide and one
        // gets ENOENT on rename). Use a unique per-process+thread
        // tmp path here and create_new semantics on the rename target.
        let tmp = dir.join(format!(
            "{}.tmp-init-{}-{:?}",
            CONFIG_JSON,
            std::process::id(),
            std::thread::current().id()
        ));
        fs::write(&tmp, json.as_bytes()).with_context(|| format!("write {}", tmp.display()))?;
        match fs::rename(&tmp, &config_path) {
            Ok(()) => {}
            Err(e) => {
                // Another writer beat us to it; clean up our tmp and
                // accept their file (idempotent — both writes are
                // structurally identical).
                let _ = fs::remove_file(&tmp);
                if !config_path.exists() {
                    return Err(e).with_context(|| {
                        format!("rename {} -> {}", tmp.display(), config_path.display())
                    });
                }
            }
        }
    }
    ensure_file_mode(&config_path, 0o600)?;

    for f in [
        ESTIMATES_JSONL,
        RUNS_JSONL,
        ANCHOR_SIGNALS_JSONL,
        RARE_CASES_JSONL,
    ] {
        let p = dir.join(f);
        // Use O_CREAT|O_EXCL via `create_new` so two concurrent
        // initialisers don't both `fs::write(&p, b"")` and race to
        // truncate each other's just-appended record. If the file
        // already exists, leave it untouched.
        match fs::OpenOptions::new().write(true).create_new(true).open(&p) {
            Ok(_) => {}
            Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => {}
            Err(e) => return Err(e).with_context(|| format!("create {}", p.display())),
        }
        ensure_file_mode(&p, 0o600)?;
    }
    Ok(())
}

/// Append a record to a JSONL file under the global ledger, deduping
/// by `id`. Returns `Ok(true)` if appended, `Ok(false)` if the id was
/// already present (a stderr warning is also emitted in the latter
/// case). Spec v2 Q3.
fn append_global_dedup<T: Serialize>(filename: &str, id: &str, record: &T) -> Result<bool> {
    init_global_velocity()?;
    let dir = global_velocity_dir()?;
    let path = dir.join(filename);

    if path.exists() {
        // Cheap dedup: scan existing ids. 1000 records × ~50 bytes for
        // the id substring is ≤50 KB to read; faster than maintaining a
        // separate index file.
        let text = fs::read_to_string(&path).unwrap_or_default();
        let id_token = format!("\"id\":\"{id}\"");
        if text.contains(&id_token) {
            eprintln!(
                "VELOCITY_GLOBAL_APPEND_SKIPPED id={id} reason=duplicate file={}",
                filename
            );
            return Ok(false);
        }
    }
    let mut line = serde_json::to_string(record)?;
    line.push('\n');
    if line.len() >= 4096 {
        // Spec §11: each JSONL record must be < PIPE_BUF (4096 bytes
        // on macOS/Linux) so O_APPEND is atomic across concurrent
        // writers. A record bigger than that breaks the concurrency
        // contract — fail loud rather than silently truncate.
        return Err(anyhow!(
            "VELOCITY_GLOBAL_APPEND_OVERSIZE id={id} bytes={} limit=4096",
            line.len()
        ));
    }
    // POSIX `O_APPEND` writes < PIPE_BUF are atomic across concurrent
    // writers on local filesystems. No lock needed — the kernel does
    // the serialization. We do NOT use `append_jsonl_atomic` here
    // because its read-modify-write+lock model becomes the bottleneck
    // under 8-process contention (lock-acquire timeouts → lost
    // writes in best-effort dual-write).
    let mut f = fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&path)
        .with_context(|| format!("open global ledger for append: {}", path.display()))?;
    f.write_all(line.as_bytes())
        .with_context(|| format!("append to global ledger: {}", path.display()))?;
    drop(f);
    ensure_file_mode(&path, 0o600)?;
    Ok(true)
}

/// Read a JSONL file from the global ledger. Missing file → empty
/// vec (the doctor is what surfaces a never-initialised ledger).
pub fn read_global_jsonl<T: for<'de> Deserialize<'de>>(filename: &str) -> Result<Vec<T>> {
    let dir = match global_velocity_dir() {
        Ok(d) => d,
        Err(_) => return Ok(Vec::new()),
    };
    let path = dir.join(filename);
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

pub fn read_global_config() -> Result<VelocityConfig> {
    let path = global_velocity_dir()?.join(CONFIG_JSON);
    if !path.exists() {
        return Ok(VelocityConfig::default());
    }
    let text = fs::read_to_string(&path).with_context(|| format!("read {}", path.display()))?;
    let config: VelocityConfig =
        serde_json::from_str(&text).with_context(|| format!("parse {}", path.display()))?;
    Ok(config)
}

// --- Privacy projections (spec v2 §7) ---
//
// The global ledger MUST NOT carry free-text task strings, project
// names, paths, user-identity fields, or anything cwd-derived.
// Implementation is *structural*: we serialize a hand-built JSON
// Value that contains only the safe fields. Rationale (per spec):
// "structural privacy is strictly stronger than cryptographic privacy"
// — there's nothing to leak even under rainbow-table attack because
// the sensitive field is not present in the record.
//
// On read-back, the existing serde-with-default deserialization
// repopulates the missing fields with empty strings / zero numbers,
// which is exactly what `compute_ai_native_estimate` already tolerates.

fn run_record_to_global_json(r: &RunRecord) -> serde_json::Value {
    serde_json::json!({
        "schemaVersion": r.schema_version,
        "id": r.id,
        "createdAt": r.created_at,
        "taskType": r.task_type,
        "model": r.model,
        "workflowLevel": r.workflow_level,
        "originalEstimateMinutes": r.original_estimate_minutes,
        "humanBaselineMinutes": r.human_baseline_minutes,
        "actualActiveMinutes": r.actual_active_minutes,
        "wallClockMinutes": r.wall_clock_minutes,
        "toolWaitMinutes": r.tool_wait_minutes,
        "blockedMinutes": r.blocked_minutes,
        "outcome": r.outcome,
        "verificationDepth": r.verification_depth,
        "qualityVerdict": r.quality_verdict,
        "reworkCount": r.rework_count,
        "ownerGateHit": r.owner_gate_hit,
        "overestimateRatio": r.overestimate_ratio,
        "humanTimeBias": r.human_time_bias,
        "slowReason": r.slow_reason,
        "seed": r.seed,
    })
}

fn estimate_record_to_global_json(e: &EstimateRecord) -> serde_json::Value {
    serde_json::json!({
        "schemaVersion": e.schema_version,
        "id": e.id,
        "createdAt": e.created_at,
        "taskType": e.task_type,
        "model": e.model,
        "workflowLevel": e.workflow_level,
        "humanBaselineMinutes": e.human_baseline_minutes,
        "humanEstimateMinutes": e.human_estimate_minutes,
        "aiNativeEstimateP50Minutes": e.ai_native_estimate_p50_minutes,
        "aiNativeEstimateP90Minutes": e.ai_native_estimate_p90_minutes,
        "confidence": e.confidence,
        "matchedRecords": e.matched_records,
    })
}

fn anchor_signal_to_global_json(s: &AnchorSignalRecord) -> serde_json::Value {
    serde_json::json!({
        "schemaVersion": s.schema_version,
        "id": s.id,
        "createdAt": s.created_at,
        "signalType": s.signal_type,
        "description": s.description,
        "humanEstimateMinutes": s.human_estimate_minutes,
        "aiNativePriorMinutes": s.ai_native_prior_minutes,
        "confidence": s.confidence,
    })
}

fn rare_case_to_global_json(r: &RareCaseRecord) -> serde_json::Value {
    serde_json::json!({
        "schemaVersion": r.schema_version,
        "id": r.id,
        "createdAt": r.created_at,
        "caseType": r.case_type,
        "overestimateRatio": r.overestimate_ratio,
        "actualActiveMinutes": r.actual_active_minutes,
        "originalEstimateMinutes": r.original_estimate_minutes,
        "outcome": r.outcome,
    })
}

pub fn append_run_to_global(r: &RunRecord) -> Result<bool> {
    let projected = run_record_to_global_json(r);
    append_global_dedup(RUNS_JSONL, &r.id, &projected)
}

pub fn append_estimate_to_global(e: &EstimateRecord) -> Result<bool> {
    let projected = estimate_record_to_global_json(e);
    append_global_dedup(ESTIMATES_JSONL, &e.id, &projected)
}

pub fn append_anchor_signal_to_global(s: &AnchorSignalRecord) -> Result<bool> {
    let projected = anchor_signal_to_global_json(s);
    append_global_dedup(ANCHOR_SIGNALS_JSONL, &s.id, &projected)
}

pub fn append_rare_case_to_global(r: &RareCaseRecord) -> Result<bool> {
    let projected = rare_case_to_global_json(r);
    append_global_dedup(RARE_CASES_JSONL, &r.id, &projected)
}

/// Spec §6: one-shot migration. Reads the project's velocity JSONLs,
/// projects each record to the global-safe shape (dropping the `task`
/// field and other cwd-derived fields), appends to global with
/// dedup. Returns counts so the CLI can print a meaningful summary.
pub struct ImportStats {
    pub runs_imported: usize,
    pub runs_skipped_duplicate: usize,
    pub estimates_imported: usize,
    pub estimates_skipped_duplicate: usize,
    pub anchor_signals_imported: usize,
    pub anchor_signals_skipped_duplicate: usize,
    pub rare_cases_imported: usize,
    pub rare_cases_skipped_duplicate: usize,
}

pub fn import_from_project(project_root: &Path) -> Result<ImportStats> {
    init_global_velocity()?;
    let mut stats = ImportStats {
        runs_imported: 0,
        runs_skipped_duplicate: 0,
        estimates_imported: 0,
        estimates_skipped_duplicate: 0,
        anchor_signals_imported: 0,
        anchor_signals_skipped_duplicate: 0,
        rare_cases_imported: 0,
        rare_cases_skipped_duplicate: 0,
    };
    for run in read_jsonl::<RunRecord>(project_root, RUNS_JSONL).unwrap_or_default() {
        if append_run_to_global(&run)? {
            stats.runs_imported += 1;
        } else {
            stats.runs_skipped_duplicate += 1;
        }
    }
    for est in read_jsonl::<EstimateRecord>(project_root, ESTIMATES_JSONL).unwrap_or_default() {
        if append_estimate_to_global(&est)? {
            stats.estimates_imported += 1;
        } else {
            stats.estimates_skipped_duplicate += 1;
        }
    }
    for sig in
        read_jsonl::<AnchorSignalRecord>(project_root, ANCHOR_SIGNALS_JSONL).unwrap_or_default()
    {
        if append_anchor_signal_to_global(&sig)? {
            stats.anchor_signals_imported += 1;
        } else {
            stats.anchor_signals_skipped_duplicate += 1;
        }
    }
    for rare in read_jsonl::<RareCaseRecord>(project_root, RARE_CASES_JSONL).unwrap_or_default() {
        if append_rare_case_to_global(&rare)? {
            stats.rare_cases_imported += 1;
        } else {
            stats.rare_cases_skipped_duplicate += 1;
        }
    }
    Ok(stats)
}

/// Spec v2 Q1 merge rule: union of latest N from project + latest M
/// from global, deduped by id, sorted by `created_at` descending.
/// `take_local` = 50, `take_global` = 150 per `MERGE_LOCAL_TAKE` /
/// `MERGE_GLOBAL_TAKE`.
pub fn merge_runs_for_estimate(
    project_root: &Path,
    include_global: bool,
) -> Result<Vec<RunRecord>> {
    let mut local: Vec<RunRecord> = read_jsonl(project_root, RUNS_JSONL).unwrap_or_default();
    local.sort_by(|a, b| b.created_at.cmp(&a.created_at));
    local.truncate(MERGE_LOCAL_TAKE);

    if !include_global {
        return Ok(local);
    }

    let mut global: Vec<RunRecord> = read_global_jsonl(RUNS_JSONL).unwrap_or_default();
    global.sort_by(|a, b| b.created_at.cmp(&a.created_at));
    global.truncate(MERGE_GLOBAL_TAKE);

    let mut seen: std::collections::HashSet<String> = local.iter().map(|r| r.id.clone()).collect();
    let mut combined = local;
    for g in global {
        if !g.id.is_empty() && seen.insert(g.id.clone()) {
            combined.push(g);
        }
    }
    combined.sort_by(|a, b| b.created_at.cmp(&a.created_at));
    Ok(combined)
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
