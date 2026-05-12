use serde::{Deserialize, Serialize};

/// Acceptance mode per deliverable.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AcceptanceMode {
    /// Authoritative. Checks reduce to deterministic verdict.
    /// v0.1 ONLY supports this mode.
    Deterministic,
    /// Always routes to Owner; Auditor skips.
    #[serde(alias = "llm-judge")]
    LlmJudge,
    /// Owner review replaces Auditor entirely.
    OwnerReview,
}

/// Tier classification for trigger policy and canary sampling.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Tier {
    #[serde(rename = "LIGHT")]
    Light,
    #[serde(rename = "MEDIUM")]
    Medium,
    #[serde(rename = "HEAVY")]
    Heavy,
    /// Stop-gate tier for blocking operations.
    StopGate,
}

/// Individual check kind inside a deliverable.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CheckKind {
    /// Command exit code must match expected_exit.
    ExitCode,
    /// File or directory must exist.
    FileExists,
    /// Command stdout must match expected_regex.
    ShellOutputMatch,
}

/// Final verdict emitted by the Auditor for a single deliverable.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "UPPERCASE")]
pub enum AuditorVerdict {
    /// All checks passed (or combiner satisfied).
    Pass,
    /// One or more checks failed.
    Fail,
    /// Pre-audit gate blocked execution.
    Blocked,
    /// Deliverable needs fixes before acceptance.
    NeedsFix,
}

/// Reasons a deliverable may be BLOCKED.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BlockedReason {
    /// Schema file hash mismatch or parse failure.
    SchemaTampered,
    /// Auditor internal error or pass disagreement.
    AuditorUnavailable,
    /// CEO (or any agent) attempted to bypass a gate.
    CeoOverrideAttempted,
    /// Release manifest not GPG-signed or wrong key fingerprint.
    ManifestUnsignedOrWrongKey,
    /// Release manifest has uncommitted changes.
    ManifestDirty,
    /// Attempted to use LLM for hash comparison (forbidden).
    HashViaLlm,
    /// Sentinel file absent or malformed during first-run setup.
    OwnershipUnverified,
    /// Sentinel path found in git history (should be gitignored).
    SentinelInGit,
}

/// Bin alias entry for cross-platform commands.
/// Schema §bin_aliases
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BinAlias {
    pub linux: String,
    pub macos: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fallback_macos: Option<String>,
}

/// Binary verdict for hash comparisons — never exposes raw hash strings to LLM.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum HashVerdict {
    HashMatch,
    HashMismatch,
}

/// Sentinel file contents (YAML).
/// Schema §release_manifest.first_run_setup.ownership_verification
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OwnerSentinel {
    pub name: String,
    pub email: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub note: Option<String>,
}

/// Release manifest schema.
/// Schema §release_manifest.schema
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReleaseManifest {
    pub schema_version: String,
    pub released_at: String,
    pub released_by: String,
    pub auditor_min_version: String,
    pub acceptance_files: Vec<String>,
    pub audit_scripts: Vec<String>,
    pub audit_script_self_tests: Vec<String>,
    pub synthetic_fixtures: Vec<String>,
    pub bin_aliases: Vec<String>,
    pub bin_aliases_hash: String,
    pub acceptance_schema_hash: String,
    pub audit_scripts_hash: String,
    pub audit_script_self_tests_hash: String,
    pub synthetic_fixtures_hash: String,
}

/// Role identifier.
pub type RoleId = String;

/// Audit subcommand arguments.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AuditArgs {
    pub subcommand: AuditSub,
}

/// Audit subcommands.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum AuditSub {
    Run {
        deliverable: Option<String>,
        mode: String,
    },
    Canary,
    Replay {
        run_id: String,
    },
    SetupGpg,
}

/// A single check inside a deliverable.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Check {
    pub id: String,
    pub kind: CheckKind,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cmd: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub expected_exit: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub path: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub expected_regex: Option<String>,
    #[serde(default = "default_check_timeout")]
    pub timeout_seconds: u64,
}

fn default_check_timeout() -> u64 {
    30
}

/// How to combine multiple checks into a deliverable verdict.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CheckCombiner {
    AllMustPass,
    AnyPass,
}

/// A deliverable as defined in the schema.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Deliverable {
    pub deliverable_id: String,
    pub description: String,
    pub acceptance_mode: AcceptanceMode,
    pub tier: Tier,
    pub check_combiner: CheckCombiner,
    pub checks: Vec<Check>,
    pub persisted_audit_script: String,
    pub self_test_script: String,
    #[serde(default)]
    pub related_stop_gates: Vec<String>,
    #[serde(default)]
    pub owner_review_required: bool,
}

/// Individual deliverable report within an audit run.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeliverableReport {
    pub deliverable_id: String,
    pub verdict: AuditorVerdict,
    pub checks: Vec<CheckReport>,
    pub execution_time_ms: u64,
}

/// Individual check report.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CheckReport {
    pub check_id: String,
    pub passed: bool,
    pub actual_exit_code: Option<i32>,
    pub actual_stdout: Option<String>,
    pub error: Option<String>,
    pub execution_time_ms: u64,
}

/// Blocked deliverable record.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BlockedDeliverable {
    pub deliverable_id: String,
    pub reason: BlockedReason,
    pub detail: String,
}

/// Audit metrics.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditMetrics {
    pub total_checks: u32,
    pub passed_checks: u32,
    pub failed_checks: u32,
    pub blocked_checks: u32,
    pub total_execution_time_ms: u64,
    pub canary_dropped_this_run: u32,
}

/// Structured auditor output.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditReport {
    pub schema_version: String,
    pub audit_run_id: String,
    pub started_at: String,
    pub completed_at: String,
    pub overall_verdict: AuditorVerdict,
    pub deliverables: Vec<DeliverableReport>,
    pub blocked_deliverables: Vec<BlockedDeliverable>,
    pub metrics: AuditMetrics,
    pub owner_feedback_prompt: String,
}

/// A single completed audit run (historical record).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditRun {
    pub run_id: String,
    pub started_at: String,
    pub completed_at: String,
    pub deliverables: Vec<DeliverableReport>,
    pub blocked_deliverables: Vec<BlockedDeliverable>,
}

/// Canary replay state entry.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CanaryState {
    pub audit_run_count: u64,
    pub last_canary_trigger: Option<String>,
    pub canary_dropped_this_run: u32,
    pub consecutive_drop_runs: u32,
}

impl Default for CanaryState {
    fn default() -> Self {
        Self {
            audit_run_count: 0,
            last_canary_trigger: None,
            canary_dropped_this_run: 0,
            consecutive_drop_runs: 0,
        }
    }
}
