// Track B.1: AEL OpenCode adapter integration tests.
//
// Mirrors the v0.2 claude-code adapter checks but against
// `.opencode/agents/` and `.opencode/commands/`. Validates:
//   (1) all 20 prefixed agent files are written with YAML frontmatter
//   (2) all 4 slash commands land at .opencode/commands/
//   (3) bare-name files from mirror_personas_to_runtimes get cleaned
//   (4) no-op when opencode is not in runtimeAdapters
//   (5) doctor passes after install
//   (6) coexistence with claude-code adapter — both written in parallel
//   (7) uninstall cleans the new opencode prefixed files (regression
//       for the A.1 sweep extension)

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
fn add_aieconlab_writes_20_opencode_subagents_with_frontmatter() {
    let temp = tempfile::tempdir().unwrap();
    let target = temp.path();
    prepare(target);

    run(target, &["install", "opencode"], 0);
    run(target, &["add", "aieconlab"], 0);

    for role in CORE_ROLES.iter().chain(EXPERTS.iter()) {
        let path = target.join(format!(".opencode/agents/aieconlab-{role}.md"));
        assert!(
            path.exists(),
            "missing .opencode/agents/aieconlab-{role}.md"
        );
        let body = fs::read_to_string(&path).unwrap();
        let first_line = body.lines().next().unwrap_or("");
        assert_eq!(
            first_line, "---",
            "aieconlab-{role}.md missing YAML opening at .opencode/agents/"
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

    // Unprefixed bare mirror files should have been cleaned up.
    for role in CORE_ROLES {
        let unprefixed = target.join(format!(".opencode/agents/{role}.md"));
        assert!(
            !unprefixed.exists(),
            "unprefixed AEL persona .opencode/agents/{role}.md should have been removed"
        );
    }
}

#[test]
fn add_aieconlab_writes_4_opencode_slash_commands() {
    let temp = tempfile::tempdir().unwrap();
    let target = temp.path();
    prepare(target);

    run(target, &["install", "opencode"], 0);
    run(target, &["add", "aieconlab"], 0);

    for cmd in [
        "aiel-route",
        "aiel-talk",
        "aiel-fire-consultant",
        "aiel-status",
    ] {
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
fn add_aieconlab_opencode_adapter_is_noop_when_opencode_not_installed() {
    let temp = tempfile::tempdir().unwrap();
    let target = temp.path();
    prepare(target);

    run(target, &["install", "claude-code"], 0);
    run(target, &["add", "aieconlab"], 0);

    // claude-code adapter wrote .claude/agents/aieconlab-*.md but
    // .opencode/ should never exist on a claude-code-only project.
    assert!(target.join(".claude/agents/aieconlab-pi.md").exists());
    assert!(
        !target.join(".opencode/agents/aieconlab-pi.md").exists(),
        "opencode adapter should be a no-op when opencode isn't installed"
    );
}

#[test]
fn doctor_passes_after_install_opencode_with_aieconlab() {
    let temp = tempfile::tempdir().unwrap();
    let target = temp.path();
    prepare(target);

    run(target, &["install", "opencode"], 0);
    run(target, &["add", "aieconlab"], 0);
    let out = run(target, &["doctor"], 0);
    let text = String::from_utf8_lossy(&out.stdout);
    assert!(
        text.contains("DOCTOR_STATUS=PASS"),
        "doctor not green:\n{text}"
    );
}

#[test]
fn dual_runtime_install_writes_both_claude_and_opencode_adapters() {
    let temp = tempfile::tempdir().unwrap();
    let target = temp.path();
    prepare(target);

    // install all → claude-code + codex + opencode in runtimeAdapters.
    run(target, &["install", "all", "--yes"], 0);
    run(target, &["add", "aieconlab"], 0);

    // Both adapter dirs have the AEL roster.
    assert!(target.join(".claude/agents/aieconlab-pi.md").exists());
    assert!(target.join(".opencode/agents/aieconlab-pi.md").exists());

    // Both have the slash commands.
    assert!(target.join(".claude/commands/aiel-route.md").exists());
    assert!(target.join(".opencode/commands/aiel-route.md").exists());

    // CLAUDE.md still has the AEL block (claude-code path), and
    // AGENTS.aiplus.md still has the AIECONLAB_TEAM section
    // (opencode path leverages this — no separate block).
    let claude_md = fs::read_to_string(target.join("CLAUDE.md")).unwrap();
    assert!(claude_md.contains("<!-- BEGIN AIECONLAB MANAGED BLOCK -->"));
    let agents_aiplus = fs::read_to_string(target.join(".aiplus/AGENTS.aiplus.md")).unwrap();
    assert!(agents_aiplus.contains("AIECONLAB_TEAM"));
}

#[test]
fn uninstall_cleans_opencode_aieconlab_prefixed_files() {
    // Regression: A.1's `remove_runtime_adapter_artifacts` should
    // catch the new `.opencode/agents/aieconlab-*.md` files written
    // by B.1.
    let temp = tempfile::tempdir().unwrap();
    let target = temp.path();
    prepare(target);

    run(target, &["install", "opencode"], 0);
    run(target, &["add", "aieconlab"], 0);
    assert!(target.join(".opencode/agents/aieconlab-pi.md").exists());

    run(target, &["uninstall", "--yes"], 0);

    for role in CORE_ROLES.iter().chain(EXPERTS.iter()) {
        let path = target.join(format!(".opencode/agents/aieconlab-{role}.md"));
        assert!(
            !path.exists(),
            "uninstall should have removed .opencode/agents/aieconlab-{role}.md"
        );
    }
    for cmd in [
        "aiel-route",
        "aiel-talk",
        "aiel-fire-consultant",
        "aiel-status",
    ] {
        let path = target.join(format!(".opencode/commands/{cmd}.md"));
        assert!(
            !path.exists(),
            "uninstall should have removed .opencode/commands/{cmd}.md"
        );
    }
}
