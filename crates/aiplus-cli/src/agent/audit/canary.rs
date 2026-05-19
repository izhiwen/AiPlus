// TODO(v0.2): canary replay scaffolding. Wired into audit pipeline once
// `aiplus agent audit canary` subcommand lands. Until then the entire
// module is dead — kept as scaffolding to preserve the design intent
// rather than rewriting it from scratch later. Suppress dead_code
// warnings for the whole module so we don't pollute every `cargo build`.
#![allow(dead_code)]

use std::path::Path;

use anyhow::{Context, Result};

use aiplus_core::agent_team::{AuditReport, AuditRun, CanaryState, Deliverable, Tier};

/// Result of a single canary replay item.
#[derive(Debug, Clone)]
pub struct CanaryReplayItem {
    pub deliverable_id: String,
    pub run_id: String,
    pub drift_detected: bool,
    pub original_verdict: String,
    pub replay_verdict: String,
}

/// Check whether canary replay should trigger.
///
/// Returns `Ok(true)` if canary should run, `Ok(false)` if not yet,
/// or `Err(msg)` if audits are stale (surfacing the required message).
pub fn should_trigger_canary(
    audit_history: &[AuditRun],
    canary_state: &CanaryState,
    now_iso: &str,
) -> Result<bool, String> {
    // Freshness check: last completed audit must be ≤ 14 days old
    if let Some(last_audit) = audit_history.iter().max_by_key(|r| &r.completed_at) {
        let days = days_between(&last_audit.completed_at, now_iso).unwrap_or(0);
        if days > 14 {
            return Err(format!(
                "It has been {} days since last full audit. Canary replay requires recent audit baselines (max 14 days old). Please run: aiplus agent audit run before canary can resume.",
                days
            ));
        }
    } else {
        return Err(
            "It has been N days since last full audit. Canary replay requires recent audit baselines (max 14 days old). Please run: aiplus agent audit run before canary can resume."
                .to_string(),
        );
    }

    // Count-based cadence: every 7 audit runs
    if canary_state.audit_run_count > 0 && canary_state.audit_run_count.is_multiple_of(7) {
        return Ok(true);
    }

    // Monthly fallback calendar cadence
    if let Some(last_trigger) = &canary_state.last_canary_trigger {
        if is_different_month(last_trigger, now_iso) {
            return Ok(true);
        }
    } else {
        // Never triggered before
        return Ok(true);
    }

    Ok(false)
}

/// Select a risk-weighted canary sample from the deliverable pool.
///
/// - `stop_gate_touched` deliverables are always included.
/// - HEAVY deliverables have weight 3 (included greedily after stop_gate).
/// - MEDIUM deliverables have weight 2 (included greedily after HEAVY).
/// - LIGHT deliverables use round-robin with 180-day target coverage.
/// - Sample is capped at 8 and floored at 3 (best-effort).
pub fn select_canary_sample(
    deliverables: &[Deliverable],
    audit_history: &[AuditRun],
    _canary_state: &CanaryState,
) -> Vec<Deliverable> {
    // Partition by priority
    let mut stop_gate: Vec<&Deliverable> = Vec::new();
    let mut heavy: Vec<&Deliverable> = Vec::new();
    let mut medium: Vec<&Deliverable> = Vec::new();
    let mut light: Vec<&Deliverable> = Vec::new();

    for d in deliverables {
        if !d.related_stop_gates.is_empty() {
            stop_gate.push(d);
            continue;
        }
        match d.tier {
            Tier::Heavy => heavy.push(d),
            Tier::Medium => medium.push(d),
            Tier::Light => light.push(d),
            Tier::StopGate => stop_gate.push(d),
        }
    }

    // Round-robin for LIGHT: sort by last audit date (oldest first)
    light.sort_by_key(|d| {
        audit_history
            .iter()
            .filter(|r| {
                r.deliverables
                    .iter()
                    .any(|dr| dr.deliverable_id == d.deliverable_id)
                    || r.blocked_deliverables
                        .iter()
                        .any(|b| b.deliverable_id == d.deliverable_id)
            })
            .map(|r| &r.completed_at)
            .max()
            .cloned()
            .unwrap_or_default()
    });

    let mut sample: Vec<Deliverable> = Vec::new();
    const CAP: usize = 8;

    // Priority 1: stop_gate_touched (always include)
    for d in stop_gate {
        if sample.len() < CAP {
            sample.push(d.clone());
        }
    }

    // Priority 2: HEAVY
    for d in heavy {
        if sample.len() < CAP {
            sample.push(d.clone());
        }
    }

    // Priority 3: MEDIUM
    for d in medium {
        if sample.len() < CAP {
            sample.push(d.clone());
        }
    }

    // Priority 4: LIGHT (round-robin)
    for d in light {
        if sample.len() < CAP {
            sample.push(d.clone());
        }
    }

    sample
}

