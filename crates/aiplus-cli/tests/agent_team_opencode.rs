// Track B.2: agent-team OpenCode adapter integration tests.
//
// Same shape as the v0.1 claude-code adapter checks, but writing
// to `.opencode/agents/` and `.opencode/commands/`. Validates:
//   (1) all 14 prefixed agent files written with YAML frontmatter
//   (2) 2 slash commands at .opencode/commands/at-{status,route}.md
//   (3) bare-mirror residue cleaned
//   (4) no-op when opencode not in runtimeAdapters
//   (5) doctor passes
//   (6) coexistence with the AEL opencode adapter
//   (7) uninstall sweeps the new opencode agent-team prefixed files

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

const CORE_ROLES: &[&str] = &[
    "advisor",
    "ceo",
    "architect",
    "pm",
    "engineer-a",
    "engineer-b",
    "reviewer",
    "qa",
];

const FUNCTIONAL_EXPERTS: &[&str] = &[
    "ai-integration",
    "security-reviewer",
    "tech-writer",
    "devops",
    "ui-designer",
    "researcher",
];

#[test]
fn install_opencode_writes_14_agent_team_subagents_with_frontmatter() {
    let temp = tempfile::tempdir().unwrap();
    let target = temp.path();
    prepare(target);

    // agent-team is auto_install: true, so install opencode brings it.
    run(target, &["install", "opencode"], 0);

    for role in CORE_ROLES.iter().chain(FUNCTIONAL_EXPERTS.iter()) {
        let path = target.join(format!(".opencode/agents/agent-team-{role}.md"));
        assert!(
            path.exists(),
            "missing .opencode/agents/agent-team-{role}.md"
        );
        let body = fs::read_to_string(&path).unwrap();
        let first_line = body.lines().next().unwrap_or("");
        assert_eq!(
            first_line, "---",
            "agent-team-{role}.md missing YAML opening at .opencode/agents/"
        );
        assert!(
            body.contains(&format!("name: agent-team-{role}")),
            "agent-team-{role}.md missing name field"
        );
    }

    for role in CORE_ROLES {
        let unprefixed = target.join(format!(".opencode/agents/{role}.md"));
        assert!(
            !unprefixed.exists(),
            "unprefixed bare agent-team persona .opencode/agents/{role}.md should have been cleaned"
        );
    }
}

#[test]
fn install_opencode_writes_agent_team_slash_commands() {
    let temp = tempfile::tempdir().unwrap();
    let target = temp.path();
    prepare(target);

    run(target, &["install", "opencode"], 0);

    for cmd in ["at-status", "at-route"] {
        let path = target.join(format!(".opencode/commands/{cmd}.md"));
        assert!(path.exists(), "missing .opencode/commands/{cmd}.md");
        let body = fs::read_to_string(&path).unwrap();
        assert!(
            body.contains(&format!("/{cmd}")),
            "{cmd} body should reference its slash form"
        );
    }
}

#[test]
fn agent_team_opencode_adapter_is_noop_when_opencode_not_installed() {
    let temp = tempfile::tempdir().unwrap();
    let target = temp.path();
    prepare(target);

    run(target, &["install", "codex"], 0);

    assert!(
        !target.join(".opencode/agents/agent-team-ceo.md").exists(),
        "agent-team opencode adapter should be a no-op when opencode isn't installed"
    );
}

#[test]
fn doctor_passes_after_install_opencode_with_agent_team() {
    let temp = tempfile::tempdir().unwrap();
    let target = temp.path();
    prepare(target);

    run(target, &["install", "opencode"], 0);
    let out = run(target, &["doctor"], 0);
    let text = String::from_utf8_lossy(&out.stdout);
    assert!(
        text.contains("DOCTOR_STATUS=PASS"),
        "doctor not green:\n{text}"
    );
}

#[test]
fn dual_install_opencode_then_add_aieconlab_writes_both_team_adapters() {
    let temp = tempfile::tempdir().unwrap();
    let target = temp.path();
    prepare(target);

    run(target, &["install", "opencode"], 0);
    // agent-team adapter is in place from install.
    assert!(target.join(".opencode/agents/agent-team-ceo.md").exists());

    // Add AEL — its opencode adapter writes alongside agent-team's.
    run(target, &["add", "aieconlab"], 0);

    assert!(target.join(".opencode/agents/aieconlab-pi.md").exists());
    // After A.2 cleanup, agent-team-only files should be cleared
    // from .aiplus/agents/, but the .opencode/agents/agent-team-*.md
    // mirror is left in place because the adapter only writes,
    // doesn't sweep cross-team. In a future improvement the
    // adapter could rewrite based on the active team — for now
    // both teams' agent files coexist in `.opencode/agents/` (both
    // are correctly frontmatter-wrapped, so no routing collision).
    assert!(target.join(".opencode/agents/agent-team-ceo.md").exists());
}

#[test]
fn uninstall_cleans_opencode_agent_team_prefixed_files() {
    let temp = tempfile::tempdir().unwrap();
    let target = temp.path();
    prepare(target);

    run(target, &["install", "opencode"], 0);
    assert!(target.join(".opencode/agents/agent-team-ceo.md").exists());

    run(target, &["uninstall", "--yes"], 0);

    for role in CORE_ROLES.iter().chain(FUNCTIONAL_EXPERTS.iter()) {
        let path = target.join(format!(".opencode/agents/agent-team-{role}.md"));
        assert!(
            !path.exists(),
            "uninstall should have removed .opencode/agents/agent-team-{role}.md"
        );
    }
    for cmd in ["at-status", "at-route"] {
        let path = target.join(format!(".opencode/commands/{cmd}.md"));
        assert!(
            !path.exists(),
            "uninstall should have removed .opencode/commands/{cmd}.md"
        );
    }
}
