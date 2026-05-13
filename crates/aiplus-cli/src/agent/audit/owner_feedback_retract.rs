use anyhow::{Context, Result};

const OWNER_FEEDBACK_PATH: &str = ".aiplus/agent-team/audit-trail/owner-feedback.jsonl";

/// Entry point for `audit owner-feedback-retract`.
pub fn handle_owner_feedback_retract(run_id: &str) -> Result<()> {
    let cwd = std::env::current_dir().context("failed to get current directory")?;
    let feedback_path = cwd.join(OWNER_FEEDBACK_PATH);

    let retraction = serde_json::json!({
        "event": "owner_feedback_retract",
        "audit_run_id": run_id,
        "retracted": true,
        "timestamp": aiplus_core::now_iso(),
    });

    aiplus_core::append_jsonl_atomic(&feedback_path, &retraction.to_string())
        .with_context(|| "failed to write owner-feedback.jsonl")?;

    println!("Owner feedback retracted for run {}", run_id);
    Ok(())
}