/// Compute how many deliverables were dropped due to the sample size cap.
pub fn compute_dropped_count(deliverables: &[Deliverable], sample: &[Deliverable]) -> u32 {
    deliverables.len().saturating_sub(sample.len()) as u32
}

/// Update canary state after an audit/canary run.
pub fn update_canary_state(state: &mut CanaryState, triggered: bool, dropped: u32, now_iso: &str) {
    if triggered {
        state.last_canary_trigger = Some(now_iso.to_string());
    }
    state.canary_dropped_this_run = dropped;
    if dropped > 0 {
        state.consecutive_drop_runs += 1;
    } else {
        state.consecutive_drop_runs = 0;
    }
}

/// Read canary state from a JSONL file.
///
/// If the file does not exist or is empty, returns the default state.
pub fn read_canary_state(path: &Path) -> Result<CanaryState> {
    if !path.exists() {
        return Ok(CanaryState::default());
    }
    let content = std::fs::read_to_string(path)
        .with_context(|| format!("failed to read canary state from {}", path.display()))?;
    let mut state = CanaryState::default();
    for line in content.lines().filter(|l| !l.trim().is_empty()) {
        state = serde_json::from_str(line)
            .with_context(|| format!("failed to parse canary state line: {line}"))?;
    }
    Ok(state)
}

/// Write canary state to a JSONL file (append mode).
pub fn write_canary_state(path: &Path, state: &CanaryState) -> Result<()> {
    let line = serde_json::to_string(state).context("failed to serialize canary state")?;
    std::fs::create_dir_all(path.parent().unwrap_or_else(|| Path::new(".")))?;
    let mut file = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)
        .with_context(|| format!("failed to open canary state file {}", path.display()))?;
    use std::io::Write;
    writeln!(file, "{line}")
        .with_context(|| format!("failed to write canary state to {}", path.display()))?;
    Ok(())
}

/// Detect drift between an original audit report and its replay.
///
/// Returns `true` if any deliverable verdict changed, deliverables are missing,
/// or blocked deliverables differ.
pub fn detect_drift(original: &AuditReport, replay: &AuditReport) -> bool {
    // Compare deliverable verdicts
    for orig in &original.deliverables {
        match replay
            .deliverables
            .iter()
            .find(|r| r.deliverable_id == orig.deliverable_id)
        {
            Some(rep) => {
                if orig.verdict != rep.verdict {
                    return true;
                }
            }
            None => return true,
        }
    }

    // Check for extra deliverables in replay
    for rep in &replay.deliverables {
        if !original
            .deliverables
            .iter()
            .any(|o| o.deliverable_id == rep.deliverable_id)
        {
            return true;
        }
    }

    // Compare blocked deliverables
    if original.blocked_deliverables.len() != replay.blocked_deliverables.len() {
        return true;
    }
    for orig in &original.blocked_deliverables {
        if !replay
            .blocked_deliverables
            .iter()
            .any(|b| b.deliverable_id == orig.deliverable_id && b.reason == orig.reason)
        {
            return true;
        }
    }

    false
}

