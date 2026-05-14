# Test coverage map

Track C.3 discipline: every closed known-limitation ships with a
regression test that would have caught the original bug. This file is
the audit index — when you close an issue or merge a PR that fixes a
known-limitation, append an entry here pointing at the regression
test that covers it.

Format: `<#PR> · <track/issue> · <one-line subject> → <test file>`

## Active coverage

### v0.5.18 — v0.5.20 (this sprint)

- #58 · Track A.1 · uninstall sweeps `.claude/{agents,commands}/` + `.opencode/aiplus-*` mirrors → `crates/aiplus-cli/tests/uninstall_hygiene.rs` (6 cases)
- #58 · Track A.1 · install.sh fallback ↔ Cargo.toml drift → `crates/aiplus-cli/tests/install_sh_fallback.rs`
- #60 · Track A.2 · `agent_team_init`/`aieconlab_init` clear OTHER team's exclusive files → `crates/aiplus-cli/tests/cross_team_residue.rs` (4 cases)
- #63 · Track B.1 · AEL OpenCode adapter v0.3 — subagents/commands/no-op/doctor/uninstall → `crates/aiplus-cli/tests/aieconlab_opencode.rs` (6 cases)
- #64 · Track B.2 · agent-team OpenCode adapter v0.2 → `crates/aiplus-cli/tests/agent_team_opencode.rs` (6 cases)
- #65 · Track B.3 · codex coexistence audit — AGENTS.aiplus.md dual-team sections → `crates/aiplus-cli/tests/codex_parity_audit.rs` (4 cases)
- #66 · Track C.1 · agent-team persona behavior cases structural sanity → `crates/aiplus-cli/tests/agent_team_persona_cases_structure.rs` (5 cases); LLM-driven runner at `tests/persona_behavior/test_persona_behavior.py`
- #67 · Track C.2 · end-to-end cross-runtime install matrix → `crates/aiplus-cli/tests/cross_runtime_install_matrix.rs`
- #69 · Track D.2 · CHANGELOG hygiene (Unreleased on top, reverse chronological, no duplicates, current-version section present) → `crates/aiplus-cli/tests/changelog_format.rs` (5 cases) — filled retroactively during Track C.3 audit
- #75 · Track A.3 · doctor stale-registry classified as INFO not NEEDS_FIX (#74) → `crates/aiplus-cli/tests/doctor_stale_registry_info.rs` (3 cases)

### v0.5.16 (prior-sprint headline fixes)

- #37 · Issue #35 · install.sh fallback bump + schemaVersion match extension → `crates/aiplus-cli/tests/install_sh_fallback.rs` + `crates/aiplus-cli/tests/aieconlab_claude_code.rs` (the latter covers the schemaVersion-supported doctor check transitively)
- #40 · Issue #31 · agent-team Claude Code adapter (frontmatter + slash commands) → `crates/aiplus-cli/tests/agent_team_claude_code.rs`
- #46 · parity test alignment for P1.3 dispatch-outcome → `crates/aiplus-cli/tests/parity.rs::agent_route_blocks_dispatch_on_unapproved_owner_gate` (rewritten in #46)
- #45 · Issue #32 + #33 + #34 · status filter / research-vocab / fresh-install compact → `crates/aiplus-cli/tests/cli_behavior_issues_32_33_34.rs` + unit tests in `crates/aiplus-cli/src/agent/state.rs::tests`

## Rules going forward (C.3 discipline)

1. Every PR that fixes a known-limitation lands with ≥ 1 new test
   that **would have failed against the bug**. If the test trivially
   passes against the old code, the test isn't real.
2. After merge, the PR description / commit body cites the test file
   inline AND a one-line entry gets appended to this map.
3. If a future CEO finds a track item with no regression coverage
   (like D.2 was during this sprint's audit), they file a Track C.3
   follow-up PR and add coverage. Don't let gaps stack.
4. This file is reviewed at every release: `## Unreleased` items in
   CHANGELOG should each have a row here before the release commit
   ships.
