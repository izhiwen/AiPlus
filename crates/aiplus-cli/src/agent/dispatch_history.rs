//! `aiplus agent dispatch-history` — read `.aiplus/agents/dispatch-log.jsonl`
//! and present a filtered view of past dispatches.
//!
//! P1.3 of the v0.5.11 testing close-out goal: until now the dispatch log
//! was append-only and could only be inspected by hand (`cat
//! .aiplus/agents/dispatch-log.jsonl | jq ...`). This subcommand makes
//! the log first-class queryable so the PI persona / Owner can answer
//! "what failed lately?" or "what has RA-Stata been doing?" without
//! grepping JSONL.
//!
//! Filters:
//!   --role <slug>        only this role
//!   --outcome <state>    only success / fail / canceled
//!   --since-days N       only entries newer than N days
//!   --json               machine-readable instead of table

use anyhow::{Context, Result};

use crate::agent::state::DispatchLogEntry;

const DISPATCH_LOG_PATH: &str = ".aiplus/agents/dispatch-log.jsonl";

pub fn handle_dispatch_history(
    role_filter: Option<&str>,
    outcome_filter: Option<&str>,
    since_days: Option<u64>,
    json: bool,
) -> Result<()> {
    let project_root = std::env::current_dir().context("get current directory")?;
    let log_path = project_root.join(DISPATCH_LOG_PATH);

    if !log_path.exists() {
        if json {
            println!("[]");
        } else {
            println!(
                "DISPATCH_HISTORY_STATUS=EMPTY reason=no_log_file path={}",
                log_path.display()
            );
        }
        return Ok(());
    }

    let text = std::fs::read_to_string(&log_path)
        .with_context(|| format!("read {}", log_path.display()))?;
    let mut entries: Vec<DispatchLogEntry> = Vec::new();
    for (idx, line) in text.lines().enumerate() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        match serde_json::from_str::<DispatchLogEntry>(line) {
            Ok(e) => entries.push(e),
            Err(err) => {
                eprintln!(
                    "DISPATCH_HISTORY_WARN reason=parse_failed line={} detail={err}",
                    idx + 1
                );
            }
        }
    }

    // Apply filters. epoch_millis() returns u128; widen everything we
    // compare against to keep arithmetic safe past 2106.
    let cutoff_ms: Option<u128> = since_days.map(|d| {
        let now_ms = aiplus_core::epoch_millis();
        let day_ms: u128 = 24 * 60 * 60 * 1000;
        now_ms.saturating_sub(d as u128 * day_ms)
    });

    let filtered: Vec<&DispatchLogEntry> = entries
        .iter()
        .filter(|e| match role_filter {
            Some(r) => e.role == r,
            None => true,
        })
        .filter(|e| match outcome_filter {
            Some(o) => e.outcome == o,
            None => true,
        })
        .filter(|e| match cutoff_ms {
            Some(cut) => parse_timestamp_ms(&e.timestamp)
                .map(|ms| (ms as u128) >= cut)
                .unwrap_or(true),
            None => true,
        })
        .collect();

    if json {
        let serialized = serde_json::to_string_pretty(&filtered)?;
        println!("{serialized}");
        return Ok(());
    }

    // Human-readable table.
    if filtered.is_empty() {
        println!("DISPATCH_HISTORY_STATUS=EMPTY reason=no_matches");
        println!(
            "filters: role={} outcome={} since_days={}",
            role_filter.unwrap_or("*"),
            outcome_filter.unwrap_or("*"),
            since_days
                .map(|d| d.to_string())
                .unwrap_or_else(|| "*".into())
        );
        return Ok(());
    }

    println!("DISPATCH_HISTORY total={}", filtered.len());
    println!(
        "{:<32} {:<14} {:<14} {:<10} {:<8}  task / reason",
        "timestamp", "role", "outcome", "tier", "source"
    );
    println!("{}", "-".repeat(110));
    for e in &filtered {
        let task_or_reason = match e.outcome.as_str() {
            "success" => truncate(&e.task, 50),
            _ => {
                let reason = e.error_reason.as_deref().unwrap_or("?");
                let detail = e.error_detail.as_deref().unwrap_or("");
                if detail.is_empty() {
                    format!("reason={reason}")
                } else {
                    format!("reason={reason} detail={}", truncate(detail, 35))
                }
            }
        };
        println!(
            "{:<32} {:<14} {:<14} {:<10} {:<8}  {}",
            truncate(&e.timestamp, 30),
            truncate(&e.role, 12),
            e.outcome,
            e.tier.as_deref().unwrap_or("unscored"),
            truncate(&e.source, 6),
            task_or_reason,
        );
    }
    Ok(())
}

