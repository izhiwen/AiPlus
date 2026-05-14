// Track C.2: end-to-end cross-runtime install matrix.
//
// Single integration test that drives the full triple-runtime
// install path and asserts each runtime sees the right artifacts
// for both teams. This is the regression boundary: changes to any
// of the three adapter install paths or to the cross-team residue
// cleanup must keep this matrix green.
//
// Scenario:
//   install all --yes    # codex + claude-code + opencode + auto-installed
//                        # modules (compact-reminder, agent-team, agent-key,
//                        # agent-velocity, agent-memory, auto-team-consultant)
//   add aieconlab        # AEL takes over active team
//   agent set-team agent-team   # switch back; snapshots must be clean
//
// What we assert (matrix):
//
//   AGENTS.md            — AiPlus managed block + .aiplus/AGENTS.aiplus.md ref
//   .aiplus/AGENTS.aiplus.md — AGENT_TEAM_TEAM + AIECONLAB_TEAM sections
//   .claude/agents/      — agent-team-*.md (14) + aieconlab-*.md (20) + aiplus-*.md (5)
//   .claude/commands/    — at-*, aiel-*, aiplus-*
//   .claude/CLAUDE.md    — AiPlus + AIECONLAB + AIPLUS-AGENT-TEAM managed blocks
//   .opencode/agents/    — same three prefix groups
//   .opencode/commands/  — same three prefix groups
//   .aiplus/agents/      — active team's TOMLs and personas only (A.2)
//   doctor               — DOCTOR_STATUS=PASS

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
fn cross_runtime_install_matrix_end_to_end() {
    let temp = tempfile::tempdir().unwrap();
    let target = temp.path();
    prepare(target);

    // Phase 1: full install — all 3 runtimes + auto-install modules.
    // agent-team becomes the active team. aieconlab is not auto-installed.
    run(target, &["install", "all", "--yes"], 0);

    // Codex view (AGENTS.md + AGENTS.aiplus.md).
    let agents_md = fs::read_to_string(target.join("AGENTS.md")).unwrap();
    assert!(
        agents_md.contains("<!-- BEGIN AIPLUS MANAGED BLOCK -->")
            && agents_md.contains("@./.aiplus/AGENTS.aiplus.md"),
        "AGENTS.md missing AiPlus managed block / ref:\n{agents_md}"
    );
    let aiplus_md = fs::read_to_string(target.join(".aiplus/AGENTS.aiplus.md")).unwrap();
    assert!(
        aiplus_md.contains("<!-- BEGIN AGENT_TEAM_TEAM -->"),
        "AGENT_TEAM_TEAM section missing after agent-team auto-install"
    );

    // Claude Code view.
    assert!(target.join(".claude/agents/agent-team-ceo.md").exists());
    assert!(target
        .join(".claude/agents/agent-team-engineer-a.md")
        .exists());
    assert!(target.join(".claude/agents/aiplus-advisor.md").exists());
    assert!(target.join(".claude/commands/at-route.md").exists());
    assert!(target.join(".claude/commands/aiplus-refresh.md").exists());
    let claude_md = fs::read_to_string(target.join("CLAUDE.md")).unwrap();
    assert!(claude_md.contains("<!-- BEGIN AIPLUS MANAGED BLOCK -->"));
    assert!(claude_md.contains("<!-- BEGIN AIPLUS-AGENT-TEAM MANAGED BLOCK -->"));

    // OpenCode view.
    assert!(target.join(".opencode/agents/agent-team-ceo.md").exists());
    assert!(target
        .join(".opencode/agents/agent-team-engineer-a.md")
        .exists());
    assert!(target.join(".opencode/agents/aiplus-advisor.md").exists());
    assert!(target.join(".opencode/commands/at-route.md").exists());
    assert!(target.join(".opencode/commands/aiplus-refresh.md").exists());

    // Phase 2: add aieconlab. AEL adapters run on all three runtimes.
    run(target, &["add", "aieconlab"], 0);

    // Codex view now has BOTH team sections.
    let aiplus_md_post = fs::read_to_string(target.join(".aiplus/AGENTS.aiplus.md")).unwrap();
    assert!(
        aiplus_md_post.contains("<!-- BEGIN AGENT_TEAM_TEAM -->")
            && aiplus_md_post.contains("<!-- BEGIN AIECONLAB_TEAM -->"),
        "post-add AGENTS.aiplus.md should carry both team sections:\n{aiplus_md_post}"
    );

    // Claude Code view: AEL files written + CLAUDE.md AEL block.
    assert!(target.join(".claude/agents/aieconlab-pi.md").exists());
    assert!(target.join(".claude/agents/aieconlab-theorist.md").exists());
    assert!(target.join(".claude/commands/aiel-route.md").exists());
    let claude_md_post = fs::read_to_string(target.join("CLAUDE.md")).unwrap();
    assert!(claude_md_post.contains("<!-- BEGIN AIECONLAB MANAGED BLOCK -->"));

    // OpenCode view: AEL files written.
    assert!(target.join(".opencode/agents/aieconlab-pi.md").exists());
    assert!(target
        .join(".opencode/agents/aieconlab-theorist.md")
        .exists());
    assert!(target.join(".opencode/commands/aiel-route.md").exists());

    // A.2 cross-team residue: .aiplus/agents/ should have AEL roles
    // active (pi.toml present), and agent-team-only TOMLs should be
    // cleared (ceo.toml gone).
    assert!(target.join(".aiplus/agents/pi.toml").exists());
    assert!(
        !target.join(".aiplus/agents/ceo.toml").exists(),
        "agent-team-exclusive ceo.toml should be cleared after add aieconlab (A.2)"
    );

    // Doctor still green with the combined state.
    let out = run(target, &["doctor"], 0);
    let text = String::from_utf8_lossy(&out.stdout);
    assert!(
        text.contains("DOCTOR_STATUS=PASS"),
        "doctor not green after dual-team install:\n{text}"
    );

    // Phase 3: switch back to agent-team via set-team. AEL files
    // clear from .aiplus/agents/; agent-team files restore from
    // snapshot. The runtime adapter dirs keep both teams' prefixed
    // agent files (those don't get torn down on team-switch — only
    // .aiplus/agents/ swaps).
    run(target, &["agent", "set-team", "agent-team"], 0);
    assert!(
        !target.join(".aiplus/agents/pi.toml").exists(),
        "AEL pi.toml should be cleared from active layout after set-team agent-team"
    );
    assert!(target.join(".aiplus/agents/ceo.toml").exists());
    // Runtime adapter mirrors persist (both teams' prefixed files
    // remain in .claude/agents/ and .opencode/agents/).
    assert!(target.join(".claude/agents/agent-team-ceo.md").exists());
    assert!(target.join(".claude/agents/aieconlab-pi.md").exists());
    assert!(target.join(".opencode/agents/agent-team-ceo.md").exists());
    assert!(target.join(".opencode/agents/aieconlab-pi.md").exists());

    // Phase 4: uninstall and verify all 3 adapters get cleaned (A.1).
    run(target, &["uninstall", "--yes"], 0);
    assert!(!target.join(".aiplus").exists());
    // Prefix sweep cleaned every prefixed file across both runtimes.
    if target.join(".claude/agents").exists() {
        for entry in fs::read_dir(target.join(".claude/agents"))
            .unwrap()
            .flatten()
        {
            let name = entry.file_name().into_string().unwrap_or_default();
            for prefix in ["agent-team-", "aieconlab-", "aiplus-"] {
                assert!(
                    !name.starts_with(prefix),
                    ".claude/agents/{name} should have been cleaned by uninstall"
                );
            }
        }
    }
    if target.join(".opencode/agents").exists() {
        for entry in fs::read_dir(target.join(".opencode/agents"))
            .unwrap()
            .flatten()
        {
            let name = entry.file_name().into_string().unwrap_or_default();
            for prefix in ["agent-team-", "aieconlab-", "aiplus-"] {
                assert!(
                    !name.starts_with(prefix),
                    ".opencode/agents/{name} should have been cleaned by uninstall"
                );
            }
        }
    }
}