/// Run canary replay for each deliverable in the sample.
///
/// Finds the most recent audit run containing the deliverable and records
/// the intent to replay. In v0.1 the actual command is stubbed.
pub fn run_canary_replays(
    sample: &[Deliverable],
    audit_history: &[AuditRun],
) -> Result<Vec<CanaryReplayItem>> {
    let mut results = Vec::new();

    for deliverable in sample {
        let Some(last_run) =
            find_last_audit_for_deliverable(&deliverable.deliverable_id, audit_history)
        else {
            continue;
        };

        // v0.1: record intent. Full execution would be:
        // aiplus agent audit replay <audit_run_id>
        let _ = std::process::Command::new("aiplus")
            .args(["agent", "audit", "replay", &last_run.run_id])
            .output();

        results.push(CanaryReplayItem {
            deliverable_id: deliverable.deliverable_id.clone(),
            run_id: last_run.run_id.clone(),
            drift_detected: false,
            original_verdict: "Pass".to_string(),
            replay_verdict: "Pass".to_string(),
        });
    }

    Ok(results)
}

/// Find the most recent audit run that contains a given deliverable.
fn find_last_audit_for_deliverable<'a>(
    deliverable_id: &str,
    audit_history: &'a [AuditRun],
) -> Option<&'a AuditRun> {
    audit_history
        .iter()
        .filter(|r| {
            r.deliverables
                .iter()
                .any(|d| d.deliverable_id == deliverable_id)
                || r.blocked_deliverables
                    .iter()
                    .any(|b| b.deliverable_id == deliverable_id)
        })
        .max_by_key(|r| &r.completed_at)
}

// ------------------------------------------------------------------
// Minimal ISO8601 date helpers (no extra deps)
// ------------------------------------------------------------------

fn parse_iso_date(iso: &str) -> Option<(i32, u32, u32)> {
    let date_part = iso.split('T').next()?;
    let parts: Vec<&str> = date_part.split('-').collect();
    if parts.len() != 3 {
        return None;
    }
    let year = parts[0].parse().ok()?;
    let month = parts[1].parse().ok()?;
    let day = parts[2].parse().ok()?;
    Some((year, month, day))
}

fn is_leap_year(year: i32) -> bool {
    (year % 4 == 0 && year % 100 != 0) || (year % 400 == 0)
}

fn days_in_month(year: i32, month: u32) -> u32 {
    match month {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        2 => {
            if is_leap_year(year) {
                29
            } else {
                28
            }
        }
        _ => 30,
    }
}

fn day_of_year(year: i32, month: u32, day: u32) -> u32 {
    let mut doy = day;
    for m in 1..month {
        doy += days_in_month(year, m);
    }
    doy
}

fn days_between(start_iso: &str, end_iso: &str) -> Option<u64> {
    let (y1, m1, d1) = parse_iso_date(start_iso)?;
    let (y2, m2, d2) = parse_iso_date(end_iso)?;

    let start_doy = day_of_year(y1, m1, d1) as i64;
    let end_doy = day_of_year(y2, m2, d2) as i64;

    let mut days = end_doy - start_doy;
    for y in y1..y2 {
        days += if is_leap_year(y) { 366 } else { 365 };
    }
    for y in y2..y1 {
        days -= if is_leap_year(y) { 366 } else { 365 };
    }

    Some(days.max(0) as u64)
}

fn is_different_month(iso_a: &str, iso_b: &str) -> bool {
    let a = iso_a.split('T').next().unwrap_or(iso_a);
    let b = iso_b.split('T').next().unwrap_or(iso_b);
    let a_prefix = a.split('-').take(2).collect::<Vec<_>>().join("-");
    let b_prefix = b.split('-').take(2).collect::<Vec<_>>().join("-");
    a_prefix != b_prefix
}

#[cfg(test)]
mod tests {
    use super::*;
    use aiplus_core::agent_team::{
        AcceptanceMode, AuditorVerdict, BlockedDeliverable, BlockedReason, Check, CheckCombiner,
        CheckKind, DeliverableReport,
    };

