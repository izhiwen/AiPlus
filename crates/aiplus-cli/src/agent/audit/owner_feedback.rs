use std::fs;
use std::path::Path;

use anyhow::{anyhow, Context, Result};

use aiplus_core::agent_team::types::AuditRun;

const AUDIT_RUNS_PATH: &str = ".aiplus/agent-team/audit-trail/audit-runs.jsonl";
const OWNER_FEEDBACK_PATH: &str = ".aiplus/agent-team/audit-trail/owner-feedback.jsonl";

/// Entry point for `audit owner-feedback`.
pub fn handle_owner_feedback(run_id: &str, actual_verdict: &str, note: Option<&str>) -> Result<()> {
    let cwd = std::env::current_dir().context("failed to get current directory")?;
    let audit_runs_path = cwd.join(AUDIT_RUNS_PATH);
    let feedback_path = cwd.join(OWNER_FEEDBACK_PATH);

    // Validate audit_run_id exists
    let run = find_audit_run(&audit_runs_path, run_id)?
        .ok_or_else(|| anyhow!("audit run {} not found", run_id))?;

    let feedback = serde_json::json!({
        "event": "owner_feedback",
        "audit_run_id": run_id,
        "actual_verdict": actual_verdict,
        "note": note.unwrap_or(""),
        "original_verdict": format!("{:?}", run.deliverables.first().map(|d| d.verdict)),
        "timestamp": aiplus_core::now_iso(),
    });

    aiplus_core::append_jsonl_atomic(&feedback_path, &feedback.to_string())
        .with_context(|| "failed to write owner-feedback.jsonl")?;

    println!("Owner feedback recorded for run {}", run_id);
    Ok(())
}

fn find_audit_run(path: &Path, run_id: &str) -> Result<Option<AuditRun>> {
    if !path.exists() {
        return Ok(None);
    }
    let content =
        fs::read_to_string(path).with_context(|| format!("failed to read {}", path.display()))?;
    for line in content.lines().filter(|l| !l.trim().is_empty()) {
        let run: AuditRun = serde_json::from_str(line)
            .with_context(|| format!("failed to parse audit run line: {line}"))?;
        if run.run_id == run_id {
            return Ok(Some(run));
        }
    }
    Ok(None)
}
