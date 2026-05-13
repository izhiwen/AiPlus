use anyhow::{anyhow, Context, Result};

const FORCE_SKIPS_PATH: &str = ".aiplus/agent-team/audit-trail/force-skips.jsonl";

/// Entry point for `audit force-skip`.
pub fn handle_force_skip(gate_id: &str, reason: &str) -> Result<()> {
    if reason.trim().is_empty() {
        return Err(anyhow!("--reason is required for force-skip"));
    }

    let cwd = std::env::current_dir().context("failed to get current directory")?;
    let skips_path = cwd.join(FORCE_SKIPS_PATH);

    let entry = serde_json::json!({
        "event": "force_skip",
        "gate_id": gate_id,
        "reason": reason,
        "flagged": true,
        "timestamp": aiplus_core::now_iso(),
    });

    aiplus_core::append_jsonl_atomic(&skips_path, &entry.to_string())
        .with_context(|| "failed to write force-skips.jsonl")?;

    println!(
        "Force-skip recorded for gate {} (flagged in report)",
        gate_id
    );
    Ok(())
}