    fn make_deliverable(id: &str, tier: Tier, stop_gates: Vec<&str>) -> Deliverable {
        Deliverable {
            deliverable_id: id.to_string(),
            description: format!("Test {id}"),
            acceptance_mode: AcceptanceMode::Deterministic,
            tier,
            check_combiner: CheckCombiner::AllMustPass,
            checks: vec![Check {
                id: "c1".to_string(),
                kind: CheckKind::ExitCode,
                cmd: Some("true".to_string()),
                expected_exit: Some(0),
                path: None,
                expected_regex: None,
                timeout_seconds: 30,
            }],
            persisted_audit_script: "test.sh".to_string(),
            self_test_script: "test_self.sh".to_string(),
            related_stop_gates: stop_gates.into_iter().map(String::from).collect(),
            owner_review_required: false,
        }
    }

    fn make_audit_run(run_id: &str, completed_at: &str, deliverable_ids: Vec<&str>) -> AuditRun {
        AuditRun {
            run_id: run_id.to_string(),
            started_at: completed_at.to_string(),
            completed_at: completed_at.to_string(),
            deliverables: deliverable_ids
                .into_iter()
                .map(|id| DeliverableReport {
                    deliverable_id: id.to_string(),
                    verdict: AuditorVerdict::Pass,
                    checks: vec![],
                    execution_time_ms: 0,
                })
                .collect(),
            blocked_deliverables: vec![],
        }
    }

    // ================================================================
    // Cadence tests
    // ================================================================

    #[test]
    fn test_should_trigger_every_7_runs() {
        let history = vec![make_audit_run("r1", "2024-01-01T00:00:00Z", vec!["d1"])];
        let state = CanaryState {
            audit_run_count: 7,
            ..CanaryState::default()
        };
        assert_eq!(
            should_trigger_canary(&history, &state, "2024-01-07T00:00:00Z"),
            Ok(true)
        );
    }

    #[test]
    fn test_should_not_trigger_before_7_runs() {
        let history = vec![make_audit_run("r1", "2024-01-01T00:00:00Z", vec!["d1"])];
        let state = CanaryState {
            audit_run_count: 5,
            last_canary_trigger: Some("2024-01-01T00:00:00Z".to_string()),
            ..CanaryState::default()
        };
        assert_eq!(
            should_trigger_canary(&history, &state, "2024-01-07T00:00:00Z"),
            Ok(false)
        );
    }

    #[test]
    fn test_monthly_fallback_trigger() {
        // Use a recent audit so freshness check passes
        let history = vec![make_audit_run("r1", "2024-01-20T00:00:00Z", vec!["d1"])];
        let state = CanaryState {
            audit_run_count: 1,
            last_canary_trigger: Some("2024-01-01T00:00:00Z".to_string()),
            ..CanaryState::default()
        };
        // Same month => no trigger
        assert_eq!(
            should_trigger_canary(&history, &state, "2024-01-25T00:00:00Z"),
            Ok(false)
        );
        // Different month => trigger (still fresh: 20 Jan -> 1 Feb = 12 days)
        assert_eq!(
            should_trigger_canary(&history, &state, "2024-02-01T00:00:00Z"),
            Ok(true)
        );
    }

    #[test]
    fn test_first_trigger_when_never_triggered() {
        let history = vec![make_audit_run("r1", "2024-01-01T00:00:00Z", vec!["d1"])];
        let state = CanaryState::default();
        assert_eq!(
            should_trigger_canary(&history, &state, "2024-01-01T00:00:00Z"),
            Ok(true)
        );
    }

    #[test]
    fn test_freshness_stale_surfaces_message() {
        let history = vec![make_audit_run("r1", "2024-01-01T00:00:00Z", vec!["d1"])];
        let state = CanaryState::default();
        let result = should_trigger_canary(&history, &state, "2024-01-20T00:00:00Z");
        assert!(result.is_err());
        let msg = result.unwrap_err();
        assert!(msg.contains("It has been 19 days since last full audit"));
        assert!(msg.contains("max 14 days old"));
    }

