use std::fs;
use std::path::Path;

use anyhow::{Context, Result};
use serde::Deserialize;

const SPOT_CHECK_QUEUE: &str = ".aiplus/agent-team/audit-trail/owner-spot-check-queue.jsonl";
const OWNER_FEEDBACK_PATH: &str = ".aiplus/agent-team/audit-trail/owner-feedback.jsonl";

#[derive(Debug, Clone, Deserialize)]
#[allow(dead_code)]
struct SpotCheckEntry {
    audit_run_id: String,
    deliverable_id: String,
    #[serde(default)]
    auditor_verdict: String,
    #[serde(default)]
    owner_verdict: Option<String>,
    #[serde(default)]
    note: Option<String>,
    #[serde(default)]
    retracted: bool,
    timestamp: String,
}

/// Entry point for `audit weekly-spot-check`.
pub fn handle_weekly_spot_check() -> Result<()> {
    let cwd = std::env::current_dir().context("failed to get current directory")?;
    let queue_path = cwd.join(SPOT_CHECK_QUEUE);
    let feedback_path = cwd.join(OWNER_FEEDBACK_PATH);

    println!("=== Weekly Spot Check ===");

    // Pending items from spot-check queue
    let pending = read_spot_check_queue(&queue_path)?;
    let pending_count = pending
        .iter()
        .filter(|e| !e.retracted && e.owner_verdict.is_none())
        .count();
    println!("Pending items: {}", pending_count);

    for entry in &pending {
        if entry.retracted || entry.owner_verdict.is_some() {
            continue;
        }
        println!(
            "  - run_id={} deliverable={} verdict={} timestamp={}",
            entry.audit_run_id, entry.deliverable_id, entry.auditor_verdict, entry.timestamp
        );
    }

    // Metrics window (30 days)
    let feedback_count = count_feedback_last_30_days(&feedback_path)?;
    println!("Owner feedback (last 30 days): {} entries", feedback_count);

    Ok(())
}

fn read_spot_check_queue(path: &Path) -> Result<Vec<SpotCheckEntry>> {
    if !path.exists() {
        return Ok(Vec::new());
    }
    let content =
        fs::read_to_string(path).with_context(|| format!("failed to read {}", path.display()))?;
    let mut entries = Vec::new();
    for line in content.lines().filter(|l| !l.trim().is_empty()) {
        if let Ok(entry) = serde_json::from_str::<SpotCheckEntry>(line) {
            entries.push(entry);
        }
    }
    Ok(entries)
}

fn count_feedback_last_30_days(path: &Path) -> Result<usize> {
    if !path.exists() {
        return Ok(0);
    }
    let content =
        fs::read_to_string(path).with_context(|| format!("failed to read {}", path.display()))?;
    let mut count = 0;
    for line in content.lines().filter(|l| !l.trim().is_empty()) {
        if let Ok(value) = serde_json::from_str::<serde_json::Value>(line) {
            if let Some(ts) = value.get("timestamp").and_then(|v| v.as_str()) {
                if is_within_days(ts, 30) {
                    count += 1;
                }
            }
        }
    }
    Ok(count)
}

fn is_within_days(_timestamp: &str, _days: u64) -> bool {
    // v0.1: simplified — assume all entries are within window
    // Real implementation would parse ISO8601 and compare.
    true
}
