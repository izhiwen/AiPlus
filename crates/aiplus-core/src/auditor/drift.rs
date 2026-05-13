use std::collections::HashMap;
use std::fs::{create_dir_all, OpenOptions};
use std::io::Write;
use std::path::Path;

use serde::{Deserialize, Serialize};

use crate::agent_team::types::{AuditorVerdict, Tier};

/// A single audit run for a deliverable.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditRun {
    pub run_id: String,
    pub deliverable_id: String,
    pub tier: Tier,
    pub verdict: AuditorVerdict,
    /// Hash of the inputs used for this audit run.
    pub inputs_hash: String,
    /// ISO8601 timestamp.
    pub timestamp: String,
}

/// Thresholds for count-by-tier drift detection.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DriftThresholds {
    pub heavy_max: usize,
    pub medium_max: usize,
    pub light_max: usize,
    pub stop_gate_max: usize,
}

impl Default for DriftThresholds {
    fn default() -> Self {
        Self {
            heavy_max: 20,
            medium_max: 10,
            light_max: 5,
            stop_gate_max: 50,
        }
    }
}

/// Types of drift that can be detected.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DriftType {
    /// Same inputs produced a different verdict compared to baseline.
    ReproducibilityDrift,
    /// Audit count for this deliverable+tier exceeds the configured threshold.
    ThresholdExceeded,
    /// Baseline is older than the maximum allowed age.
    BaselineStale,
}

/// A single drift finding.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DriftFinding {
    pub drift_type: DriftType,
    pub deliverable_id: String,
    pub run_id: String,
    pub baseline_run_id: Option<String>,
    pub message: String,
    pub priority: String,
}

/// Owner spot-check queue entry.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpotCheckEntry {
    pub audit_run_id: String,
    pub deliverable_id: String,
    pub auditor_verdict: AuditorVerdict,
    pub owner_verdict: Option<AuditorVerdict>,
    pub note: Option<String>,
    pub retracted: bool,
    pub timestamp: String,
}

/// Detects when audit results drift from reproducible baselines.
pub struct DriftDetector {
    thresholds: DriftThresholds,
    history: Vec<AuditRun>,
    /// Count-by-tier: deliverable_id -> Tier -> count
    audit_counts: HashMap<String, HashMap<Tier, usize>>,
    /// Maximum age in days before a baseline is considered stale.
    baseline_max_age_days: u64,
}

impl DriftDetector {
    pub fn new(thresholds: DriftThresholds) -> Self {
        Self {
            thresholds,
            history: Vec::new(),
            audit_counts: HashMap::new(),
            baseline_max_age_days: 30,
        }
    }

    /// Create with custom baseline max age (in days).
    pub fn with_baseline_max_age(mut self, days: u64) -> Self {
        self.baseline_max_age_days = days;
        self
    }

