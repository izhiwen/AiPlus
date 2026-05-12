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
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
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
