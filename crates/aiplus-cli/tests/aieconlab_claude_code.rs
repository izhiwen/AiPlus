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
    "pi",
    "theorist",
    "pm",
    "ra-stata",
    "ra-python",
    "referee",
    "replicator",
];

const EXPERTS: &[&str] = &[
    "coauthor-liaison",
    "computation",
    "econometrician",
    "ethics-irb",
    "historical-sources",
    "job-talk-coach",
    "lit-reviewer",
    "llm-measurement",
    "reproducibility",
    "survey-experiment",
    "viz-specialist",
    "writer",
];

#[test]
fn add_aieconlab_writes_20_claude_subagents_with_frontmatter() {
    let temp = tempfile::tempdir().unwrap();
    let target = temp.path();
    prepare(target);

    run(target, &["install", "claude-code"], 0);
    run(target, &["add", "aieconlab"], 0);

    for role in CORE_ROLES.iter().chain(EXPERTS.iter()) {
        let path = target.join(format!(".claude/agents/aieconlab-{role}.md"));
        assert!(path.exists(), "missing aieconlab-{role}.md");
        let body = fs::read_to_string(&path).unwrap();
        let first_line = body.lines().next().unwrap_or("");
        assert_eq!(
            first_line, "---",
            "aieconlab-{role}.md missing YAML opening"
        );
        assert!(
            body.contains(&format!("name: aieconlab-{role}")),
            "aieconlab-{role}.md missing name field"
        );
        assert!(
            body.contains("description:"),
            "aieconlab-{role}.md missing description"
        );
    }

    // Unprefixed AEL persona duplicates from mirror_personas_to_runtimes
    // should be cleaned up; only the prefixed (frontmatter-bearing)
    // versions should remain for AEL roles.
    for role in CORE_ROLES {
        let unprefixed = target.join(format!(".claude/agents/{role}.md"));
        assert!(
            !unprefixed.exists(),
            "unprefixed AEL persona .claude/agents/{role}.md should have been removed"
        );
    }
}

#[test]
fn add_aieconlab_writes_4_slash_commands() {
    let temp = tempfile::tempdir().unwrap();
    let target = temp.path();
    prepare(target);

    run(target, &["install", "claude-code"], 0);
    run(target, &["add", "aieconlab"], 0);

    for cmd in [
        "aiel-route",
        "aiel-talk",
        "aiel-fire-consultant",
        "aiel-status",
    ] {
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
fn add_aieconlab_inserts_claude_md_managed_block() {
    let temp = tempfile::tempdir().unwrap();
    let target = temp.path();
    prepare(target);

    let preexisting = "## Project Notes\nKeep me.\n";
    fs::write(target.join("CLAUDE.md"), preexisting).unwrap();

    run(target, &["install", "claude-code"], 0);
    run(target, &["add", "aieconlab"], 0);

    let body = fs::read_to_string(target.join("CLAUDE.md")).unwrap();
    assert!(
        body.starts_with("## Project Notes\nKeep me."),
        "user content not preserved at top:\n{body}"
    );
    assert_eq!(
        body.matches("<!-- BEGIN AIPLUS MANAGED BLOCK -->").count(),
        1,
        "expected exactly one AiPlus block"
    );
    assert_eq!(
        body.matches("<!-- BEGIN AIECONLAB MANAGED BLOCK -->")
            .count(),
        1,
        "expected exactly one AiEconLab block"
    );
    assert!(
        body.contains("AiEconLab (AEL) is installed in this project"),
        "AEL block body not inserted"
    );
}

#[test]
fn add_aieconlab_is_noop_when_claude_code_not_installed() {
    let temp = tempfile::tempdir().unwrap();
    let target = temp.path();
    prepare(target);

    run(target, &["install", "codex"], 0);
    run(target, &["add", "aieconlab"], 0);

    // No .claude/ adapter side-effects since claude-code wasn't installed.
    assert!(
        !target.join(".claude/agents/aieconlab-pi.md").exists(),
        "aieconlab claude-code adapter should be a no-op when claude-code isn't installed"
    );
    // CLAUDE.md should not exist either.
    assert!(!target.join("CLAUDE.md").exists());
}

#[test]
fn doctor_passes_after_install_then_add_aieconlab() {
    let temp = tempfile::tempdir().unwrap();
    let target = temp.path();
    prepare(target);

    run(target, &["install", "claude-code"], 0);
    run(target, &["add", "aieconlab"], 0);
    let out = run(target, &["doctor"], 0);
    let text = String::from_utf8_lossy(&out.stdout);
    assert!(
        text.contains("DOCTOR_STATUS=PASS"),
        "doctor not green:\n{text}"
    );
    // Spot-check three roles + slash command + managed block:
    assert!(text.contains("PASS .claude/agents/aieconlab-pi.md exists"));
    assert!(text.contains("PASS .claude/agents/aieconlab-llm-measurement.md exists"));
    assert!(text.contains("PASS .claude/agents/aieconlab-writer.md exists"));
    assert!(text.contains("PASS .claude/commands/aiel-route.md exists"));
    assert!(text.contains("PASS CLAUDE.md contains exactly one AiEconLab managed block"));
}

#[test]
fn uninstall_strips_aieconlab_block_and_preserves_user_content() {
    let temp = tempfile::tempdir().unwrap();
    let target = temp.path();
    prepare(target);

    let preexisting = "## My Notes\nKept content.\n";
    fs::write(target.join("CLAUDE.md"), preexisting).unwrap();

    run(target, &["install", "claude-code"], 0);
    run(target, &["add", "aieconlab"], 0);
    run(target, &["uninstall", "--yes"], 0);

    assert!(
        !target.join(".aiplus").exists(),
        ".aiplus should be removed"
    );

    let body = fs::read_to_string(target.join("CLAUDE.md")).unwrap();
    assert!(body.contains("## My Notes"), "user content lost:\n{body}");
    assert!(body.contains("Kept content."), "user content lost:\n{body}");
    assert!(
        !body.contains("<!-- BEGIN AIPLUS MANAGED BLOCK -->"),
        "AiPlus block should be removed on uninstall:\n{body}"
    );
    assert!(
        !body.contains("<!-- BEGIN AIECONLAB MANAGED BLOCK -->"),
        "AEL block should be removed on uninstall:\n{body}"
    );
    assert!(
        !body.contains("AiEconLab (AEL) is installed in this project"),
        "AEL block body should be removed on uninstall:\n{body}"
    );
}
