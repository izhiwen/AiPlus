// Track A.1: Uninstall hygiene regression tests.
//
// Before v0.5.17, `aiplus uninstall --yes` left runtime-adapter
// artifacts behind:
//   .claude/agents/aieconlab-*.md   (~20 files when AEL installed)
//   .claude/agents/agent-team-*.md  (~14 files when agent-team installed)
//   .claude/agents/aiplus-*.md      (5 files from claude-code adapter)
//   .claude/commands/aiel-*.md
//   .claude/commands/aiplus-*.md
//   .claude/commands/at-*.md
//   .opencode/agents/aiplus-*.md
//   .opencode/commands/aiplus-*.md
//   .opencode/prompts/aiplus*.md
//
// `remove_runtime_adapter_artifacts` now sweeps these. Tests assert:
//   (1) all matching prefixed files are gone after uninstall
//   (2) user-authored files at the same paths (different prefix) survive
//   (3) empty parent dirs are pruned but non-empty ones are preserved
//   (4) idempotent — re-running uninstall on a partial state is safe

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

#[test]
fn uninstall_cleans_claude_agents_and_commands_for_both_teams() {
    let temp = tempfile::tempdir().unwrap();
    let target = temp.path();
    prepare(target);

    run(target, &["install", "claude-code"], 0);
    run(target, &["add", "aieconlab"], 0);

    // Sanity: prefixed files exist before uninstall.
    assert!(target.join(".claude/agents/aieconlab-pi.md").exists());
    assert!(target.join(".claude/agents/agent-team-ceo.md").exists());
    assert!(target.join(".claude/agents/aiplus-advisor.md").exists());
    assert!(target.join(".claude/commands/aiel-route.md").exists());
    assert!(target.join(".claude/commands/aiplus-refresh.md").exists());
    assert!(target.join(".claude/commands/at-status.md").exists());

    run(target, &["uninstall", "--yes"], 0);

    // After uninstall: every prefixed adapter file we wrote should be gone.
    for missing in [
        ".claude/agents/aieconlab-pi.md",
        ".claude/agents/aieconlab-theorist.md",
        ".claude/agents/aieconlab-writer.md",
        ".claude/agents/agent-team-ceo.md",
        ".claude/agents/agent-team-architect.md",
        ".claude/agents/agent-team-engineer-a.md",
        ".claude/agents/aiplus-advisor.md",
        ".claude/agents/aiplus-memory.md",
        ".claude/agents/aiplus-compact.md",
        ".claude/commands/aiel-route.md",
        ".claude/commands/aiel-status.md",
        ".claude/commands/aiplus-refresh.md",
        ".claude/commands/aiplus-route.md",
        ".claude/commands/at-route.md",
        ".claude/commands/at-status.md",
    ] {
        assert!(
            !target.join(missing).exists(),
            "uninstall should have removed {missing}"
        );
    }
}

#[test]
fn uninstall_preserves_user_authored_files_in_claude_dirs() {
    let temp = tempfile::tempdir().unwrap();
    let target = temp.path();
    prepare(target);

    run(target, &["install", "claude-code"], 0);
    run(target, &["add", "aieconlab"], 0);

    // User adds their own subagent + slash command BEFORE uninstall.
    // These use names that do NOT match any of our owned prefixes.
    let user_agent = target.join(".claude/agents/my-personal.md");
    fs::write(
        &user_agent,
        "---\nname: my-personal\ndescription: my own subagent\n---\n\nBody.\n",
    )
    .unwrap();
    let user_command = target.join(".claude/commands/scratch.md");
    fs::write(&user_command, "# /scratch — user command body\n").unwrap();

    run(target, &["uninstall", "--yes"], 0);

    assert!(
        user_agent.exists(),
        "user-authored .claude/agents/my-personal.md must survive uninstall"
    );
    assert!(
        user_command.exists(),
        "user-authored .claude/commands/scratch.md must survive uninstall"
    );

    // And ours are gone:
    assert!(!target.join(".claude/agents/aieconlab-pi.md").exists());
    assert!(!target.join(".claude/commands/aiplus-refresh.md").exists());
}

