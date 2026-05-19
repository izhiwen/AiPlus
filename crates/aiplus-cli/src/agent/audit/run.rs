use std::fs;
use std::path::Path;
use std::process::Command;
use std::time::SystemTime;

use anyhow::{Context, Result};
use serde::Deserialize;
use sha2::{Digest, Sha256};

use aiplus_core::agent_team::types::{
    AuditMetrics, AuditReport, AuditRun, AuditorVerdict, BlockedDeliverable, Check, CheckCombiner,
    CheckKind, CheckReport, Deliverable, DeliverableReport,
};
use aiplus_core::auditor::drift::{DriftDetector, DriftThresholds};
use aiplus_core::auditor::evidence_collector::{CheckEvidence, EvidenceCollector};
use aiplus_core::auditor::gate::{GateResult, PreAuditGate};

const EMBEDDED_SCHEMA_PATH: &str = "aiplus-agent-team/core/schemas/acceptance-v0.1.4.yaml";
const SCHEMA_SHA256: &str = "06ee2b35466f6bd2019dbed3bf70384f98428f5eacd6cc117ba2e74fcaf5b526";
// TODO(audit-migration): referenced when migrating projects from
// pre-v0.1.4 schema layout. Not used in steady-state — kept as a
// documented constant for the eventual migration path.
#[allow(dead_code)]
const LEGACY_SCHEMA_PATH: &str = ".aiplus/agent-team/acceptance/v0.1.4/schema.yaml";
const AUDIT_TRAIL_DIR: &str = ".aiplus/agent-team/audit-trail";
const RELEASE_MANIFEST_PATH: &str = ".aiplus/agent-team/release-manifest.yaml";
const FINGERPRINT_PATH: &str = ".aiplus/agent-team/owner-key-fingerprint";
const SENTINEL_PATH: &str = ".aiplus/agent-team/.owner-setup-authorized";

type CheckExecution = (bool, Option<i32>, Option<String>, Option<String>);

#[derive(Debug, Clone, Deserialize)]
struct SchemaFile {
    #[serde(default)]
    #[allow(dead_code)]
    schema_version: String,
    #[serde(default)]
    deliverables: Vec<Deliverable>,
}

