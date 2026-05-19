use crate::error::Result;
use crate::pricing::PricingTable;
use crate::rollup::{rollup_from_dispatch_log, WindowSpec};
use chrono::{DateTime, Duration, Utc};
use serde_json::Value;
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::Path;

pub fn maybe_write_hourly_snapshot(
    log_path: &Path,
    snapshot_path: &Path,
    pricing: &PricingTable,
    now: DateTime<Utc>,
    windows: &[WindowSpec],
    top_n: usize,
) -> Result<bool> {
    if let Some(last) = last_snapshot_timestamp(snapshot_path)? {
        if now - last < Duration::hours(1) {
            return Ok(false);
        }
    }

    if let Some(parent) = snapshot_path.parent() {
        fs::create_dir_all(parent)?;
    }
    let windows = rollup_from_dispatch_log(log_path, pricing, now, windows, top_n)?;
    let line = serde_json::json!({
        "schemaVersion": "0.1.0",
        "event": "token_cost_snapshot",
        "timestamp": now.to_rfc3339(),
        "windows": windows,
        "secretValues": "none"
    });
    let mut file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(snapshot_path)?;
    writeln!(file, "{line}")?;
    Ok(true)
}

fn last_snapshot_timestamp(snapshot_path: &Path) -> Result<Option<DateTime<Utc>>> {
    if !snapshot_path.exists() {
        return Ok(None);
    }
    let body = fs::read_to_string(snapshot_path)?;
    for line in body.lines().rev() {
        let Ok(value) = serde_json::from_str::<Value>(line) else {
            continue;
        };
        let Some(timestamp) = value.get("timestamp").and_then(Value::as_str) else {
            continue;
        };
        let Ok(parsed) = DateTime::parse_from_rfc3339(timestamp) else {
            continue;
        };
        return Ok(Some(parsed.with_timezone(&Utc)));
    }
    Ok(None)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rollup::default_windows;

    #[test]
    fn snapshot_writes_once_per_hour() {
        let temp = tempfile::tempdir().unwrap();
        let log = temp.path().join("dispatch-log.jsonl");
        fs::write(&log, "").unwrap();
        let snapshot = temp.path().join("token-cost-snapshots.jsonl");
        let pricing = PricingTable::embedded();
        let now = Utc::now();

        assert!(
            maybe_write_hourly_snapshot(&log, &snapshot, &pricing, now, &default_windows(), 5)
                .unwrap()
        );
        assert!(!maybe_write_hourly_snapshot(
            &log,
            &snapshot,
            &pricing,
            now + Duration::minutes(30),
            &default_windows(),
            5
        )
        .unwrap());
        assert!(maybe_write_hourly_snapshot(
            &log,
            &snapshot,
            &pricing,
            now + Duration::hours(1),
            &default_windows(),
            5
        )
        .unwrap());
    }
}
