use std::fs;
use std::path::Path;

use anyhow::{Context, Result};

const AUDIT_RUNS_PATH: &str = ".aiplus/agent-team/audit-trail/audit-runs.jsonl";
const SPOT_CHECK_QUEUE: &str = ".aiplus/agent-team/audit-trail/owner-spot-check-queue.jsonl";
const CANARY_STATE_PATH: &str = ".aiplus/agent-team/audit-trail/canary-replay-state.jsonl";
const DRIFT_PATH: &str = ".aiplus/agent-team/audit-trail/drift-findings.jsonl";

/// Entry point for `audit status`.
pub fn handle_status() -> Result<()> {
    let cwd = std::env::current_dir().context("failed to get current directory")?;

    // Last run timestamp
    let last_run = read_last_audit_run(&cwd.join(AUDIT_RUNS_PATH))?;
    println!("=== Audit System Status ===");
    match last_run {
        Some(ts) => println!("Last audit run: {}", ts),
        None => println!("Last audit run: never"),
    }

    // Pending items count
    let pending = count_pending_spot_checks(&cwd.join(SPOT_CHECK_QUEUE))?;
    println!("Pending spot-check items: {}", pending);

    // Canary state
    let canary = read_canary_state(&cwd.join(CANARY_STATE_PATH))?;
    println!("Canary state: {}", canary);

    // Drift status
    let drift_count = count_drift_findings(&cwd.join(DRIFT_PATH))?;
    println!("Active drift findings: {}", drift_count);

    Ok(())
}

fn read_last_audit_run(path: &Path) -> Result<Option<String>> {
    if !path.exists() {
        return Ok(None);
    }
    let content = fs::read_to_string(path)
        .with_context(|| format!("failed to read {}", path.display()))?;
    let mut last_ts = None;
    for line in content.lines().filter(|l| !l.trim().is_empty()) {
        if let Ok(value) = serde_json::from_str::<serde_json::Value>(line) {
            if let Some(ts) = value.get("completed_at").and_then(|v| v.as_str()) {
                last_ts = Some(ts.to_string());
            }
        }
    }
    Ok(last_ts)
}

fn count_pending_spot_checks(path: &Path) -> Result<usize> {
    if !path.exists() {
        return Ok(0);
    }
    let content = fs::read_to_string(path)
        .with_context(|| format!("failed to read {}", path.display()))?;
    let mut pending = 0;
    for line in content.lines().filter(|l| !l.trim().is_empty()) {
        if let Ok(value) = serde_json::from_str::<serde_json::Value>(line) {
            let retracted = value.get("retracted").and_then(|v| v.as_bool()).unwrap_or(false);
            let has_owner = value.get("owner_verdict").is_some();
            if !retracted && !has_owner {
                pending += 1;
            }
        }
    }
    Ok(pending)
}

fn read_canary_state(path: &Path) -> Result<String> {
    if !path.exists() {
        return Ok("not initialized".to_string());
    }
    let content = fs::read_to_string(path)
        .with_context(|| format!("failed to read {}", path.display()))?;
    // Read the last line (most recent state)
    let last_line = content.lines().filter(|l| !l.trim().is_empty()).last();
    match last_line {
        Some(line) => {
            if let Ok(value) = serde_json::from_str::<serde_json::Value>(line) {
                let count = value.get("audit_run_count").and_then(|v| v.as_u64()).unwrap_or(0);
                let drops = value
                    .get("consecutive_drop_runs")
                    .and_then(|v| v.as_u64())
                    .unwrap_or(0);
                Ok(format!("runs={}, consecutive_drops={}", count, drops))
            } else {
                Ok("unparseable".to_string())
            }
        }
        None => Ok("empty".to_string()),
    }
}

fn count_drift_findings(path: &Path) -> Result<usize> {
    if !path.exists() {
        return Ok(0);
    }
    let content = fs::read_to_string(path)
        .with_context(|| format!("failed to read {}", path.display()))?;
    Ok(content.lines().filter(|l| !l.trim().is_empty()).count())
}
