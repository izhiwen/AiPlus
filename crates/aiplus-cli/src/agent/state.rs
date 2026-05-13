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
    let text = std::fs::read_to_string(&path)
        .with_context(|| format!("read {}", path.display()))?;
    Ok(serde_json::from_str(&text).unwrap_or_default())
}

/// Mark `role` active and append a dispatch-log entry. Called from
/// `aiplus agent route <role>` whenever a known role is dispatched.
pub fn record_dispatch(
    project_root: &Path,
    role: &str,
    task: &str,
    source: &str,
) -> Result<()> {
    let agents_dir = project_root.join(".aiplus").join("agents");
    std::fs::create_dir_all(&agents_dir).context("ensure .aiplus/agents/")?;

    // 1) Append a dispatch-log line.
    let entry = DispatchLogEntry {
        schema_version: "0.1.0".to_string(),
        timestamp: aiplus_core::now_iso(),
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
    Ok(())
}