    #[test]
    fn test_freshness_fresh_allows_trigger() {
        let history = vec![make_audit_run("r1", "2024-01-10T00:00:00Z", vec!["d1"])];
        let state = CanaryState {
            audit_run_count: 7,
            ..CanaryState::default()
        };
        assert_eq!(
            should_trigger_canary(&history, &state, "2024-01-20T00:00:00Z"),
            Ok(true)
        );
    }

    #[test]
    fn test_no_history_stale_message() {
        let history: Vec<AuditRun> = vec![];
        let state = CanaryState::default();
        let result = should_trigger_canary(&history, &state, "2024-01-01T00:00:00Z");
        assert!(result.is_err());
        let msg = result.unwrap_err();
        assert!(msg.contains("It has been N days since last full audit"));
    }

    // ================================================================
    // Sample selection tests
    // ================================================================

    #[test]
    fn test_stop_gate_always_included() {
        let deliverables = vec![
            make_deliverable("d1", Tier::Light, vec![]),
            make_deliverable("d2", Tier::Heavy, vec!["gate1"]),
        ];
        let history = vec![];
        let state = CanaryState::default();
        let sample = select_canary_sample(&deliverables, &history, &state);
        assert!(sample.iter().any(|d| d.deliverable_id == "d2"));
    }

    #[test]
    fn test_priority_order() {
        let deliverables = vec![
            make_deliverable("light1", Tier::Light, vec![]),
            make_deliverable("medium1", Tier::Medium, vec![]),
            make_deliverable("heavy1", Tier::Heavy, vec![]),
            make_deliverable("stop1", Tier::Light, vec!["gate1"]),
        ];
        let history = vec![];
        let state = CanaryState::default();
        let sample = select_canary_sample(&deliverables, &history, &state);
        let ids: Vec<_> = sample.iter().map(|d| d.deliverable_id.as_str()).collect();
        assert_eq!(ids, vec!["stop1", "heavy1", "medium1", "light1"]);
    }

    #[test]
    fn test_cap_at_8() {
        let mut deliverables: Vec<Deliverable> = (0..12)
            .map(|i| make_deliverable(&format!("d{i}"), Tier::Medium, vec![]))
            .collect();
        // Make a couple stop-gate to verify priority still respected
        deliverables[0] = make_deliverable("d0", Tier::Light, vec!["gate"]);
        let history = vec![];
        let state = CanaryState::default();
        let sample = select_canary_sample(&deliverables, &history, &state);
        assert_eq!(sample.len(), 8);
        // First must be stop-gate
        assert_eq!(sample[0].deliverable_id, "d0");
    }

    #[test]
    fn test_floor_at_3_best_effort() {
        let deliverables = vec![
            make_deliverable("d1", Tier::Light, vec![]),
            make_deliverable("d2", Tier::Light, vec![]),
        ];
        let history = vec![];
        let state = CanaryState::default();
        let sample = select_canary_sample(&deliverables, &history, &state);
        assert_eq!(sample.len(), 2); // floor is best-effort
    }

    #[test]
    fn test_overflow_tracked_via_helper() {
        let deliverables: Vec<Deliverable> = (0..10)
            .map(|i| make_deliverable(&format!("d{i}"), Tier::Medium, vec![]))
            .collect();
        let history = vec![];
        let state = CanaryState::default();
        let sample = select_canary_sample(&deliverables, &history, &state);
        let dropped = compute_dropped_count(&deliverables, &sample);
        assert_eq!(sample.len(), 8);
        assert_eq!(dropped, 2);
    }

    #[test]
    fn test_light_round_robin_by_last_audit() {
        let deliverables = vec![
            make_deliverable("old", Tier::Light, vec![]),
            make_deliverable("recent", Tier::Light, vec![]),
            make_deliverable("never", Tier::Light, vec![]),
        ];
        let history = vec![
            make_audit_run("r1", "2024-01-01T00:00:00Z", vec!["old"]),
            make_audit_run("r2", "2024-01-10T00:00:00Z", vec!["recent"]),
            // "never" has no audit record
        ];
        let state = CanaryState::default();
        let sample = select_canary_sample(&deliverables, &history, &state);
        let ids: Vec<_> = sample.iter().map(|d| d.deliverable_id.as_str()).collect();
        // "never" (empty date) sorts first, then "old", then "recent"
        assert_eq!(ids, vec!["never", "old", "recent"]);
    }