    /// Detect drift for a new audit run.
    pub fn detect_drift(&self, new_run: &AuditRun) -> Vec<DriftFinding> {
        let mut findings = Vec::new();

        // Find baseline for same deliverable with same inputs
        let baseline = self
            .history
            .iter()
            .filter(|run| {
                run.deliverable_id == new_run.deliverable_id
                    && run.inputs_hash == new_run.inputs_hash
            })
            .last();

        if let Some(base) = baseline {
            if base.verdict != new_run.verdict {
                findings.push(DriftFinding {
                    drift_type: DriftType::ReproducibilityDrift,
                    deliverable_id: new_run.deliverable_id.clone(),
                    run_id: new_run.run_id.clone(),
                    baseline_run_id: Some(base.run_id.clone()),
                    message: format!(
                        "Verdict changed from {:?} to {:?} for same inputs (baseline run {})",
                        base.verdict, new_run.verdict, base.run_id
                    ),
                    priority: "HIGH".to_string(),
                });
            }

            // Check baseline staleness
            if Self::days_between(&base.timestamp, &new_run.timestamp) > self.baseline_max_age_days
            {
                findings.push(DriftFinding {
                    drift_type: DriftType::BaselineStale,
                    deliverable_id: new_run.deliverable_id.clone(),
                    run_id: new_run.run_id.clone(),
                    baseline_run_id: Some(base.run_id.clone()),
                    message: format!(
                        "Baseline run {} is older than {} days",
                        base.run_id, self.baseline_max_age_days
                    ),
                    priority: "HIGH".to_string(),
                });
            }
        }

        // Check threshold
        let tier = new_run.tier;
        let count = self
            .audit_counts
            .get(&new_run.deliverable_id)
            .and_then(|m| m.get(&tier))
            .copied()
            .unwrap_or(0);

        let max = match tier {
            Tier::Heavy => self.thresholds.heavy_max,
            Tier::Medium => self.thresholds.medium_max,
            Tier::Light => self.thresholds.light_max,
            Tier::StopGate => self.thresholds.stop_gate_max,
        };

        if count >= max {
            findings.push(DriftFinding {
                drift_type: DriftType::ThresholdExceeded,
                deliverable_id: new_run.deliverable_id.clone(),
                run_id: new_run.run_id.clone(),
                baseline_run_id: baseline.map(|b| b.run_id.clone()),
                message: format!(
                    "Audit count ({}) exceeds {:?} threshold ({})",
                    count, tier, max
                ),
                priority: "HIGH".to_string(),
            });
        }

        findings
    }

    /// Record an audit run, updating count-by-tier tracking.
    pub fn record_run(&mut self, run: AuditRun) {
        let tier = run.tier;
        let entry = self
            .audit_counts
            .entry(run.deliverable_id.clone())
            .or_default();
        *entry.entry(tier).or_insert(0) += 1;
        self.history.push(run);
    }

    /// Log drift findings to `audit-trail/drift-findings.jsonl`.
    pub fn log_findings(
        &self,
        findings: &[DriftFinding],
        output_dir: &Path,
    ) -> std::io::Result<()> {
        if findings.is_empty() {
            return Ok(());
        }
        create_dir_all(output_dir)?;
        let path = output_dir.join("drift-findings.jsonl");
        let mut file = OpenOptions::new().create(true).append(true).open(&path)?;
        for finding in findings {
            let line = serde_json::to_string(finding)
                .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
            writeln!(file, "{}", line)?;
        }
        Ok(())
    }

    /// Enqueue drift findings to the owner spot-check queue.
    pub fn enqueue_findings(&self, findings: &[DriftFinding]) -> Vec<SpotCheckEntry> {
        findings
            .iter()
            .map(|finding| SpotCheckEntry {
                audit_run_id: finding.run_id.clone(),
                deliverable_id: finding.deliverable_id.clone(),
                auditor_verdict: AuditorVerdict::Fail,
                owner_verdict: None,
                note: Some(finding.message.clone()),
                retracted: false,
                timestamp: finding.run_id.clone(), // simplified; real code would use chrono
            })
            .collect()
    }

    /// Surface drift findings in an audit report summary.
    pub fn surface_findings(&self, findings: &[DriftFinding]) -> Vec<String> {
        findings
            .iter()
            .map(|f| {
                format!(
                    "[{}] {} for deliverable {}: {}",
                    f.priority,
                    format!("{:?}", f.drift_type),
                    f.deliverable_id,
                    f.message
                )
            })
            .collect()
    }

    /// Get current count for a deliverable+tier.
    pub fn count_for(&self, deliverable_id: &str, tier: Tier) -> usize {
        self.audit_counts
            .get(deliverable_id)
            .and_then(|m| m.get(&tier))
            .copied()
            .unwrap_or(0)
    }

