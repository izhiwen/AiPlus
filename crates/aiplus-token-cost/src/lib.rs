pub mod embedded;
pub mod error;
pub mod pricing;
pub mod rollup;
pub mod snapshot;

use crate::error::Result;
use crate::pricing::PricingTable;
use crate::rollup::{default_windows, parse_window, rollup_from_dispatch_log, RollupResult};
use crate::snapshot::maybe_write_hourly_snapshot;
use chrono::Utc;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone)]
pub struct TokenCostOptions {
    pub by_role: bool,
    pub window: Option<String>,
    pub top_n: usize,
}

#[derive(Debug, Clone)]
pub struct TokenCostReport {
    pub pricing_source: String,
    pub pricing_entries: usize,
    pub dispatch_log: PathBuf,
    pub snapshot_path: PathBuf,
    pub snapshot_written: bool,
    pub windows: Vec<RollupResult>,
    pub warnings: Vec<String>,
}

impl Default for TokenCostOptions {
    fn default() -> Self {
        Self {
            by_role: false,
            window: None,
            top_n: 5,
        }
    }
}

pub fn run_token_cost(project_root: &Path, options: &TokenCostOptions) -> Result<TokenCostReport> {
    let windows = if let Some(label) = &options.window {
        vec![parse_window(label)?]
    } else {
        default_windows()
    };
    let top_n = options.top_n.max(1);
    let pricing = PricingTable::load(project_root);
    let dispatch_log = project_root.join(".aiplus/agents/dispatch-log.jsonl");
    let snapshot_path = project_root.join(".aiplus/agents/token-cost-snapshots.jsonl");
    let now = Utc::now();
    let snapshot_written = maybe_write_hourly_snapshot(
        &dispatch_log,
        &snapshot_path,
        &pricing,
        now,
        &windows,
        top_n,
    )?;
    let rollups = rollup_from_dispatch_log(&dispatch_log, &pricing, now, &windows, top_n)?;
    let warnings = pricing.warnings().to_vec();

    Ok(TokenCostReport {
        pricing_source: pricing.source().to_string(),
        pricing_entries: pricing.len(),
        dispatch_log,
        snapshot_path,
        snapshot_written,
        windows: rollups,
        warnings,
    })
}

pub fn format_report(report: &TokenCostReport, by_role: bool) -> String {
    let mut out = String::new();
    out.push_str("AIPLUS_TOKEN_COST\n");
    out.push_str(&format!("pricing_source={}\n", report.pricing_source));
    out.push_str(&format!("pricing_entries={}\n", report.pricing_entries));
    out.push_str(&format!("dispatch_log={}\n", report.dispatch_log.display()));
    out.push_str(&format!(
        "snapshot_path={}\n",
        report.snapshot_path.display()
    ));
    out.push_str(&format!("snapshot_written={}\n", report.snapshot_written));
    for warning in &report.warnings {
        out.push_str(&format!("WARN {warning}\n"));
    }

    for window in &report.windows {
        out.push('\n');
        out.push_str(&format!(
            "WINDOW {} total_tokens={} total_usd={:.6}\n",
            window.window_label, window.total_tokens, window.total_usd
        ));
        out.push_str("TOP_TASKS\n");
        if window.top_tasks.is_empty() {
            out.push_str("(none)\n");
        } else {
            for (index, task) in window.top_tasks.iter().enumerate() {
                out.push_str(&format!(
                    "{}. usd={:.6} tokens={} role={} provider={} model={} key={} task=\"{}\"\n",
                    index + 1,
                    task.usd,
                    task.total_tokens,
                    task.role,
                    task.provider,
                    task.model,
                    task.key,
                    task.task_excerpt.replace('"', "'")
                ));
            }
        }
        if by_role {
            out.push_str("BY_ROLE\n");
            if window.by_role.is_empty() {
                out.push_str("(none)\n");
            } else {
                for (role, cost) in &window.by_role {
                    out.push_str(&format!(
                        "{} tokens={} input={} output={} usd={:.6}\n",
                        role, cost.total_tokens, cost.input_tokens, cost.output_tokens, cost.usd
                    ));
                }
            }
        }
        for warning in &window.warnings {
            out.push_str(&format!("WARN {} {warning}\n", window.window_label));
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;
    use std::fs;

    #[test]
    fn report_formatter_includes_role_section_when_requested() {
        let temp = tempfile::tempdir().unwrap();
        let log = temp.path().join(".aiplus/agents/dispatch-log.jsonl");
        fs::create_dir_all(log.parent().unwrap()).unwrap();
        let line = serde_json::json!({
            "timestamp": Utc::now().to_rfc3339(),
            "dispatchId": "dispatch-1",
            "role": "engineer-a",
            "task": "implement payment",
            "provider": "anthropic",
            "model": "claude-sonnet-4-6",
            "usage_tokens": {"input_tokens": 100, "output_tokens": 10}
        });
        fs::write(&log, format!("{line}\n")).unwrap();
        let report = run_token_cost(temp.path(), &TokenCostOptions::default()).unwrap();
        let output = format_report(&report, true);
        assert!(output.contains("AIPLUS_TOKEN_COST"));
        assert!(output.contains("WINDOW 1h"));
        assert!(output.contains("BY_ROLE"));
        assert!(output.contains("engineer-a"));
    }
}