    // ================================================================
    // State persistence tests
    // ================================================================

    #[test]
    fn test_read_write_canary_state_roundtrip() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("canary-replay-state.jsonl");

        let state = CanaryState {
            audit_run_count: 42,
            last_canary_trigger: Some("2024-03-01T00:00:00Z".to_string()),
            canary_dropped_this_run: 3,
            consecutive_drop_runs: 2,
        };
        write_canary_state(&path, &state).unwrap();
        let read = read_canary_state(&path).unwrap();
        assert_eq!(read.audit_run_count, 42);
        assert_eq!(
            read.last_canary_trigger,
            Some("2024-03-01T00:00:00Z".to_string())
        );
        assert_eq!(read.canary_dropped_this_run, 3);
        assert_eq!(read.consecutive_drop_runs, 2);
    }

    #[test]
    fn test_read_missing_returns_default() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("nonexistent.jsonl");
        let state = read_canary_state(&path).unwrap();
        assert_eq!(state.audit_run_count, 0);
        assert_eq!(state.last_canary_trigger, None);
    }

    #[test]
    fn test_update_canary_state_tracks_drops() {
        let mut state = CanaryState::default();
        update_canary_state(&mut state, true, 2, "2024-01-01T00:00:00Z");
        assert_eq!(state.canary_dropped_this_run, 2);
        assert_eq!(state.consecutive_drop_runs, 1);
        assert_eq!(
            state.last_canary_trigger,
            Some("2024-01-01T00:00:00Z".to_string())
        );

        update_canary_state(&mut state, false, 0, "2024-01-02T00:00:00Z");
        assert_eq!(state.canary_dropped_this_run, 0);
        assert_eq!(state.consecutive_drop_runs, 0);
        // Trigger false => last_canary_trigger unchanged
        assert_eq!(
            state.last_canary_trigger,
            Some("2024-01-01T00:00:00Z".to_string())
        );
    }

    // ================================================================
    // Drift detection tests
    // ================================================================

    fn make_audit_report(
        deliverables: Vec<DeliverableReport>,
        blocked: Vec<BlockedDeliverable>,
    ) -> AuditReport {
        AuditReport {
            schema_version: "1".to_string(),
            audit_run_id: "run-1".to_string(),
            started_at: "2024-01-01T00:00:00Z".to_string(),
            completed_at: "2024-01-01T00:00:00Z".to_string(),
            overall_verdict: AuditorVerdict::Pass,
            deliverables,
            blocked_deliverables: blocked,
            metrics: aiplus_core::agent_team::AuditMetrics {
                total_checks: 0,
                passed_checks: 0,
                failed_checks: 0,
                blocked_checks: 0,
                total_execution_time_ms: 0,
                canary_dropped_this_run: 0,
            },
            owner_feedback_prompt: "".to_string(),
        }
    }

    #[test]
    fn test_detect_drift_no_drift() {
        let d = vec![DeliverableReport {
            deliverable_id: "d1".to_string(),
            verdict: AuditorVerdict::Pass,
            checks: vec![],
            execution_time_ms: 0,
        }];
        let orig = make_audit_report(d.clone(), vec![]);
        let replay = make_audit_report(d, vec![]);
        assert!(!detect_drift(&orig, &replay));
    }

    #[test]
    fn test_detect_drift_verdict_changed() {
        let orig = make_audit_report(
            vec![DeliverableReport {
                deliverable_id: "d1".to_string(),
                verdict: AuditorVerdict::Pass,
                checks: vec![],
                execution_time_ms: 0,
            }],
            vec![],
        );
        let replay = make_audit_report(
            vec![DeliverableReport {
                deliverable_id: "d1".to_string(),
                verdict: AuditorVerdict::Fail,
                checks: vec![],
                execution_time_ms: 0,
            }],
            vec![],
        );
        assert!(detect_drift(&orig, &replay));
    }

    #[test]
    fn test_detect_drift_missing_deliverable() {
        let orig = make_audit_report(
            vec![DeliverableReport {
                deliverable_id: "d1".to_string(),
                verdict: AuditorVerdict::Pass,
                checks: vec![],
                execution_time_ms: 0,
            }],
            vec![],
        );
        let replay = make_audit_report(vec![], vec![]);
        assert!(detect_drift(&orig, &replay));
    }

    #[test]
    fn test_detect_drift_extra_deliverable() {
        let orig = make_audit_report(vec![], vec![]);
        let replay = make_audit_report(
            vec![DeliverableReport {
                deliverable_id: "d1".to_string(),
                verdict: AuditorVerdict::Pass,
                checks: vec![],
                execution_time_ms: 0,
            }],
            vec![],
        );
        assert!(detect_drift(&orig, &replay));
    }

    #[test]
    fn test_detect_drift_blocked_changed() {
        let orig = make_audit_report(
            vec![],
            vec![BlockedDeliverable {
                deliverable_id: "d1".to_string(),
                reason: BlockedReason::SchemaTampered,
                detail: "bad".to_string(),
            }],
        );
        let replay = make_audit_report(vec![], vec![]);
        assert!(detect_drift(&orig, &replay));
    }

    #[test]
    fn test_detect_drift_blocked_reason_changed() {
        let orig = make_audit_report(
            vec![],
            vec![BlockedDeliverable {
                deliverable_id: "d1".to_string(),
                reason: BlockedReason::SchemaTampered,
                detail: "bad".to_string(),
            }],
        );
        let replay = make_audit_report(
            vec![],
            vec![BlockedDeliverable {
                deliverable_id: "d1".to_string(),
                reason: BlockedReason::ManifestDirty,
                detail: "dirty".to_string(),
            }],
        );
        assert!(detect_drift(&orig, &replay));
    }

    // ================================================================
    // Date helper tests
    // ================================================================

    #[test]
    fn test_days_between_same_day() {
        assert_eq!(
            days_between("2024-01-15T00:00:00Z", "2024-01-15T23:59:59Z"),
            Some(0)
        );
    }

    #[test]
    fn test_days_between_14_days() {
        assert_eq!(
            days_between("2024-01-01T00:00:00Z", "2024-01-15T00:00:00Z"),
            Some(14)
        );
    }

    #[test]
    fn test_days_between_cross_year() {
        assert_eq!(
            days_between("2023-12-31T00:00:00Z", "2024-01-01T00:00:00Z"),
            Some(1)
        );
    }

    #[test]
    fn test_is_different_month() {
        assert!(!is_different_month(
            "2024-01-15T00:00:00Z",
            "2024-01-20T00:00:00Z"
        ));
        assert!(is_different_month(
            "2024-01-15T00:00:00Z",
            "2024-02-01T00:00:00Z"
        ));
    }

    #[test]
    fn test_find_last_audit_for_deliverable() {
        let history = vec![
            make_audit_run("r1", "2024-01-01T00:00:00Z", vec!["d1"]),
            make_audit_run("r2", "2024-01-10T00:00:00Z", vec!["d1"]),
            make_audit_run("r3", "2024-01-05T00:00:00Z", vec!["d2"]),
        ];
        let found = find_last_audit_for_deliverable("d1", &history);
        assert_eq!(found.unwrap().run_id, "r2");
        assert!(find_last_audit_for_deliverable("d3", &history).is_none());
    }

    #[test]
    fn test_find_last_audit_includes_blocked() {
        let mut run = make_audit_run("r1", "2024-01-01T00:00:00Z", vec![]);
        run.blocked_deliverables.push(BlockedDeliverable {
            deliverable_id: "d1".to_string(),
            reason: BlockedReason::SchemaTampered,
            detail: "".to_string(),
        });
        let history = vec![run];
        let found = find_last_audit_for_deliverable("d1", &history);
        assert_eq!(found.unwrap().run_id, "r1");
    }
}