fn truncate(s: &str, max: usize) -> String {
    if s.chars().count() <= max {
        s.to_string()
    } else {
        let kept: String = s.chars().take(max.saturating_sub(1)).collect();
        format!("{kept}…")
    }
}

/// Parse a `now_iso()` timestamp (e.g. "2026-05-14T01:23:45.678Z") to
/// epoch millis. Returns None if the timestamp doesn't fit the expected
/// shape — caller treats that as "include the entry rather than drop it"
/// so we never silently hide records due to parser fragility.
fn parse_timestamp_ms(ts: &str) -> Option<u64> {
    // Format: YYYY-MM-DDTHH:MM:SS(.sss)?Z
    // Extract fields and compute days-since-epoch + intra-day seconds.
    let bytes = ts.as_bytes();
    if bytes.len() < 20 {
        return None;
    }
    let year: u64 = std::str::from_utf8(&bytes[0..4]).ok()?.parse().ok()?;
    let month: u64 = std::str::from_utf8(&bytes[5..7]).ok()?.parse().ok()?;
    let day: u64 = std::str::from_utf8(&bytes[8..10]).ok()?.parse().ok()?;
    let hour: u64 = std::str::from_utf8(&bytes[11..13]).ok()?.parse().ok()?;
    let min: u64 = std::str::from_utf8(&bytes[14..16]).ok()?.parse().ok()?;
    let sec: u64 = std::str::from_utf8(&bytes[17..19]).ok()?.parse().ok()?;

    // Days from 1970-01-01 to YYYY-MM-DD using a simple Gregorian count.
    let mut days: u64 = 0;
    for y in 1970..year {
        days += if is_leap(y) { 366 } else { 365 };
    }
    let month_days = [
        31,
        if is_leap(year) { 29 } else { 28 },
        31,
        30,
        31,
        30,
        31,
        31,
        30,
        31,
        30,
        31,
    ];
    for m in 0..(month as usize - 1) {
        days += month_days[m];
    }
    days += day - 1;
    let seconds = days * 86400 + hour * 3600 + min * 60 + sec;
    Some(seconds * 1000)
}

fn is_leap(y: u64) -> bool {
    (y % 4 == 0 && y % 100 != 0) || (y % 400 == 0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn truncate_short_string_passes_through() {
        assert_eq!(truncate("hello", 50), "hello");
    }

    #[test]
    fn truncate_long_string_gets_ellipsis() {
        let out = truncate("0123456789", 5);
        assert!(out.ends_with('…'));
        assert!(out.chars().count() <= 5);
    }

    #[test]
    fn parse_timestamp_handles_iso() {
        // 2026-01-01T00:00:00Z is 1767225600000 ms after epoch.
        let ms = parse_timestamp_ms("2026-01-01T00:00:00.000Z").unwrap();
        assert_eq!(ms, 1767225600000);
    }

    #[test]
    fn parse_timestamp_rejects_garbage() {
        assert!(parse_timestamp_ms("not-a-timestamp").is_none());
        assert!(parse_timestamp_ms("").is_none());
    }

    #[test]
    fn is_leap_year_recognized() {
        assert!(is_leap(2024));
        assert!(is_leap(2000));
        assert!(!is_leap(1900));
        assert!(!is_leap(2025));
    }
}