/// Entry point for `audit run`.
pub fn handle_run(
    deliverable_filter: Option<&str>,
    _mode: &str,
    schema_path_override: Option<&str>,
) -> Result<()> {
    let cwd = std::env::current_dir().context("failed to get current directory")?;
    let manifest_path = cwd.join(RELEASE_MANIFEST_PATH);
    let lock_path = cwd.join(".aiplus/agent-team/.audit.lock");
    let fingerprint_path = cwd.join(FINGERPRINT_PATH);
    let sentinel_path = cwd.join(SENTINEL_PATH);
    let audit_trail = cwd.join(AUDIT_TRAIL_DIR);

    // Run pre-audit gate
    let gate = PreAuditGate::new(
        &manifest_path,
        &lock_path,
        &fingerprint_path,
        &sentinel_path,
    );
    let gate_result = gate.run().context("pre-audit gate failed")?;

    let run_id = format!("audit-run-{}", aiplus_core::epoch_millis());
    let started_at = aiplus_core::now_iso();

    let mut blocked_deliverables: Vec<BlockedDeliverable> = Vec::new();
    let mut deliverable_reports: Vec<DeliverableReport> = Vec::new();
    let mut collector = EvidenceCollector::new();
    let mut metrics = AuditMetrics {
        total_checks: 0,
        passed_checks: 0,
        failed_checks: 0,
        blocked_checks: 0,
        total_execution_time_ms: 0,
        canary_dropped_this_run: 0,
    };

    match gate_result {
        GateResult::Passed => {
            // Load schema (embedded by default, overridden via --schema-path)
            let schema = load_schema(schema_path_override)?;
            let deliverables = filter_deliverables(schema.deliverables, deliverable_filter);

            for deliverable in deliverables {
                let start = SystemTime::now();
                let (report, evidence_vec) = run_deliverable(&deliverable, &cwd)?;
                let elapsed = start.elapsed().unwrap_or_default().as_millis() as u64;

                metrics.total_checks += report.checks.len() as u32;
                for check in &report.checks {
                    if check.passed {
                        metrics.passed_checks += 1;
                    } else {
                        metrics.failed_checks += 1;
                    }
                }
                metrics.total_execution_time_ms += elapsed;

                for ev in evidence_vec {
                    collector.collect(ev);
                }

                deliverable_reports.push(report);
            }
        }
        GateResult::Blocked(reason) => {
            let detail = format!("{:?}", reason);
            let is_first_run = detail.contains("OwnershipUnverified");
            blocked_deliverables.push(BlockedDeliverable {
                deliverable_id: deliverable_filter.unwrap_or("*").to_string(),
                reason,
                detail: detail.clone(),
            });
            metrics.blocked_checks = 1;
            println!("AUDIT_BLOCKED: {}", detail);
            if is_first_run {
                println!(
                    "next=run `aiplus agent audit setup-gpg` to register Owner ownership (one-time first-run wizard)"
                );
            }
        }
        GateResult::AuditInProgress => {
            println!("AUDIT_BLOCKED: Another audit is currently in progress");
            return Ok(());
        }
    }

    let overall_verdict = compute_overall_verdict(&deliverable_reports, &blocked_deliverables);
    let completed_at = aiplus_core::now_iso();

    // Write audit run to JSONL
    let audit_run = AuditRun {
        run_id: run_id.clone(),
        started_at: started_at.clone(),
        completed_at: completed_at.clone(),
        deliverables: deliverable_reports.clone(),
        blocked_deliverables: blocked_deliverables.clone(),
    };
    let audit_runs_path = audit_trail.join("audit-runs.jsonl");
    let line = serde_json::to_string(&audit_run).context("failed to serialize audit run")?;
    aiplus_core::append_jsonl_atomic(&audit_runs_path, &line)
        .with_context(|| "failed to write audit-runs.jsonl")?;

    // Drift detection (v0.1: instantiate fresh; history loading can be added later)
    let _drift_detector = DriftDetector::new(DriftThresholds::default());

    let report = AuditReport {
        schema_version: "0.1.4".to_string(),
        audit_run_id: run_id,
        started_at,
        completed_at,
        overall_verdict,
        deliverables: deliverable_reports,
        blocked_deliverables,
        metrics,
        owner_feedback_prompt: format!(
            "If you disagree with verdict {:?}, run:\n  aiplus agent audit owner-feedback <run-id> --actual-verdict PASS|FAIL --note \"...\"",
            overall_verdict
        ),
    };

    let yaml = serde_yaml_ng::to_string(&report).context("failed to serialize report to YAML")?;
    println!("{yaml}");

    Ok(())
}

fn load_schema(schema_path_override: Option<&str>) -> Result<SchemaFile> {
    let content = if let Some(path) = schema_path_override {
        fs::read_to_string(path)
            .with_context(|| format!("failed to read schema override at {path}"))?
    } else {
        let embedded = aiplus_core::embedded_asset_text(EMBEDDED_SCHEMA_PATH)
            .with_context(|| "embedded acceptance schema not found")?;
        // Verify SHA256 of embedded asset matches expected
        let mut hasher = Sha256::new();
        hasher.update(&embedded);
        let computed = hex::encode(hasher.finalize());
        anyhow::ensure!(
            computed == SCHEMA_SHA256,
            "embedded schema SHA256 mismatch: expected {SCHEMA_SHA256}, got {computed}"
        );
        embedded
    };
    let schema: SchemaFile =
        serde_yaml_ng::from_str(&content).with_context(|| "failed to parse acceptance schema")?;
    Ok(schema)
}

fn filter_deliverables(all: Vec<Deliverable>, filter: Option<&str>) -> Vec<Deliverable> {
    match filter {
        Some(id) => all.into_iter().filter(|d| d.deliverable_id == id).collect(),
        None => all,
    }
}

