// Integration tests for PR-C — Issues #32, #33, #34.
//
// #32: `aiplus agent status` filters roster by active_team when both
//      agent-team and aieconlab are installed in the same project.
// #33: `aiplus agent route pi <research task>` scores research-paper
//      vocabulary (scoping note, data acquisition, referee response,
//      etc.) at MEDIUM/HEAVY so the consultant can fire.
// #34: `aiplus compact prepare` on a fresh install (seed handoff,
//      seed Owner Gate placeholder) returns FRESH_INSTALL_AWAITING_FIRST_USE
//      with exit 0 instead of UNKNOWN_NEEDS_REVIEW.

use std::fs;
use std::path::Path;
use std::process::{Command, Output};

fn bin() -> &'static str {
    env!("CARGO_BIN_EXE_aiplus")
}

fn run_with_status(cwd: &Path, args: &[&str]) -> Output {
    let mut command = Command::new(bin());
    command
        .args(args)
        .current_dir(cwd)
        .env("HOME", cwd.join("fake-home"))
        .env("CODEX_HOME", cwd.join("fake-codex-home"))
        .env("XDG_CONFIG_HOME", cwd.join("fake-xdg"));
    command.output().expect("run aiplus")
}

fn run(cwd: &Path, args: &[&str], expected: i32) -> Output {
    let output = run_with_status(cwd, args);
    assert_eq!(
        output.status.code(),
        Some(expected),
        "{} failed (expected {expected})\nstdout:\n{}\nstderr:\n{}",
        args.join(" "),
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    output
}

fn prepare(target: &Path) {
    fs::create_dir(target.join("fake-home")).unwrap();
    fs::create_dir(target.join("fake-codex-home")).unwrap();
    fs::create_dir(target.join("fake-xdg")).unwrap();
}

// ---------------------------------------------------------------- #32

#[test]
fn agent_status_filters_roster_by_active_team_aieconlab() {
    let temp = tempfile::tempdir().unwrap();
    let target = temp.path();
    prepare(target);

    // Install both modules so the dual-team scenario triggers.
    run(target, &["install", "codex"], 0); // installs agent-team via auto-install
    run(target, &["add", "aieconlab"], 0); // activates aieconlab

    let out = run(target, &["agent", "status"], 0);
    let text = String::from_utf8_lossy(&out.stdout);
    assert!(
        text.contains("Active team: aieconlab"),
        "active team should be aieconlab after add aieconlab:\n{text}"
    );

    // The agent-team-only roles (architect, ceo, engineer-a, etc.) must
    // NOT appear in the roster while aieconlab is active. Likewise
    // aieconlab-only roles (pi, theorist, ra-stata) MUST appear.
    let total_line = text
        .lines()
        .find(|l| l.contains("Total agents:"))
        .expect("Total agents: line should exist");
    let total: usize = total_line
        .split(':')
        .nth(1)
        .and_then(|s| s.trim().parse().ok())
        .unwrap_or(0);
    assert!(
        total <= 20,
        "with aieconlab active, total agents should be the AEL roster (8 core + 12 experts = 20), \
         not the merged 37; got {total}\n{text}"
    );

    // Spot-check: SWE roles should not appear in the worktree section
    // (which is built from the same filtered roster).
    assert!(
        !text.contains("ceo: ../") && !text.contains("architect: ../"),
        "SWE roles ceo / architect should not appear when aieconlab is active:\n{text}"
    );
}

#[test]
fn agent_status_filters_roster_by_active_team_agent_team() {
    let temp = tempfile::tempdir().unwrap();
    let target = temp.path();
    prepare(target);

    run(target, &["install", "codex"], 0);
    run(target, &["add", "aieconlab"], 0);
    // Switch back to agent-team.
    run(target, &["agent", "set-team", "agent-team"], 0);

    let out = run(target, &["agent", "status"], 0);
    let text = String::from_utf8_lossy(&out.stdout);
    assert!(
        text.contains("Active team: agent-team"),
        "active team should be agent-team after set-team:\n{text}"
    );
    let total_line = text
        .lines()
        .find(|l| l.contains("Total agents:"))
        .expect("Total agents: line should exist");
    let total: usize = total_line
        .split(':')
        .nth(1)
        .and_then(|s| s.trim().parse().ok())
        .unwrap_or(0);
    assert!(
        total <= 20,
        "with agent-team active, total agents should be the SWE roster, not merged; got {total}\n{text}"
    );
    // AEL-only roles should not appear.
    assert!(
        !text.contains("pi: ../")
            && !text.contains("theorist: ../")
            && !text.contains("ra-stata: ../"),
        "AEL roles pi / theorist / ra-stata should not appear when agent-team is active:\n{text}"
    );
}

// Note: Issue #33 (`score_task_tier` research vocabulary) is covered
// by unit tests in `crates/aiplus-cli/src/agent/state.rs` because
// `aiplus-cli` is a binary crate (no lib target) and integration
// tests cannot import `score_task_tier` directly. The unit-test
// module asserts heavy/medium/LIGHT regression coverage.

// ---------------------------------------------------------------- #34

#[test]
fn compact_prepare_on_fresh_install_returns_fresh_install_state_with_exit_0() {
    let temp = tempfile::tempdir().unwrap();
    let target = temp.path();
    prepare(target);

    run(target, &["install", "claude-code"], 0);

    // No edits yet — handoff and Owner Gates are still in seed state.
    let out = run_with_status(target, &["compact", "prepare"]);
    assert_eq!(
        out.status.code(),
        Some(0),
        "fresh-install compact prepare should exit 0\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
    let text = String::from_utf8_lossy(&out.stdout);
    assert!(
        text.contains("READINESS_STATE=FRESH_INSTALL_AWAITING_FIRST_USE"),
        "fresh install should report FRESH_INSTALL_AWAITING_FIRST_USE state, got:\n{text}"
    );
    assert!(
        text.contains("COMPACT_PRESSURE=INFO"),
        "fresh install pressure should be INFO, got:\n{text}"
    );
    assert!(
        text.contains("seed state"),
        "fresh install explanation should mention seed state:\n{text}"
    );
}

#[test]
fn compact_prepare_after_handoff_edit_returns_unknown_needs_review() {
    let temp = tempfile::tempdir().unwrap();
    let target = temp.path();
    prepare(target);

    run(target, &["install", "claude-code"], 0);

    // Edit the seed Current Goal but keep the seed Owner Gate as-is.
    // This is the genuine "unreviewed-after-edit" state — we want it
    // to still surface UNKNOWN_NEEDS_REVIEW (the noise-suppressing
    // fix in #34 must not silently approve real review work).
    let handoff = target.join(".aiplus/compact/current-handoff.md");
    let body = fs::read_to_string(&handoff).unwrap();
    let modified = body.replace(
        "Initialize compact/resume handoff state for",
        "Investigate the failing OAuth refresh flow for",
    );
    fs::write(&handoff, modified).unwrap();

    let out = run_with_status(target, &["compact", "prepare"]);
    let text = String::from_utf8_lossy(&out.stdout);
    assert!(
        text.contains("READINESS_STATE=UNKNOWN_NEEDS_REVIEW"),
        "post-edit state should still surface UNKNOWN_NEEDS_REVIEW, got:\n{text}"
    );
    assert_ne!(
        out.status.code(),
        Some(0),
        "real review state must not exit 0\nstdout:\n{text}"
    );
}
