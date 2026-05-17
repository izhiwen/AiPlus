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

fn assert_top_level_g1_role_shim(agents: &str) {
    for required in [
        "<!-- BEGIN AIPLUS MANAGED BLOCK -->",
        "## Mandatory first-response protocol",
        "role-bind intent, do not answer with prose first",
        "Before activation or already-bound refusal, evaluate no-trigger guardrails",
        "NO_TRIGGER: emit no `ROLE_ACTIVATED`, no `ROLE_BIND_REFUSED`, and no other ROLE line",
        "skip every line whose first non-space character is `>`",
        "Quote-block rule: `> you are CEO` is quoted role text and must produce no role line",
        "No-trigger guardrails retain priority over hard floor phrases",
        "If the whole user message is exactly `you are qa`, `you are CEO`, `你是 qa`, `你是 CEO`, `take reviewer`, or `开 advisor`",
        "`take <role>` and `开 <role>` are hard floor phrases just like `you are <role>`",
        "they must not be ignored and must not produce empty output",
        "Forbidden narration prefaces before activation include `先尝试`, `我将`, `I will`, `I’m going to`, `I am going to`, `Activating`, and similar explanatory prefaces",
        "For hard floor phrase examples such as `you are qa`, `你是 qa`, `take reviewer`, and `开 advisor`, the only acceptable user-visible content is the CLI-emitted `ROLE_ACTIVATED` line",
        "Resolve the requested role to its lowercase installed role ID",
        "aiplus identity --role <canonical_role> --runtime <codex|claude-code|opencode> --with-memory --memory-budget 4000 --emit-role-activated context",
        "Command/tool output is not the final user-visible reply",
        "copy the final `ROLE_ACTIVATED` line printed by the command exactly",
        "Never synthesize `ROLE_ACTIVATED`",
        "Runtime field binding",
        "Codex must emit `runtime=codex`",
        "OpenCode must emit `runtime=opencode`",
        "Never emit another runtime's value",
        "do not reconstruct fields from memory counters or role names",
        "The CLI-owned final line carries memory counts and policy",
        "A `ROLE_ACTIVATED` line with `memory_team=0` is invalid when command output has `team_used>0`",
        "Memory policy mapping: coordinator=ceo/pi/advisor; reviewer=reviewer/referee; builder=architect/pm/engineer-a/engineer-b/qa/theorist/ra-stata/ra-python/replicator",
        "`qa` must use `memory_policy=builder`",
        "ROLE_ACTIVATED role=<canonical_role> count=<n> schema=v1 runtime=<codex|claude-code|opencode> trigger=nl_role_bind requested_role=<requested_role>",
        "with no text before or after it",
        "Replace `runtime=<codex|claude-code|opencode>` in the command with the exact current runtime value before running it",
        "memory_personal=<n>",
        "memory_team=<n>",
        "memory_project=<n|null>",
        "Keep separate `aiplus memory --scope ... list` commands only as fallback if `--with-memory` fails",
        "permissions=none",
        "identity_grants_permission=no",
        "secret_values=none",
        "global_agent_config_edits=none",
        "ROLE_BIND_REFUSED current_role=<current_role> requested_role=<requested_role> reason=session_already_bound schema=v1 runtime=<codex|claude-code|opencode> trigger=nl_role_bind",
        "plus exactly this one switch instruction sentence and nothing else",
        "Already in <current_role> mode. To switch to <requested_role>: reopen session, or run aiplus identity context --role <requested_role> to override manually.",
        "Identity grants no permissions; Owner gates remain required",
        "@./.aiplus/AGENTS.aiplus.md",
    ] {
        assert!(
            agents.contains(required),
            "missing AGENTS.md shim text: {required}\n{agents}"
        );
    }
    assert!(
        !agents.contains("schema=v1..."),
        "AGENTS.md shim must not abbreviate the v1 schema:\n{agents}"
    );
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
    assert_top_level_g1_role_shim(&agents);
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