fn run_deliverable(
    deliverable: &Deliverable,
    _cwd: &Path,
) -> Result<(DeliverableReport, Vec<CheckEvidence>)> {
    let mut checks = Vec::new();
    let mut evidence_vec = Vec::new();
    let mut all_passed = true;

    for check in &deliverable.checks {
        let ev_start = SystemTime::now();
        let (passed, actual_exit, actual_stdout, error) = execute_check(check)?;
        let elapsed = ev_start.elapsed().unwrap_or_default().as_millis() as u64;

        if !passed {
            all_passed = false;
        }

        checks.push(CheckReport {
            check_id: check.id.clone(),
            passed,
            actual_exit_code: actual_exit,
            actual_stdout: actual_stdout.clone(),
            error: error.clone(),
            execution_time_ms: elapsed,
        });

        evidence_vec.push(CheckEvidence {
            check_id: check.id.clone(),
            kind: check.kind,
            cmd: check.cmd.clone(),
            exit_code: actual_exit,
            stdout: actual_stdout,
            stderr: error.clone(),
            file_exists: if check.kind == CheckKind::FileExists {
                check.path.as_ref().map(|p| Path::new(p).exists())
            } else {
                None
            },
            regex_matched: if check.kind == CheckKind::ShellOutputMatch {
                Some(passed)
            } else {
                None
            },
            duration_ms: elapsed,
            timestamp: aiplus_core::now_iso(),
        });
    }

    let verdict = match deliverable.check_combiner {
        CheckCombiner::AllMustPass => {
            if all_passed {
                AuditorVerdict::Pass
            } else {
                AuditorVerdict::Fail
            }
        }
        CheckCombiner::AnyPass => {
            if checks.iter().any(|c| c.passed) {
                AuditorVerdict::Pass
            } else {
                AuditorVerdict::Fail
            }
        }
    };

    let report = DeliverableReport {
        deliverable_id: deliverable.deliverable_id.clone(),
        verdict,
        checks,
        execution_time_ms: evidence_vec.iter().map(|e| e.duration_ms).sum(),
    };

    Ok((report, evidence_vec))
}

fn execute_check(check: &Check) -> Result<CheckExecution> {
    match check.kind {
        CheckKind::ExitCode => {
            let cmd_str = check.cmd.as_deref().unwrap_or("");
            let output = run_shell(cmd_str, check.timeout_seconds)?;
            let expected = check.expected_exit.unwrap_or(0);
            let passed = output.status.code() == Some(expected);
            let stdout = String::from_utf8_lossy(&output.stdout).to_string();
            let stderr = String::from_utf8_lossy(&output.stderr).to_string();
            let error = if passed { None } else { Some(stderr) };
            Ok((passed, output.status.code(), Some(stdout), error))
        }
        CheckKind::FileExists => {
            let path = check.path.as_deref().unwrap_or("");
            let exists = Path::new(path).exists();
            Ok((exists, None, None, None))
        }
        CheckKind::ShellOutputMatch => {
            let cmd_str = check.cmd.as_deref().unwrap_or("");
            let output = run_shell(cmd_str, check.timeout_seconds)?;
            let stdout = String::from_utf8_lossy(&output.stdout);
            let regex_str = check.expected_regex.as_deref().unwrap_or(".*");
            // v0.1: simplified matching — check if output contains the pattern
            // (stripping anchoring characters for basic substring search)
            let needle = regex_str.trim_start_matches('^').trim_end_matches('$');
            let passed = stdout.contains(needle);
            let stderr = String::from_utf8_lossy(&output.stderr).to_string();
            let error = if passed { None } else { Some(stderr) };
            Ok((
                passed,
                output.status.code(),
                Some(stdout.to_string()),
                error,
            ))
        }
    }
}

fn run_shell(cmd: &str, timeout_seconds: u64) -> Result<std::process::Output> {
    let output = Command::new("sh")
        .arg("-c")
        .arg(cmd)
        .output()
        .with_context(|| format!("failed to execute: {}", cmd))?;
    // v0.1: timeout is advisory; actual OS-level timeout not enforced here.
    let _ = timeout_seconds;
    Ok(output)
}

fn compute_overall_verdict(
    deliverables: &[DeliverableReport],
    blocked: &[BlockedDeliverable],
) -> AuditorVerdict {
    if !blocked.is_empty() {
        return AuditorVerdict::Blocked;
    }
    if deliverables.is_empty() {
        return AuditorVerdict::NeedsFix;
    }
    if deliverables
        .iter()
        .all(|d| d.verdict == AuditorVerdict::Pass)
    {
        AuditorVerdict::Pass
    } else if deliverables
        .iter()
        .any(|d| d.verdict == AuditorVerdict::Fail)
    {
        AuditorVerdict::Fail
    } else {
        AuditorVerdict::NeedsFix
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn schema_override_loads_from_filesystem() {
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("override.yaml");
        fs::write(
            &path,
            r#"schema_version: "0.1.4"
deliverables: []
"#,
        )
        .unwrap();
        let schema =
            load_schema(Some(path.to_str().unwrap())).expect("override schema should load");
        assert_eq!(schema.schema_version, "0.1.4");
        assert!(schema.deliverables.is_empty());
    }
}
