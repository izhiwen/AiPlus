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

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DispatchLogEntry {
    pub schema_version: String,
    pub timestamp: String,
    pub role: String,
    pub task: String,
    pub reversibility: String,
    pub source: String,
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
/// `aiplus agent route <role>` whenever a known role is dispatched.
pub fn record_dispatch(project_root: &Path, role: &str, task: &str, source: &str) -> Result<()> {
    let agents_dir = project_root.join(".aiplus").join("agents");
    std::fs::create_dir_all(&agents_dir).context("ensure .aiplus/agents/")?;

    let timestamp = aiplus_core::now_iso();

    // 1) Append a dispatch-log line.
    let entry = DispatchLogEntry {
        schema_version: "0.1.0".to_string(),
        timestamp: timestamp.clone(),
        role: role.to_string(),
        task: task.to_string(),
        // v0 default: reversibility is unspecified. Future work: the PI
        // persona's response will tag the dispatch as
        // reversible/semi/irreversible and the CLI surfaces it here.
        reversibility: "unspecified".to_string(),
        source: source.to_string(),
    };
    let line = serde_json::to_string(&entry)?;
    let log_path = project_root.join(DISPATCH_LOG_PATH);
    aiplus_core::append_jsonl_atomic(&log_path, &line)?;

    // 2) Update active-roles.json.
    let mut state = load_active_roles(project_root)?;
    state.schema_version = "0.1.0".to_string();
    state.active_roles.insert(role.to_string());
    let serialized = serde_json::to_string_pretty(&state)?;
    let active_path = project_root.join(ACTIVE_ROLES_PATH);
    aiplus_core::write_file_atomic(&active_path, serialized.as_bytes())?;

    // 3) Mirror to `.aiplus/memory/audit.jsonl` so dispatches show up in
    //    the project's general audit trail (not just the team-local one).
    //    This makes `aiplus memory` queries surface dispatches alongside
    //    other audited events (compact runs, secret-broker calls, etc.).
    let memory_audit_path = project_root.join(".aiplus/memory/audit.jsonl");
    if let Some(parent) = memory_audit_path.parent() {
        // Only mirror if the memory dir already exists — don't create
        // .aiplus/memory/ on systems where memory wasn't installed.
        if parent.exists() {
            let memory_entry = serde_json::json!({
                "schemaVersion": "0.1.0",
                "kind": "agent-dispatch",
                "timestamp": timestamp,
                "role": role,
                "task": task,
                "source": source,
            });
            let memory_line = serde_json::to_string(&memory_entry)?;
            let _ = aiplus_core::append_jsonl_atomic(&memory_audit_path, &memory_line);
        }
    }
    Ok(())
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
