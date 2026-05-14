// Track B.3: Codex adapter parity audit.
//
// Codex reads `.aiplus/` directly — there is no separate runtime
// adapter directory like `.claude/` or `.opencode/`. The single
// codex-visible surface is AGENTS.md, which carries a managed block
// pointing to `.aiplus/AGENTS.aiplus.md`. Each team's init appends a
// section (`AGENT_TEAM_TEAM`, `AIECONLAB_TEAM`) to that file via
// `append_team_section_to_agents_aiplus`.
//
// These tests verify that when both teams are installed, BOTH team
// sections survive in AGENTS.aiplus.md — closing the codex coexistence
// half of the B.3 audit. The corresponding A.2 cross-team residue
// cleanup affects `.aiplus/agents/` (TOML/persona files) but does NOT
// touch AGENTS.aiplus.md sections, so both stay advertised.

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
fn codex_install_writes_agents_md_managed_block_pointing_to_aiplus() {
    let temp = tempfile::tempdir().unwrap();
    let target = temp.path();
    prepare(target);

    run(target, &["install", "codex"], 0);

    let agents = fs::read_to_string(target.join("AGENTS.md")).unwrap();
    assert!(
        agents.contains("<!-- BEGIN AIPLUS MANAGED BLOCK -->"),
        "AGENTS.md should carry the AiPlus managed block (codex bridge):\n{agents}"
    );
    assert!(
        agents.contains("@./.aiplus/AGENTS.aiplus.md"),
        "managed block should reference .aiplus/AGENTS.aiplus.md:\n{agents}"
    );
}

#[test]
fn codex_dual_team_install_surfaces_both_team_sections() {
    // The acid test for B.3: with both teams installed, AGENTS.aiplus.md
    // must carry BOTH team-roster sections so a codex session reading
    // it sees the full virtual organization.
    let temp = tempfile::tempdir().unwrap();
    let target = temp.path();
    prepare(target);

    run(target, &["install", "codex"], 0);
    run(target, &["add", "aieconlab"], 0);

    let aiplus_md = fs::read_to_string(target.join(".aiplus/AGENTS.aiplus.md")).unwrap();

    // AGENT_TEAM_TEAM section came from agent-team's auto-install.
    assert!(
        aiplus_md.contains("<!-- BEGIN AGENT_TEAM_TEAM -->"),
        "AGENTS.aiplus.md missing AGENT_TEAM_TEAM section after dual install:\n{aiplus_md}"
    );
    assert!(
        aiplus_md.contains("<!-- END AGENT_TEAM_TEAM -->"),
        "AGENTS.aiplus.md AGENT_TEAM_TEAM has no end marker"
    );

    // AIECONLAB_TEAM section came from `add aieconlab`.
    assert!(
        aiplus_md.contains("<!-- BEGIN AIECONLAB_TEAM -->"),
        "AGENTS.aiplus.md missing AIECONLAB_TEAM section after dual install:\n{aiplus_md}"
    );
    assert!(
        aiplus_md.contains("<!-- END AIECONLAB_TEAM -->"),
        "AGENTS.aiplus.md AIECONLAB_TEAM has no end marker"
    );

    // Roster spot-check: each section should list at least the
    // hero role for that team in its body.
    let agent_team_idx = aiplus_md.find("<!-- BEGIN AGENT_TEAM_TEAM -->").unwrap();
    let agent_team_end = aiplus_md.find("<!-- END AGENT_TEAM_TEAM -->").unwrap();
    let agent_team_body = &aiplus_md[agent_team_idx..agent_team_end];
    assert!(
        agent_team_body.contains("CEO"),
        "AGENT_TEAM_TEAM body should mention the SWE CEO role:\n{agent_team_body}"
    );

    let aieconlab_idx = aiplus_md.find("<!-- BEGIN AIECONLAB_TEAM -->").unwrap();
    let aieconlab_end = aiplus_md.find("<!-- END AIECONLAB_TEAM -->").unwrap();
    let aieconlab_body = &aiplus_md[aieconlab_idx..aieconlab_end];
    assert!(
        aieconlab_body.contains("PI"),
        "AIECONLAB_TEAM body should mention the AEL PI role:\n{aieconlab_body}"
    );
}

#[test]
fn codex_install_does_not_write_claude_or_opencode_dirs() {
    // A codex-only install should leave .claude/ and .opencode/
    // untouched — runtime adapter isolation.
    let temp = tempfile::tempdir().unwrap();
    let target = temp.path();
    prepare(target);

    run(target, &["install", "codex"], 0);
    run(target, &["add", "aieconlab"], 0);

    assert!(
        !target.join(".claude/agents").exists(),
        "codex-only install should not create .claude/agents/"
    );
    assert!(
        !target.join(".opencode/agents").exists(),
        "codex-only install should not create .opencode/agents/"
    );
}

#[test]
fn codex_install_keeps_personas_under_aiplus_agents_for_direct_read() {
    // Codex reads `.aiplus/agents/personas/<role>.md` directly via
    // its filesystem permissions. After install, those persona files
    // must exist for the active team.
    let temp = tempfile::tempdir().unwrap();
    let target = temp.path();
    prepare(target);

    run(target, &["install", "codex"], 0);

    // Auto-installed agent-team is active.
    let personas = target.join(".aiplus/agents/personas");
    for role in [
        "advisor",
        "ceo",
        "architect",
        "pm",
        "engineer-a",
        "engineer-b",
        "reviewer",
        "qa",
    ] {
        assert!(
            personas.join(format!("{role}.md")).exists(),
            ".aiplus/agents/personas/{role}.md missing — codex cannot route to {role}"
        );
    }
}
