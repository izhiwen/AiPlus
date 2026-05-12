use std::fs;
use std::path::Path;

use anyhow::{anyhow, Context, Result};

use aiplus_core::agent_team::types::{AuditRun, AuditorVerdict, DeliverableReport};

const AUDIT_RUNS_PATH: &str = ".aiplus/agent-team/audit-trail/audit-runs.jsonl";

/// Entry point for `audit replay <run_id>`.
pub fn handle_replay(run_id: &str) -> Result<()> {
    let cwd = std::env::current_dir().context("failed to get current directory")?;
    let audit_runs_path = cwd.join(AUDIT_RUNS_PATH);

    // Load historical audit run
    let baseline = find_audit_run(&audit_runs_path, run_id)?
        .ok_or_else(|| anyhow!("audit run {} not found", run_id))?;

    println!("Replaying audit run: {}", run_id);
    println!("Started at: {}", baseline.started_at);

    // Re-run checks for each deliverable
    let mut replay_reports: Vec<DeliverableReport> = Vec::new();
    for deliverable in &baseline.deliverables {
        let replay_report = rerun_deliverable_checks(deliverable)?;
        replay_reports.push(replay_report);
    }

    // Build a minimal replay report for drift comparison
    let replay_report = aiplus_core::agent_team::types::AuditReport {
        schema_version: "0.1.4".to_string(),
        audit_run_id: format!("replay-{}", run_id),
        started_at: aiplus_core::now_iso(),
        completed_at: aiplus_core::now_iso(),
        overall_verdict: compute_overall_verdict(&replay_reports),
        deliverables: replay_reports.clone(),
        blocked_deliverables: baseline.blocked_deliverables.clone(),
        metrics: aiplus_core::agent_team::types::AuditMetrics {
            total_checks: replay_reports.iter().map(|d| d.checks.len() as u32).sum(),
            passed_checks: replay_reports
                .iter()
                .flat_map(|d| &d.checks)
                .filter(|c| c.passed)
                .count() as u32,
            failed_checks: replay_reports
                .iter()
                .flat_map(|d| &d.checks)
                .filter(|c| !c.passed)
                .count() as u32,
            blocked_checks: baseline.blocked_deliverables.len() as u32,
            total_execution_time_ms: replay_reports.iter().map(|d| d.execution_time_ms).sum(),
            canary_dropped_this_run: 0,
        },
        owner_feedback_prompt: String::new(),
    };

    let baseline_report = aiplus_core::agent_team::types::AuditReport {
        schema_version: "0.1.4".to_string(),
        audit_run_id: baseline.run_id.clone(),
        started_at: baseline.started_at.clone(),
        completed_at: baseline.completed_at.clone(),
        overall_verdict: compute_overall_verdict(&baseline.deliverables),
        deliverables: baseline.deliverables.clone(),
        blocked_deliverables: baseline.blocked_deliverables.clone(),
        metrics: aiplus_core::agent_team::types::AuditMetrics {
            total_checks: baseline.deliverables.iter().map(|d| d.checks.len() as u32).sum(),
            passed_checks: baseline
                .deliverables
                .iter()
                .flat_map(|d| &d.checks)
                .filter(|c| c.passed)
                .count() as u32,
            failed_checks: baseline
                .deliverables
                .iter()
                .flat_map(|d| &d.checks)
                .filter(|c| !c.passed)
                .count() as u32,
            blocked_checks: baseline.blocked_deliverables.len() as u32,
            total_execution_time_ms: baseline.deliverables.iter().map(|d| d.execution_time_ms).sum(),
            canary_dropped_this_run: 0,
        },
        owner_feedback_prompt: String::new(),
    };

    let has_drift = super::canary::detect_drift(&baseline_report,
        &replay_report,
    );

    if has_drift {
        println!("DRIFT_DETECTED: true");
        // Log drift finding
        let drift_path = cwd.join(".aiplus/agent-team/audit-trail/drift-findings.jsonl");
        let finding = aiplus_core::auditor::drift::DriftFinding {
            drift_type: aiplus_core::auditor::drift::DriftType::ReproducibilityDrift,
            deliverable_id: replay_reports
                .first()
                .map(|d| d.deliverable_id.clone())
                .unwrap_or_default(),
            run_id: baseline.run_id.clone(),
            baseline_run_id: Some(baseline.run_id.clone()),
            message: format!("Replay of {} produced different results", run_id),
            priority: "HIGH".to_string(),
        };
        let line = serde_json::to_string(&finding).context("failed to serialize drift finding")?;
        aiplus_core::append_jsonl_atomic(
            &drift_path, &line)
            .with_context(|| "failed to write drift-findings.jsonl")?;
    } else {
        println!("DRIFT_DETECTED: false");
    }

    let yaml = serde_yaml_ng::to_string(&replay_report).context("failed to serialize replay report")?;
    println!("{yaml}");

    Ok(())
}

fn find_audit_run(path: &Path, run_id: &str) -> Result<Option<AuditRun>> {
    if !path.exists() {
        return Ok(None);
    }
    let content = fs::read_to_string(path)
        .with_context(|| format!("failed to read {}", path.display()))?;
    for line in content.lines().filter(|l| !l.trim().is_empty()) {
        let run: AuditRun = serde_json::from_str(line)
            .with_context(|| format!("failed to parse audit run line: {line}"))?;
        if run.run_id == run_id {
            return Ok(Some(run));
        }
    }
    Ok(None)
}

fn rerun_deliverable_checks(original: &DeliverableReport) -> Result<DeliverableReport> {
    // v0.1: We do not have the original Check definitions, only reports.
    // For replay we re-execute the persisted audit script if available,
    // otherwise return a stub that mirrors the original for drift comparison.
    let script_path = format!(
        ".aiplus/agent-team/audit-scripts/{}.sh",
        original.deliverable_id
    );
    let mut checks = original.checks.clone();

    if std::path::Path::new(&script_path).exists() {
        let output = std::process::Command::new("sh")
            .arg(&script_path)
            .output()
            .with_context(|| format!("failed to run audit script {}", script_path))?;
        let passed = output.status.success();
        // Update all checks to the replay result (simplified)
        for check in &mut checks {
            check.passed = passed;
        }
    }

    Ok(DeliverableReport {
        deliverable_id: original.deliverable_id.clone(),
        verdict: original.verdict,
        checks,
        execution_time_ms: original.execution_time_ms,
    })
}

fn compute_overall_verdict(deliverables: &[DeliverableReport]) -> AuditorVerdict {
    if deliverables.is_empty() {
        return AuditorVerdict::NeedsFix;
    }
    if deliverables.iter().all(|d| d.verdict == AuditorVerdict::Pass) {
        AuditorVerdict::Pass
    } else if deliverables.iter().any(|d| d.verdict == AuditorVerdict::Fail) {
        AuditorVerdict::Fail
    } else {
        AuditorVerdict::NeedsFix
    }
}
