// Integration test for the agent-team Claude Code adapter (Issue #31).
//
// Before the adapter shipped, `aiplus add agent-team` (or its
// auto-install path during `aiplus install claude-code`) left
// `.claude/agents/<role>.md` files without YAML frontmatter, so
// Claude Code's auto-routing could not see them. This test asserts:
//
//   1. After `aiplus install claude-code`, all 14 prefixed subagent
//      files exist with valid YAML frontmatter (name + description).
//   2. The legacy bare-name files (advisor.md, ceo.md, …) written by
//      `mirror_personas_to_runtimes` are cleaned up so Claude Code
//      sees only the prefixed, frontmatter-bearing entries.
//   3. Slash commands /at-status and /at-route are installed.
//   4. CLAUDE.md gains exactly one AiPlus-Agent-Team managed block
//      (without disturbing user-authored content above it).
//   5. The adapter is a no-op when claude-code is not among the
//      project's runtime adapters.
//   6. `aiplus uninstall --yes` strips the managed block from
//      CLAUDE.md while preserving user content.

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
fn install_claude_code_writes_14_agent_team_subagents_with_frontmatter() {
    let temp = tempfile::tempdir().unwrap();
    let target = temp.path();
    prepare(target);

    // agent-team is `auto_install: true`, so `aiplus install claude-code`
    // implicitly runs agent_team_init and therefore the new adapter.
    run(target, &["install", "claude-code"], 0);

    for role in CORE_ROLES.iter().chain(FUNCTIONAL_EXPERTS.iter()) {
        let path = target.join(format!(".claude/agents/agent-team-{role}.md"));
        assert!(path.exists(), "missing agent-team-{role}.md");
        let body = fs::read_to_string(&path).unwrap();
        let first_line = body.lines().next().unwrap_or("");
        assert_eq!(
            first_line, "---",
            "agent-team-{role}.md missing YAML opening"
        );
        assert!(
            body.contains(&format!("name: agent-team-{role}")),
            "agent-team-{role}.md missing name field"
        );
        assert!(
            body.contains("description:"),
            "agent-team-{role}.md missing description"
        );
    }

    // Legacy bare-name persona files written by mirror_personas_to_runtimes
    // (advisor.md, ceo.md, …) should be cleaned up; only the prefixed,
    // frontmatter-bearing copies remain.
    for role in CORE_ROLES {
        let unprefixed = target.join(format!(".claude/agents/{role}.md"));
        assert!(
            !unprefixed.exists(),
            "unprefixed bare agent-team persona .claude/agents/{role}.md \
             should have been cleaned up so Claude Code only sees the prefixed copy"
        );
    }
}

#[test]
fn install_claude_code_writes_agent_team_slash_commands() {
    let temp = tempfile::tempdir().unwrap();
    let target = temp.path();
    prepare(target);

    run(target, &["install", "claude-code"], 0);

    for cmd in ["at-status", "at-route"] {
        let path = target.join(format!(".claude/commands/{cmd}.md"));
        assert!(path.exists(), "missing slash command {cmd}");
        let body = fs::read_to_string(&path).unwrap();
        assert!(
            body.contains(&format!("/{cmd}")),
            "{cmd} body should reference its slash form"
        );
    }
}

#[test]
fn install_claude_code_inserts_agent_team_managed_block_preserving_user_content() {
    let temp = tempfile::tempdir().unwrap();
    let target = temp.path();
    prepare(target);

    let preexisting = "## Project Notes\nKeep me.\n";
    fs::write(target.join("CLAUDE.md"), preexisting).unwrap();

    run(target, &["install", "claude-code"], 0);

    let body = fs::read_to_string(target.join("CLAUDE.md")).unwrap();
    assert!(
        body.starts_with("## Project Notes\nKeep me."),
        "user content not preserved at top:\n{body}"
    );
    assert_eq!(
        body.matches("<!-- BEGIN AIPLUS-AGENT-TEAM MANAGED BLOCK -->")
            .count(),
        1,
        "expected exactly one agent-team managed block"
    );
    assert_eq!(
        body.matches("<!-- END AIPLUS-AGENT-TEAM MANAGED BLOCK -->")
            .count(),
        1,
        "expected matching end marker"
    );
    assert!(
        body.contains("AiPlus Agent Team is installed in this project"),
        "agent-team block body not inserted"
    );
}

#[test]
fn add_agent_team_is_noop_when_claude_code_not_installed() {
    let temp = tempfile::tempdir().unwrap();
    let target = temp.path();
    prepare(target);

    run(target, &["install", "codex"], 0);
    // agent-team is auto-installed by `install codex`; the claude-code
    // adapter should be a no-op in this configuration.

    assert!(
        !target.join(".claude/agents/agent-team-ceo.md").exists(),
        "agent-team claude-code adapter should be a no-op when claude-code isn't installed"
    );
    // No CLAUDE.md should be created on a codex-only install.
    let claude_md = target.join("CLAUDE.md");
    if claude_md.exists() {
        let body = fs::read_to_string(&claude_md).unwrap();
        assert!(
            !body.contains("<!-- BEGIN AIPLUS-AGENT-TEAM MANAGED BLOCK -->"),
            "agent-team block should not be inserted when claude-code isn't installed"
        );
    }
}

#[test]
fn doctor_passes_after_install_claude_code_with_agent_team() {
    let temp = tempfile::tempdir().unwrap();
    let target = temp.path();
    prepare(target);

    run(target, &["install", "claude-code"], 0);
    let out = run(target, &["doctor"], 0);
    let text = String::from_utf8_lossy(&out.stdout);
    assert!(
        text.contains("DOCTOR_STATUS=PASS"),
        "doctor not green:\n{text}"
    );
}

#[test]
fn uninstall_strips_agent_team_block_and_preserves_user_content() {
    let temp = tempfile::tempdir().unwrap();
    let target = temp.path();
    prepare(target);

    let preexisting = "## My Notes\nKept content.\n";
    fs::write(target.join("CLAUDE.md"), preexisting).unwrap();

    run(target, &["install", "claude-code"], 0);
    run(target, &["uninstall", "--yes"], 0);

    assert!(
        !target.join(".aiplus").exists(),
        ".aiplus should be removed"
    );

    let body = fs::read_to_string(target.join("CLAUDE.md")).unwrap();
    assert!(body.contains("## My Notes"), "user content lost:\n{body}");
    assert!(body.contains("Kept content."), "user content lost:\n{body}");
    assert!(
        !body.contains("<!-- BEGIN AIPLUS-AGENT-TEAM MANAGED BLOCK -->"),
        "agent-team block should be removed on uninstall:\n{body}"
    );
    assert!(
        !body.contains("AiPlus Agent Team is installed in this project"),
        "agent-team block body should be removed on uninstall:\n{body}"
    );
}
