//! Persistent agent-team state. Phase D v0.
//!
//! The agent-team layer used to be entirely narrative — the PI persona
//! could *describe* a dispatch but no file system side effects landed,
//! which left users without any audit trail and made `aiplus agent
//! status` show `Active roles: []` indefinitely.
//!
//! This module persists two pieces of state under `.aiplus/agents/`:
//!
//! 1. `active-roles.json` — the current set of roles that have been
//!    dispatched at least once. `aiplus agent status` reads this.
//! 2. `dispatch-log.jsonl` — an append-only record of every dispatch
//!    routed via `aiplus agent route <role> ...`. Each entry pins the
//!    role, task description, timestamp, and the originating CLI
//!    invocation so the audit path can reconstruct what happened.
//!
//! Scope deliberately narrow: this is the *first* real persistent
//! side effect of the agent-team layer. Phase D v1 will extend it with
//! integration → completion → handoff state, but v0 just commits to
//! "PI dispatch produces a real artifact, not just prose."

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;
use std::path::Path;

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct ActiveRolesState {
    pub schema_version: String,
    pub active_roles: BTreeSet<String>,
}

/// P1.3: dispatch outcomes. A dispatch can succeed, fail (worktree
/// creation broke, role unknown, etc.), or be canceled by the owner
/// gate. Recording the outcome — not just successes — lets `aiplus
/// agent dispatch-history --outcome fail` show what went wrong.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DispatchOutcome<'a> {
    Success,
    Fail { reason: &'a str, detail: &'a str },
    Canceled { reason: &'a str },
}