#[test]
fn uninstall_prunes_empty_claude_dirs_single_team() {
    // Single-team install: agent-team is auto-installed by
    // `install claude-code`. No `aieconlab` add, so the cross-team
    // bare-mirror residue tracked in Track A.2 doesn't apply here —
    // uninstall should fully prune both `.claude/agents/` and
    // `.claude/commands/`.
    let temp = tempfile::tempdir().unwrap();
    let target = temp.path();
    prepare(target);

    run(target, &["install", "claude-code"], 0);
    run(target, &["uninstall", "--yes"], 0);

    assert!(
        !target.join(".claude/agents").exists(),
        ".claude/agents/ should be pruned when empty after uninstall (single-team)"
    );
    assert!(
        !target.join(".claude/commands").exists(),
        ".claude/commands/ should be pruned when empty after uninstall (single-team)"
    );
}

#[test]
fn uninstall_dual_team_leaves_only_bare_mirror_orphans_in_agents() {
    // Dual-team install: agent-team (auto) + add aieconlab. Currently
    // mirror_personas_to_runtimes writes bare role-name files
    // (architect.md, ceo.md, engineer-a.md, engineer-b.md, qa.md,
    // reviewer.md) from agent-team personas left in
    // `.aiplus/agents/personas/` because aieconlab_init does not
    // clear the other team's personas first. Track A.2 closes that;
    // until then, uninstall provably cleans every prefixed file we
    // own, and leaves only the bare-mirror residue.
    let temp = tempfile::tempdir().unwrap();
    let target = temp.path();
    prepare(target);

    run(target, &["install", "claude-code"], 0);
    run(target, &["add", "aieconlab"], 0);
    run(target, &["uninstall", "--yes"], 0);

    // Every prefixed adapter file we wrote is gone.
    let agents_dir = target.join(".claude/agents");
    if agents_dir.exists() {
        for entry in fs::read_dir(&agents_dir).unwrap().flatten() {
            let name = entry.file_name().to_string_lossy().into_owned();
            for prefix in ["aieconlab-", "agent-team-", "aiplus-"] {
                assert!(
                    !name.starts_with(prefix),
                    "uninstall should have removed prefixed agent {name}"
                );
            }
        }
    }
    // `.claude/commands/` had no bare-mirror residue, so it should
    // still prune fully.
    assert!(
        !target.join(".claude/commands").exists(),
        ".claude/commands/ should be pruned when empty after uninstall"
    );
}

#[test]
fn uninstall_keeps_non_empty_claude_dir_with_user_content() {
    let temp = tempfile::tempdir().unwrap();
    let target = temp.path();
    prepare(target);

    run(target, &["install", "claude-code"], 0);

    // User content in .claude/agents/ — non-prefixed.
    fs::write(
        target.join(".claude/agents/my-personal.md"),
        "---\nname: my-personal\ndescription: my own\n---\n\nBody.\n",
    )
    .unwrap();

    run(target, &["uninstall", "--yes"], 0);

    assert!(
        target.join(".claude/agents/my-personal.md").exists(),
        "user file must survive"
    );
    assert!(
        target.join(".claude/agents").exists(),
        ".claude/agents/ must NOT be pruned because user file remains"
    );
}

#[test]
fn uninstall_is_idempotent_when_run_twice() {
    let temp = tempfile::tempdir().unwrap();
    let target = temp.path();
    prepare(target);

    run(target, &["install", "claude-code"], 0);
    run(target, &["uninstall", "--yes"], 0);

    // Second uninstall must not error even though .aiplus/ is gone
    // and the adapter artifacts are already removed. We expect a
    // non-zero exit because uninstall on a missing AiPlus install
    // surfaces a clear error, but the binary itself must not panic.
    let mut command = Command::new(bin());
    command
        .args(["uninstall", "--yes"])
        .current_dir(target)
        .env("HOME", target.join("fake-home"))
        .env("CODEX_HOME", target.join("fake-codex-home"))
        .env("XDG_CONFIG_HOME", target.join("fake-xdg"));
    let output = command.output().expect("second uninstall must not panic");
    // Either exits 0 (idempotent OK) or exits non-zero with a clear
    // "nothing to remove" message — but NEVER a Rust panic
    // (exit-code 101) or signal-killed (None).
    assert!(
        output.status.code().is_some(),
        "second uninstall was killed by a signal — stdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert_ne!(
        output.status.code(),
        Some(101),
        "second uninstall panicked — stderr:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );
}