    /// Parse ISO8601 timestamps and compute days difference.
    /// Simplified: assumes YYYY-MM-DDTHH:MM:SS format or similar.
    fn days_between(baseline: &str, current: &str) -> u64 {
        // Parse only the date portion (first 10 chars: YYYY-MM-DD)
        let parse_date = |s: &str| {
            let date_str = &s[..s.len().min(10)];
            if date_str.len() < 10 {
                return None;
            }
            let year = date_str[0..4].parse::<i32>().ok()?;
            let month = date_str[5..7].parse::<u32>().ok()?;
            let day = date_str[8..10].parse::<u32>().ok()?;
            Some((year, month, day))
        };

        let base = parse_date(baseline);
        let curr = parse_date(current);

        match (base, curr) {
            (Some((by, bm, bd)), Some((cy, cm, cd))) => {
                let base_days = by as i64 * 365 + bm as i64 * 30 + bd as i64;
                let curr_days = cy as i64 * 365 + cm as i64 * 30 + cd as i64;
                (curr_days - base_days).max(0) as u64
            }
            _ => 0,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_run(id: &str, did: &str, tier: Tier, verdict: AuditorVerdict, hash: &str) -> AuditRun {
        AuditRun {
            run_id: id.to_string(),
            deliverable_id: did.to_string(),
            tier,
            verdict,
            inputs_hash: hash.to_string(),
            timestamp: "2024-01-01T00:00:00Z".to_string(),
        }
    }

    #[test]
    fn test_default_thresholds() {
        let dt = DriftThresholds::default();
        assert_eq!(dt.heavy_max, 20);
        assert_eq!(dt.medium_max, 10);
        assert_eq!(dt.light_max, 5);
        assert_eq!(dt.stop_gate_max, 50);
    }

    #[test]
    fn test_reproducibility_drift() {
        let thresholds = DriftThresholds::default();
        let mut detector = DriftDetector::new(thresholds);

        let baseline = make_run("run-1", "del-a", Tier::Light, AuditorVerdict::Pass, "hash1");
        detector.record_run(baseline);

        let new_run = make_run("run-2", "del-a", Tier::Light, AuditorVerdict::Fail, "hash1");
        let findings = detector.detect_drift(&new_run);

        assert_eq!(findings.len(), 1);
        assert_eq!(findings[0].drift_type, DriftType::ReproducibilityDrift);
        assert!(findings[0].message.contains("Verdict changed"));
    }

    #[test]
    fn test_no_drift_when_verdict_same() {
        let thresholds = DriftThresholds::default();
        let mut detector = DriftDetector::new(thresholds);

        let baseline = make_run("run-1", "del-a", Tier::Light, AuditorVerdict::Pass, "hash1");
        detector.record_run(baseline);

        let new_run = make_run("run-2", "del-a", Tier::Light, AuditorVerdict::Pass, "hash1");
        let findings = detector.detect_drift(&new_run);

        // Only threshold check — count is 1, light_max is 5, so no drift
        assert!(findings.is_empty());
    }

    #[test]
    fn test_threshold_exceeded() {
        let thresholds = DriftThresholds {
            light_max: 2,
            ..DriftThresholds::default()
        };
        let mut detector = DriftDetector::new(thresholds);

        detector.record_run(make_run(
            "r1",
            "del-a",
            Tier::Light,
            AuditorVerdict::Pass,
            "h1",
        ));
        detector.record_run(make_run(
            "r2",
            "del-a",
            Tier::Light,
            AuditorVerdict::Pass,
            "h2",
        ));

        let new_run = make_run("r3", "del-a", Tier::Light, AuditorVerdict::Pass, "h3");
        let findings = detector.detect_drift(&new_run);

        assert_eq!(findings.len(), 1);
        assert_eq!(findings[0].drift_type, DriftType::ThresholdExceeded);
        assert!(findings[0].message.contains("Audit count (2)"));
    }

    #[test]
    fn test_baseline_stale() {
        let thresholds = DriftThresholds::default();
        let _detector = DriftDetector::new(thresholds).with_baseline_max_age(7);

        let mut baseline = make_run("run-1", "del-a", Tier::Light, AuditorVerdict::Pass, "hash1");
        baseline.timestamp = "2024-01-01T00:00:00Z".to_string();

        let mut new_run = make_run("run-2", "del-a", Tier::Light, AuditorVerdict::Pass, "hash1");
        new_run.timestamp = "2024-01-15T00:00:00Z".to_string();

        // Need to use a detector with history; detect_drift is &self so we can't use record_run
        // For this test we construct detector with pre-populated history manually
        let detector = DriftDetector {
            thresholds: DriftThresholds::default(),
            history: vec![baseline],
            audit_counts: HashMap::new(),
            baseline_max_age_days: 7,
        };

        let findings = detector.detect_drift(&new_run);
        assert_eq!(findings.len(), 1);
        assert_eq!(findings[0].drift_type, DriftType::BaselineStale);
    }

    #[test]
    fn test_count_by_tier_per_deliverable() {
        let thresholds = DriftThresholds::default();
        let mut detector = DriftDetector::new(thresholds);

        detector.record_run(make_run(
            "r1",
            "del-a",
            Tier::Light,
            AuditorVerdict::Pass,
            "h1",
        ));
        detector.record_run(make_run(
            "r2",
            "del-a",
            Tier::Medium,
            AuditorVerdict::Pass,
            "h2",
        ));
        detector.record_run(make_run(
            "r3",
            "del-b",
            Tier::Light,
            AuditorVerdict::Pass,
            "h3",
        ));

        assert_eq!(detector.count_for("del-a", Tier::Light), 1);
        assert_eq!(detector.count_for("del-a", Tier::Medium), 1);
        assert_eq!(detector.count_for("del-a", Tier::Heavy), 0);
        assert_eq!(detector.count_for("del-b", Tier::Light), 1);
    }

    #[test]
    fn test_log_findings_creates_jsonl() {
        let temp_dir = std::env::temp_dir().join("aiplus_drift_test");
        let _ = std::fs::remove_dir_all(&temp_dir);

        let thresholds = DriftThresholds::default();
        let detector = DriftDetector::new(thresholds);

        let findings = vec![DriftFinding {
            drift_type: DriftType::ReproducibilityDrift,
            deliverable_id: "del-a".to_string(),
            run_id: "run-1".to_string(),
            baseline_run_id: Some("run-0".to_string()),
            message: "Test drift".to_string(),
            priority: "HIGH".to_string(),
        }];

        detector.log_findings(&findings, &temp_dir).unwrap();

        let path = temp_dir.join("drift-findings.jsonl");
        assert!(path.exists());

        let content = std::fs::read_to_string(&path).unwrap();
        assert!(content.contains("reproducibility_drift"));
        assert!(content.contains("del-a"));

        let _ = std::fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_enqueue_findings() {
        let thresholds = DriftThresholds::default();
        let detector = DriftDetector::new(thresholds);

        let findings = vec![DriftFinding {
            drift_type: DriftType::ThresholdExceeded,
            deliverable_id: "del-a".to_string(),
            run_id: "run-1".to_string(),
            baseline_run_id: None,
            message: "Too many audits".to_string(),
            priority: "HIGH".to_string(),
        }];

        let entries = detector.enqueue_findings(&findings);
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].deliverable_id, "del-a");
        assert_eq!(entries[0].auditor_verdict, AuditorVerdict::Fail);
        assert!(!entries[0].retracted);
    }

    #[test]
    fn test_surface_findings() {
        let thresholds = DriftThresholds::default();
        let detector = DriftDetector::new(thresholds);

        let findings = vec![DriftFinding {
            drift_type: DriftType::ReproducibilityDrift,
            deliverable_id: "del-a".to_string(),
            run_id: "run-1".to_string(),
            baseline_run_id: Some("run-0".to_string()),
            message: "Verdict mismatch".to_string(),
            priority: "HIGH".to_string(),
        }];

        let lines = detector.surface_findings(&findings);
        assert_eq!(lines.len(), 1);
        assert!(lines[0].contains("HIGH"));
        assert!(lines[0].contains("del-a"));
    }
}