impl<'a> DispatchOutcome<'a> {
    fn as_str(&self) -> &'static str {
        match self {
            DispatchOutcome::Success => "success",
            DispatchOutcome::Fail { .. } => "fail",
            DispatchOutcome::Canceled { .. } => "canceled",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DispatchLogEntry {
    pub schema_version: String,
    /// P1.3: stable unique ID for this dispatch. Format
    /// `dispatch-<epoch-ms>-<role>`. Linkable across dispatch-log.jsonl
    /// and the audit.jsonl mirror so `aiplus agent dispatch-history`
    /// can join them.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub dispatch_id: String,
    pub timestamp: String,
    pub role: String,
    /// Original role token from the command line when it differed from the
    /// canonical role recorded above, e.g. AEL `ceo` -> `pi`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub role_input: Option<String>,
    pub task: String,
    pub reversibility: String,
    pub source: String,
    /// Tier scored from the task description at dispatch time
    /// ("LIGHT" / "MEDIUM" / "HEAVY"). Older log entries that pre-date
    /// this field will deserialize with `tier = None`; the transcript
    /// renderer displays "unscored" for those.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tier: Option<String>,
    /// P1.3: dispatch outcome. Older entries that pre-date this field
    /// deserialize with outcome = "success" (the only thing that used to
    /// be recordable).
    #[serde(default = "default_outcome_success")]
    pub outcome: String,
    /// P1.3: when outcome != "success", a snake_case key naming the
    /// failure category. Omitted for successes.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub error_reason: Option<String>,
    /// P1.3: when outcome != "success", a human-readable detail for the
    /// failure. Omitted for successes.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub error_detail: Option<String>,
}

fn default_outcome_success() -> String {
    "success".to_string()
}

const ACTIVE_ROLES_PATH: &str = ".aiplus/agents/active-roles.json";
const DISPATCH_LOG_PATH: &str = ".aiplus/agents/dispatch-log.jsonl";

pub fn load_active_roles(project_root: &Path) -> Result<ActiveRolesState> {
    let path = project_root.join(ACTIVE_ROLES_PATH);
    if !path.exists() {
        return Ok(ActiveRolesState {
            schema_version: "0.1.0".to_string(),
            active_roles: BTreeSet::new(),
        });
    }
    let text =
        std::fs::read_to_string(&path).with_context(|| format!("read {}", path.display()))?;
    Ok(serde_json::from_str(&text).unwrap_or_default())
}

/// Mark `role` active and append a dispatch-log entry. Called from
/// `aiplus agent route <role>` whenever a known role is dispatched
/// successfully. Convenience wrapper around `record_dispatch_with_outcome`
/// for the happy path.
pub fn record_dispatch(
    project_root: &Path,
    role: &str,
    task: &str,
    source: &str,
) -> Result<String> {
    record_dispatch_with_outcome(project_root, role, task, source, DispatchOutcome::Success)
}

pub fn record_dispatch_with_role_input(
    project_root: &Path,
    role: &str,
    role_input: Option<&str>,
    task: &str,
    source: &str,
) -> Result<String> {
    record_dispatch_inner(
        project_root,
        role,
        role_input,
        task,
        source,
        DispatchOutcome::Success,
    )
}

/// P1.3: full-control variant. Records the dispatch outcome (success /
/// fail / canceled) and returns the dispatch_id so the caller can
/// reference it in subsequent audit-trail writes. Used by:
///   - the happy path via the `record_dispatch` wrapper above
///   - the worktree-creation failure path in route.rs
///   - the owner-gate cancellation path in route.rs
pub fn record_dispatch_with_outcome(
    project_root: &Path,
    role: &str,
    task: &str,
    source: &str,
    outcome: DispatchOutcome<'_>,
) -> Result<String> {
    record_dispatch_inner(project_root, role, None, task, source, outcome)
}

fn record_dispatch_inner(
    project_root: &Path,
    role: &str,
    role_input: Option<&str>,
    task: &str,
    source: &str,
    outcome: DispatchOutcome<'_>,
) -> Result<String> {
    let agents_dir = project_root.join(".aiplus").join("agents");
    std::fs::create_dir_all(&agents_dir).context("ensure .aiplus/agents/")?;

    let timestamp = aiplus_core::now_iso();
    let dispatch_id = format!("dispatch-{}-{}", aiplus_core::epoch_millis(), role);

    let tier = if task.is_empty() {
        None
    } else {
        let (t, _why) = score_task_tier(task);
        Some(t.to_string())
    };

    let (error_reason, error_detail) = match outcome {
        DispatchOutcome::Success => (None, None),
        DispatchOutcome::Fail { reason, detail } => {
            (Some(reason.to_string()), Some(detail.to_string()))
        }
        DispatchOutcome::Canceled { reason } => (Some(reason.to_string()), None),
    };

    let entry = DispatchLogEntry {
        schema_version: "0.4.0".to_string(),
        dispatch_id: dispatch_id.clone(),
        timestamp: timestamp.clone(),
        role: role.to_string(),
        role_input: role_input
            .filter(|input| *input != role)
            .map(|input| input.to_string()),
        task: task.to_string(),
        reversibility: "unspecified".to_string(),
        source: source.to_string(),
        tier,
        outcome: outcome.as_str().to_string(),
        error_reason,
        error_detail,
    };
    let log_path = project_root.join(DISPATCH_LOG_PATH);
    let mut log_value = serde_json::to_value(&entry)?;
    crate::agent::audit::verify_log::append_chained_jsonl_value(&log_path, &mut log_value)?;

    // Only mark role active on success — a failed/canceled dispatch
    // should not light up `aiplus agent status`'s active-roles list.
    if matches!(outcome, DispatchOutcome::Success) {
        let mut state = load_active_roles(project_root)?;
        state.schema_version = "0.1.0".to_string();
        state.active_roles.insert(role.to_string());
        let serialized = serde_json::to_string_pretty(&state)?;
        let active_path = project_root.join(ACTIVE_ROLES_PATH);
        aiplus_core::write_file_atomic(&active_path, serialized.as_bytes())?;
    }

    // Mirror to .aiplus/memory/audit.jsonl with the same dispatch_id so
    // `aiplus agent dispatch-history` (and future cross-trail joiners)
    // can correlate dispatch records to memory-audit entries.
    let memory_audit_path = project_root.join(".aiplus/memory/audit.jsonl");
    if let Some(parent) = memory_audit_path.parent() {
        if parent.exists() {
            let memory_entry = serde_json::json!({
                "schemaVersion": "0.2.0",
                "kind": "agent-dispatch",
                "dispatchId": dispatch_id,
                "timestamp": timestamp,
                "role": role,
                "roleInput": role_input.filter(|input| *input != role),
                "task": task,
                "source": source,
                "outcome": outcome.as_str(),
            });
            let memory_line = serde_json::to_string(&memory_entry)?;
            let _ = aiplus_core::append_jsonl_atomic(&memory_audit_path, &memory_line);
        }
    }
    Ok(dispatch_id)
}

/// Score a task description on the LIGHT/MEDIUM/HEAVY scale that
/// `aiplus-auto-team-consultant` and the PI persona use. The scoring is
/// keyword-driven and deliberately conservative — it errs on the side of
/// triggering a consult-before-plan suggestion rather than missing one.
///
/// Returns the tier and the human-readable rationale.
pub fn score_task_tier(task: &str) -> (&'static str, &'static str) {
    let lower = task.to_lowercase();
    let heavy_signals = [
        "submit",
        "submission",
        "structural",
        "redesign",
        "rewrite",
        "refactor",
        "schema",
        "migrate",
        "migration",
        "release",
        "deploy",
        "production",
        "r&r",
        "revise",
        "identification strategy",
        "authorship",
        // Issue #33: research-task vocabulary that should always score
        // HEAVY (Owner brief: "paper", "scope", "data", "rebuttal"
        // require heavy compounds — standalone words are too common to
        // promote without false positives, but these specific compounds
        // are unambiguous heavy research moves).
        "paper submission",
        "data acquisition plan",
        "referee response",
        "rebuttal letter",
    ];
    let medium_signals = [
        "robustness",
        "specification",
        "spec",
        "identification",
        "instrument",
        "fixed effect",
        "cluster",
        "merge",
        "integrate",
        "review",
        "audit",
        "rebuttal",
        "regression",
        // AEL / research-tuned: LLM-as-measurement and validity work
        // were missing from the original SWE-tuned set, so AEL tasks
        // like "design validity protocol for scoring documents across
        // 5 LLMs" were silently scored LIGHT. Adding these here keeps
        // the tier detection in sync with the AEL consultant team's
        // ai_integration seat triggers.
        "llm",
        "gpt",
        "claude",
        "gemini",
        "qwen",
        "deepseek",
        "validity",
        "validate",
        "multi-llm",
        "inter-rater",
        "held-out",
        "prompt-version",
        "scoring archival",
        "text-as-data",
        // AEL day-1 reproducibility seat triggers
        "archive",
        "gazetteer",
        "ocr",
        "pipeline",
        "makefile",
        "replication package",
        "aea data editor",
        // AEL IRB / disclosure gate seat triggers
        "irb",
        "consent",
        "restricted data",
        "dua",
        "small-cell",
        "re-identification",
        "anonymization",
        "pii",
        // AEL contribution framing seat triggers
        "intro",
        "abstract",
        "contribution",
        "placement",
        "target-journal",
        "comparable",
        "lit-gap",
        "differential",
        // Issue #33: research-task vocabulary that the original
        // SWE-flavored signal set missed. Owner brief flagged five
        // categories — "paper", "scope", "identification" (already
        // covered), "data", "rebuttal" (already covered). Standalone
        // "paper" / "scope" / "data" are too common, so use specific
        // compounds that are unambiguous research moves.
        "paper revision",
        "first paper",
        "scoping note",
        "scope cut",
        "scope expansion",
        "data acquisition",
        "data appendix",
        "data share",
        "referee",
        "iv exogeneity",
        "weak instrument",
        "treaty port",
        "main spec",
    ];
    let heavy_hits = heavy_signals
        .iter()
        .filter(|kw| lower.contains(*kw))
        .count();
    let medium_hits = medium_signals
        .iter()
        .filter(|kw| lower.contains(*kw))
        .count();
    if heavy_hits >= 1 {
        ("HEAVY", "task description contains submission / structural / redesign / migration / rewrite keywords")
    } else if medium_hits >= 2 {
        ("MEDIUM", "task description contains multiple specification / robustness / identification / review keywords")
    } else if medium_hits >= 1 {
        ("MEDIUM", "task description contains a specification / robustness / identification / review keyword")
    } else {
        ("LIGHT", "no high-risk signals detected in task description")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Issue #33: research-paper vocabulary regression tests.
    // Owner brief: heavy research moves like "scoping note", "data
    // acquisition", "referee response", "rebuttal" should fire the
    // consultant. Trivial work must keep falling through to LIGHT.

    #[test]
    fn heavy_research_compounds_score_heavy() {
        for task in [
            "paper submission to QJE for the X paper",
            "draft a referee response to the IV exogeneity criticism",
            "data acquisition plan for the treaty port project",
            "draft rebuttal letter for the R&R",
        ] {
            let (tier, _) = score_task_tier(task);
            assert_eq!(tier, "HEAVY", "{task:?} should score HEAVY, got {tier}");
        }
    }

    #[test]
    fn research_vocab_promotes_to_at_least_medium() {
        for task in [
            "draft scoping note for the new project",
            "respond to referee 2 on weak instrument concern",
            "rework the data appendix structure",
            "rewrite paper revision section",
            "first paper main spec discussion",
        ] {
            let (tier, _) = score_task_tier(task);
            assert_ne!(
                tier, "LIGHT",
                "{task:?} should reach the consultant team (MEDIUM/HEAVY), \
                 but scored LIGHT"
            );
        }
    }

    #[test]
    fn light_regression_guard_for_trivial_tasks() {
        for task in [
            "fix typo in README",
            "rename a variable",
            "bump version number",
        ] {
            let (tier, _) = score_task_tier(task);
            assert_eq!(
                tier, "LIGHT",
                "{task:?} should remain LIGHT after the #33 signal additions"
            );
        }
    }
}
