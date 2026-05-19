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
        "**Dispatch flow**",
        "call `agent_route_score_only` to preview staffing",
        "call `agent_integrate <role>`",
        "prefer aiplus `agent_*` MCP tools over shell grep",
        "do NOT answer from training data first",
        "Full tool list: 11 existing `agent_*` tools + 3 from v0.6.7",
    ] {
        assert!(
            text.contains(expected),
            "{} missing {expected}:\n{text}",
            path.display()
        );
    }
    assert_eq!(
        text.matches("**Dispatch flow**").count(),
        1,
        "{} should contain one dispatch-flow paragraph:\n{text}",
        path.display()
    );
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
        "Prefer MCP Tools Over CLI Subcommands",
        "Do NOT immediately answer with design checklists from training data",
        "aiplus agent dispatch-history --json",
        "Known Runtime Limitation",
        "## Dispatch Flow",
        "call `agent_route_score_only` with the user's current task",
        "call `agent_integrate <role>` per completed role",
        "## Multi-turn Patterns",
        "### Follow-up Cost Question",
        "### Mid-flight Scope Change",
        "### Ambiguous Audit Intent",
    ] {
        assert!(
            text.contains(expected),
            "{} missing {expected}:\n{text}",
            path.display()
        );
    }
    assert_eq!(
        text.matches("## Dispatch Flow").count(),
        1,
        "{} should contain one Dispatch Flow section:\n{text}",
        path.display()
    );
    assert_eq!(
        text.matches("## Multi-turn Patterns").count(),
        1,
        "{} should contain one Multi-turn Patterns section:\n{text}",
        path.display()
    );
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
