use anyhow::{Context, Result};
use serde::Deserialize;
use std::fs::File;
use std::io::{BufRead, BufReader};

#[derive(Debug, Deserialize)]
struct DispatchRecord {
    #[serde(default)]
    timestamp: Option<String>,
    #[serde(default)]
    role: Option<String>,
    #[serde(default)]
    task: Option<String>,
    #[serde(default)]
    reversibility: Option<String>,
    #[serde(default)]
    source: Option<String>,
    #[serde(default)]
    tier: Option<String>,
}

/// Render the dispatch log as a human-readable transcript.
///
/// Reads `.aiplus/agents/dispatch-log.jsonl`, prints each entry as a
/// timestamped block. Most recent dispatches at the bottom (chronological).
pub fn handle_transcript() -> Result<()> {
    let project_root = std::env::current_dir()?;
    let log_path = project_root
        .join(".aiplus")
        .join("agents")
        .join("dispatch-log.jsonl");

    if !log_path.exists() {
        println!("AGENT_TRANSCRIPT");
        println!("status=empty");
        println!("path={}", log_path.display());
        println!(
            "next=run `aiplus agent route <role> \"<task>\"` to record the first dispatch"
        );
        return Ok(());
    }

    let file = File::open(&log_path)
        .with_context(|| format!("opening dispatch log at {}", log_path.display()))?;
    let reader = BufReader::new(file);

    let mut count = 0usize;
    let mut parse_errors = 0usize;
    println!("AGENT_TRANSCRIPT");
    println!("path={}", log_path.display());
    println!("---");

    for (line_no, raw) in reader.lines().enumerate() {
        let raw = match raw {
            Ok(line) if line.trim().is_empty() => continue,
            Ok(line) => line,
            Err(e) => {
                parse_errors += 1;
                eprintln!("WARN: line {} read error: {}", line_no + 1, e);
                continue;
            }
        };
        let record: DispatchRecord = match serde_json::from_str(&raw) {
            Ok(r) => r,
            Err(e) => {
                parse_errors += 1;
                eprintln!("WARN: line {} parse error: {}", line_no + 1, e);
                continue;
            }
        };
        count += 1;
        let timestamp = record.timestamp.as_deref().unwrap_or("(no timestamp)");
        let role = record.role.as_deref().unwrap_or("(no role)");
        let task = record.task.as_deref().unwrap_or("(no task)");
        let source = record.source.as_deref().unwrap_or("(no source)");
        let reversibility = record.reversibility.as_deref().unwrap_or("unspecified");
        let tier = record.tier.as_deref().unwrap_or("unscored");

        println!("[{timestamp}] {role}");
        println!("  task: {task}");
        println!("  tier: {tier}  reversibility: {reversibility}  source: {source}");
        println!();
    }

    println!("---");
    println!("total_dispatches={count}");
    if parse_errors > 0 {
        println!("parse_errors={parse_errors}");
        println!(
            "next=inspect {} for malformed JSONL lines",
            log_path.display()
        );
    }

    Ok(())
}
