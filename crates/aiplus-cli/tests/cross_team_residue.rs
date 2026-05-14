// Track A.2: cross-team residue cleanup at install time.
//
// Before this fix, `agent_team_init` and `aieconlab_init` wrote their
// files into `.aiplus/agents/` on top of whatever the other team had
// left behind. `snapshot_active_team` then captured the merged state,
// `set_active_team` restored it, and `mirror_personas_to_runtimes`
// pushed both teams' bare role-name files into `.claude/agents/` —
// orphans that uninstall could not safely remove.
//
// Now each init's first move is to clear the OTHER team's exclusive
// files. Shared role names (advisor, pm) survive because the calling
// init overwrites them right after.

use std::collections::HashSet;
use std::fs;
use std::path::Path;
use std::process::{Command, Output};

fn bin() -> &'static str {
    env!("CARGO_BIN_EXE_aiplus")
}

fn run(cwd: &Path, args: &[&str], expected: i32) -> Output {
    let mut command = Command::new(bin());
    command
        .args(args)
        .current_dir(cwd)
        .env("HOME", cwd.join("fake-home"))
        .env("CODEX_HOME", cwd.join("fake-codex-home"))
        .env("XDG_CONFIG_HOME", cwd.join("fake-xdg"));
    let output = command.output().expect("run aiplus");
    assert_eq!(
        output.status.code(),
        Some(expected),
        "{} failed\nstdout:\n{}\nstderr:\n{}",
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

/// All file basenames under a directory (non-recursive).
fn basenames(dir: &Path) -> HashSet<String> {
    if !dir.exists() {
        return HashSet::new();
    }
    fs::read_dir(dir)
        .unwrap()
        .filter_map(|e| e.ok())
        .filter(|e| e.path().is_file())
        .filter_map(|e| e.file_name().into_string().ok())
        .collect()
}

#[test]
fn add_aieconlab_clears_agent_team_exclusive_files() {
    let temp = tempfile::tempdir().unwrap();
    let target = temp.path();
    prepare(target);

    // `install codex` auto-installs agent-team. After this step,
    // .aiplus/agents/ holds the agent-team layout.
    run(target, &["install", "codex"], 0);
    let agents = target.join(".aiplus/agents");
    let personas = agents.join("personas");
    let experts = agents.join("experts");
    let stubs = personas.join("_stubs");
    assert!(agents.join("ceo.toml").exists());
    assert!(personas.join("architect.md").exists());

    // Now add aieconlab. Track A.2 says: agent-team's exclusive files
    // must NOT be in .aiplus/agents/ afterwards.
    run(target, &["add", "aieconlab"], 0);

    let role_tomls = basenames(&agents);
    for name in [
        "ceo.toml",
        "architect.toml",
        "engineer-a.toml",
        "engineer-b.toml",
        "reviewer.toml",
        "qa.toml",
        "agent-team.toml",
    ] {
        assert!(
            !role_tomls.contains(name),
            "agent-team-exclusive {name} should have been cleared from .aiplus/agents/, found: {role_tomls:?}"
        );
    }

    let core_personas = basenames(&personas);
    for name in [
        "ceo.md",
        "architect.md",
        "engineer-a.md",
        "engineer-b.md",
        "reviewer.md",
        "qa.md",
    ] {
        assert!(
            !core_personas.contains(name),
            "agent-team-exclusive persona {name} should have been cleared, found: {core_personas:?}"
        );
    }

    let expert_tomls = basenames(&experts);
    for name in [
        "ai-integration.toml",
        "security-reviewer.toml",
        "tech-writer.toml",
        "devops.toml",
        "ui-designer.toml",
        "researcher.toml",
        "data-analyst.toml",
        "customer-researcher.toml",
        "performance-engineer.toml",
        "accessibility.toml",
        "compliance-reviewer.toml",
    ] {
        assert!(
            !expert_tomls.contains(name),
            "agent-team expert {name} should have been cleared, found: {expert_tomls:?}"
        );
    }

    // The agent-team-only _stubs personas should also be gone.
    let stub_personas = basenames(&stubs);
    for name in [
        "data-analyst.md",
        "customer-researcher.md",
        "performance-engineer.md",
        "accessibility.md",
        "compliance-reviewer.md",
    ] {
        assert!(
            !stub_personas.contains(name),
            "agent-team stub {name} should have been cleared, found: {stub_personas:?}"
        );
    }

    // And AEL's own files should be present.
    assert!(agents.join("pi.toml").exists());
    assert!(agents.join("theorist.toml").exists());
    assert!(agents.join("econ-team.toml").exists());
    assert!(personas.join("pi.md").exists());
    // Shared names survive because aieconlab_init overwrote them.
    assert!(agents.join("advisor.toml").exists());
    assert!(agents.join("pm.toml").exists());
}

#[test]
fn set_team_back_to_agent_team_restores_clean_swe_layout() {
    // After A.2 cleans residue at init time, the snapshot mechanism
    // saves clean per-team state. `aiplus agent set-team agent-team`
    // after AEL was active must restore agent-team-only files —
    // proving the snapshot captured at install/add time was free of
    // cross-team merge.
    let temp = tempfile::tempdir().unwrap();
    let target = temp.path();
    prepare(target);

    run(target, &["install", "codex"], 0);
    run(target, &["add", "aieconlab"], 0);
    // Now active = aieconlab. Switch back to agent-team via the
    // documented set-team primitive (re-running `aiplus add
    // agent-team` is a no-op when the module is already installed).
    run(target, &["agent", "set-team", "agent-team"], 0);

    let agents = target.join(".aiplus/agents");
    let personas = agents.join("personas");
    let experts = agents.join("experts");

    // AEL exclusive files should be out of the live layout.
    let role_tomls = basenames(&agents);
    for name in [
        "pi.toml",
        "theorist.toml",
        "ra-stata.toml",
        "ra-python.toml",
        "referee.toml",
        "replicator.toml",
        "econ-team.toml",
    ] {
        assert!(
            !role_tomls.contains(name),
            "aieconlab-exclusive {name} should have been cleared by set-team, found: {role_tomls:?}"
        );
    }

    let core_personas = basenames(&personas);
    for name in [
        "pi.md",
        "theorist.md",
        "ra-stata.md",
        "ra-python.md",
        "referee.md",
        "replicator.md",
    ] {
        assert!(
            !core_personas.contains(name),
            "aieconlab persona {name} should have been cleared, found: {core_personas:?}"
        );
    }

    let expert_tomls = basenames(&experts);
    for name in [
        "coauthor-liaison.toml",
        "computation.toml",
        "econometrician.toml",
        "ethics-irb.toml",
        "historical-sources.toml",
        "job-talk-coach.toml",
        "lit-reviewer.toml",
        "llm-measurement.toml",
        "reproducibility.toml",
        "survey-experiment.toml",
        "viz-specialist.toml",
        "writer.toml",
    ] {
        assert!(
            !expert_tomls.contains(name),
            "aieconlab expert {name} should have been cleared, found: {expert_tomls:?}"
        );
    }

    // agent-team's own files are back.
    assert!(agents.join("ceo.toml").exists());
    assert!(agents.join("architect.toml").exists());
    assert!(agents.join("agent-team.toml").exists());
    assert!(personas.join("ceo.md").exists());
}

#[test]
fn dual_install_then_uninstall_leaves_no_bare_mirror_orphans() {
    // The acid-test for A.2 closing the residue gap that A.1
    // documented as a known limitation. With A.2 in place,
    // `aiplus install + add aieconlab + uninstall` should leave NO
    // bare role-name files behind in `.claude/agents/`.
    let temp = tempfile::tempdir().unwrap();
    let target = temp.path();
    prepare(target);

    run(target, &["install", "claude-code"], 0);
    run(target, &["add", "aieconlab"], 0);
    run(target, &["uninstall", "--yes"], 0);

    // .claude/agents/ should be either pruned (empty) or contain
    // only user-authored files (none in this test).
    let agents_dir = target.join(".claude/agents");
    let residue: Vec<String> = if agents_dir.exists() {
        fs::read_dir(&agents_dir)
            .unwrap()
            .filter_map(|e| e.ok())
            .filter_map(|e| e.file_name().into_string().ok())
            .collect()
    } else {
        Vec::new()
    };
    assert!(
        residue.is_empty(),
        "post-uninstall .claude/agents/ has residue {residue:?} — \
         A.2 should have prevented the cross-team merge that \
         caused these orphans"
    );
}

#[test]
fn switching_teams_restores_correct_roster() {
    // End-to-end: install, add aieconlab (active = aieconlab),
    // switch back to agent-team via set-team, verify roster reflects
    // agent-team only. Validates that the snapshot mechanism is now
    // also clean after A.2.
    let temp = tempfile::tempdir().unwrap();
    let target = temp.path();
    prepare(target);

    run(target, &["install", "codex"], 0);
    run(target, &["add", "aieconlab"], 0);
    run(target, &["agent", "set-team", "agent-team"], 0);

    let agents = target.join(".aiplus/agents");
    // After switching back to agent-team, the AEL files should be
    // out of the live layout.
    assert!(
        !agents.join("pi.toml").exists(),
        "AEL pi.toml leaked after set-team agent-team"
    );
    assert!(!agents.join("theorist.toml").exists());
    // And agent-team files are back.
    assert!(agents.join("ceo.toml").exists());
    assert!(agents.join("architect.toml").exists());
}
