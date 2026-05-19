use std::fs;
use std::path::Path;
use std::process::{Command, Output};

const DISCOVERY_BEGIN: &str = "<!-- aiplus-discovery-block:start -->";
const DISCOVERY_END: &str = "<!-- /aiplus-discovery-block -->";

fn bin() -> &'static str {
    env!("CARGO_BIN_EXE_aiplus")
}

fn run(cwd: &Path, args: &[&str], expected: i32) -> Output {
    let output = Command::new(bin())
        .args(args)
        .current_dir(cwd)
        .env("HOME", cwd.join("fake-home"))
        .env("CODEX_HOME", cwd.join("fake-codex-home"))
        .env("CLAUDE_CONFIG_DIR", cwd.join("fake-claude-home"))
        .env("XDG_CONFIG_HOME", cwd.join("fake-xdg"))
        .env("AIPLUS_SECRET_BROKER_DISABLE_KEYCHAIN", "1")
        .env_remove("ANTHROPIC_API_KEY")
        .env_remove("OPENAI_API_KEY")
        .env_remove("BWS_ACCESS_TOKEN")
        .output()
        .expect("run aiplus");
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
    fs::write(target.join("README.md"), "# Discovery test\n").unwrap();
    Command::new("git")
        .args(["init"])
        .current_dir(target)
        .output()
        .expect("git init");
    Command::new("git")
        .args(["config", "user.email", "test@example.com"])
        .current_dir(target)
        .output()
        .expect("git config email");
    Command::new("git")
        .args(["config", "user.name", "Test User"])
        .current_dir(target)
        .output()
        .expect("git config name");
}

fn assert_discovery_block_once(path: &Path) {
    let text = fs::read_to_string(path).unwrap();
    assert_eq!(
        text.matches(DISCOVERY_BEGIN).count(),
        1,
        "{} should contain one discovery begin marker:\n{text}",
        path.display()
    );
    assert_eq!(
        text.matches(DISCOVERY_END).count(),
        1,
        "{} should contain one discovery end marker:\n{text}",
        path.display()
    );
    for expected in [
        "agent_token_cost",
        "agent_route_score_only",
        "agent_audit_verify_log",
        "agent_route",
        "agent_status",
        "agent_set_team",
        "agent_list",
        "agent_doctor",
        "agent_invite",
        "agent_dismiss",
        "agent_disable",
        "agent_enable",
        "agent_integrate",
        "agent_talk",
        "aiplus memory record|context|status",
        "aiplus compact",
        "aiplus velocity",
        "aiplus identity setup-signing",
        "aiplus doctor",
        "prefer aiplus `agent_*` MCP tools over shell grep",
        "do NOT answer from training data first",
        "Full tool list: 14 `agent_*` MCP tools",
    ] {
        assert!(
            text.contains(expected),
            "{} missing {expected}:\n{text}",
            path.display()
        );
    }
}

fn assert_skill(path: &Path) {
    let text = fs::read_to_string(path).unwrap();
    assert!(
        text.starts_with("---\nname: aiplus\n"),
        "{} missing skill frontmatter:\n{text}",
        path.display()
    );
    for expected in [
        "agent_token_cost",
        "agent_route_score_only",
        "agent_audit_verify_log",
        "agent_route",
        "agent_status",
        "agent_set_team",
        "agent_list",
        "agent_doctor",
        "agent_invite",
        "agent_dismiss",
        "agent_disable",
        "agent_enable",
        "agent_integrate",
        "agent_talk",
        "aiplus memory record",
        "aiplus memory context --runtime <runtime>",
        "aiplus memory status",
        "aiplus compact prepare",
        "aiplus compact resume",
        "aiplus compact savings",
        "aiplus velocity estimate",
        "aiplus velocity report",
        "aiplus identity setup-signing [--dry-run]",
        "aiplus doctor [--quiet] [--check-keyring]",
        "Prefer MCP Tools Over CLI Subcommands",
        "Use These Tools First",
        "Do NOT immediately answer with design checklists from training data",
        "aiplus agent dispatch-history --json",
        "Known Runtime Limitation",
    ] {
        assert!(
            text.contains(expected),
            "{} missing {expected}:\n{text}",
            path.display()
        );
    }
}

#[test]
fn install_all_writes_discovery_skills_and_preambles_idempotently() {
    let temp = tempfile::tempdir().unwrap();
    let target = temp.path();
    prepare(target);

    fs::write(
        target.join("AGENTS.md"),
        "# Existing Agents\n\nKeep this user content.\n",
    )
    .unwrap();
    fs::write(
        target.join("CLAUDE.md"),
        "# Existing Claude\n\nKeep this Claude content.\n",
    )
    .unwrap();

    run(target, &["install", "all", "--yes"], 0);
    run(target, &["install", "all", "--yes"], 0);

    assert_skill(&target.join(".claude/skills/aiplus/SKILL.md"));
    assert_skill(&target.join(".codex/skills/aiplus/SKILL.md"));
    assert_skill(&target.join(".agents/skills/aiplus/SKILL.md"));
    assert_skill(&target.join(".opencode/skills/aiplus/SKILL.md"));

    assert_discovery_block_once(&target.join("AGENTS.md"));
    assert_discovery_block_once(&target.join("CLAUDE.md"));
    assert_discovery_block_once(&target.join(".opencode/instructions/aiplus.md"));

    let agents = fs::read_to_string(target.join("AGENTS.md")).unwrap();
    assert!(
        agents.contains("Keep this user content."),
        "AGENTS.md user content should survive:\n{agents}"
    );
    assert!(
        agents.contains("<!-- BEGIN AIPLUS MANAGED BLOCK -->"),
        "AGENTS.md should still contain core AiPlus block:\n{agents}"
    );

    let claude = fs::read_to_string(target.join("CLAUDE.md")).unwrap();
    assert!(
        claude.contains("Keep this Claude content."),
        "CLAUDE.md user content should survive:\n{claude}"
    );
    assert!(
        claude.contains("<!-- BEGIN AIPLUS MANAGED BLOCK -->"),
        "CLAUDE.md should still contain core AiPlus block:\n{claude}"
    );
}
