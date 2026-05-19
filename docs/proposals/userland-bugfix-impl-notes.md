# Userland Bugfix Implementation Notes

Status: Phase 3 complete. Implementation and verification passed locally.

## 1. Reproduction Commands

### Bug 1: `mcp-register --runtime claude-code`

```text
cargo run -q -p aiplus-cli --bin aiplus -- mcp-register --runtime claude-code --dry-run
AIPLUS_UNEXPECTED_ERROR reason=uncaught detail=unknown --runtime 'claude-code'. Valid: codex, claude, opencode.
```

Root cause: `command_mcp_register` only matches `codex|claude|opencode`.

### Bug 2: top-level `doctor --quiet`

```text
cargo run -q -p aiplus-cli --bin aiplus -- doctor --quiet
error: unexpected argument '--quiet' found
```

Root cause: `aiplus agent doctor --quiet` exists and is wired through
`crates/aiplus-cli/src/agent/commands.rs` and `agent/doctor.rs`, but top-level
`aiplus doctor` has no `quiet` field in the `Commands::Doctor` clap variant and
`command_doctor` has no quiet parameter.

### Bug 3: OpenAI-key userland auto-summon fail-closed

```text
OPENAI_API_KEY=dummy aiplus agent route --score-only "实现支付接口"
Adaptive coordinator: complexity=5 risk=0.85 tier=HEAVY code_change=true design_impact=true consultant=fire
Plan step: would fire consultant for HEAVY task
Would staff: [pm,architect,engineer-a,engineer-b,reviewer,qa]
Forced by risk: [reviewer,qa]
```

No `Auto-summoned experts:` line appears. This reproduces the userland failure:
only `OPENAI_API_KEY` is present, but the classifier ignores it and silently
returns no match.

### Bug 4: `CODEX_HOME` ignored by `mcp-register`

```text
tmp=$(mktemp -d)
mkdir -p "$tmp/home/.codex" "$tmp/codex-home"
HOME="$tmp/home" CODEX_HOME="$tmp/codex-home" \
  ./target/debug/aiplus mcp-register --runtime codex --dry-run

MCP_REGISTER_CODEX=WOULD_WRITE path=$tmp/home/.codex/config.toml
```

Expected: `$tmp/codex-home/config.toml`.

## 2. Bug 3 Root Cause Findings

Code path:

- `route.rs` calls `coordinator::plan_task_for_project`.
- `plan_task_for_project` calls `apply_auto_summon`.
- `apply_auto_summon` reads `[autosummon] intent_hint` from installed expert TOML.
- `expert_intent_match` calls `classify_intent_match`.
- `classify_intent_match` has a mock path for tests, then reads only
  `ANTHROPIC_API_KEY`.

Root cause:

1. The classifier is wired, so this is not structural.
2. Real userland with only `OPENAI_API_KEY` always skips the classifier because
   `ANTHROPIC_API_KEY` is required.
3. All classifier failures collapse to `None`, then `unwrap_or(false)`, so the
   user gets no warning.
4. Existing tests set `AIPLUS_AUTOSUMMON_INTENT_MOCK=1`, so they never exercised
   the live provider-selection path.

Fix scope: keep scoring rubric locked. Change only intent classifier provider
selection/failure surfacing.

## 3. Fix Plan

Bug 1:

- Normalize `claude-code` to canonical `claude` in `mcp-register`.
- Update help/diagnostic text to list `claude-code` and `claude`.

Bug 4:

- Add `--config-dir <DIR>` to `mcp-register`.
- For codex global scope, resolve config dir as:
  1. explicit `--config-dir`
  2. `CODEX_HOME`
  3. `$HOME/.codex`
- For claude global scope, resolve config dir as:
  1. explicit `--config-dir`
  2. `CLAUDE_CONFIG_DIR`
  3. `$HOME/.claude`
- Project scope remains project-local unless `--config-dir` is provided.

Bug 2:

- Add `quiet: bool` to top-level `Commands::Doctor`.
- Thread it to `command_doctor(fix, quiet)`.
- Suppress `INFO` severity lines while keeping PASS/NEEDS_FIX and
  `DOCTOR_STATUS`.

Bug 3:

- Introduce an intent classifier outcome type: match/no-match/skipped/failed.
- Prefer Anthropic when `ANTHROPIC_API_KEY` is available; otherwise use OpenAI
  when `OPENAI_API_KEY` is available.
- Add OpenAI Chat Completions call path for `OPENAI_API_KEY`.
- Preserve mock path for deterministic tests.
- Surface missing-key or request/parse failures as score-only/route warning lines
  and in the coordinator decision JSON.

## 4. Test Plan

Bug 1/4:

- `mcp-register --runtime claude-code --dry-run` succeeds.
- `mcp-register --runtime claude --dry-run` still succeeds.
- invalid runtime still errors.
- `CODEX_HOME=/tmp/x ... --runtime codex --dry-run` reports `/tmp/x/config.toml`.
- `CLAUDE_CONFIG_DIR=/tmp/x ... --runtime claude-code --dry-run --scope global`
  reports `/tmp/x/.mcp.json`.
- `--config-dir /tmp/x` overrides both env and default.

Bug 2:

- `aiplus doctor --quiet` runs.
- `aiplus doctor --help` lists `--quiet`.
- quiet output contains no `INFO ` lines.
- quiet output still contains `DOCTOR_STATUS=...`.

Bug 3:

- Unit test provider selection with `OPENAI_API_KEY` and mocked HTTP endpoint
  using `AIPLUS_AUTOSUMMON_INTENT_URL=file://...`.
- Integration smoke: userland score-only with OpenAI-key path and mock response
  returns `Auto-summoned experts: [security-reviewer]`.
- Missing-key path emits visible `Autosummon intent warning:` line.

Full gates:

- `cargo fmt --check`
- `cargo test --workspace`
- `cargo clippy --workspace --all-targets -- -D warnings`

Actual local gate used for this bugfix:

- `cargo test` (workspace default): PASS, 579 passed, 1 ignored.
- `cargo fmt --check`: PASS.
- `cargo clippy --workspace --all-targets -- -D warnings`: PASS.

## 5. CHANGELOG Draft

```markdown
## Unreleased

- Fix userland setup bugs: `mcp-register --runtime claude-code` is accepted,
  `mcp-register` honors `CODEX_HOME` / `CLAUDE_CONFIG_DIR` and explicit
  `--config-dir`, top-level `aiplus doctor --quiet` works, and autosummon intent
  classification supports OpenAI-key environments with visible failure warnings.
```

## 6. Phase 3 Evidence

Bug 1 fixed:

```text
./target/debug/aiplus mcp-register --runtime claude-code --dry-run
MCP_REGISTER_CLAUDE=WOULD_WRITE path=/Users/steve/Projects/AiPlus/aiplus-public.userland-bugfix/.mcp.json
MCP_REGISTER_STATUS=DRY_RUN_OK any_change=true
```

Bug 4 fixed:

```text
HOME=$tmp/home CODEX_HOME=$tmp/codex ./target/debug/aiplus mcp-register --runtime codex --dry-run
MCP_REGISTER_CODEX=WOULD_WRITE path=$tmp/codex/config.toml
MCP_REGISTER_STATUS=DRY_RUN_OK any_change=true
```

Bug 2 fixed:

```text
./target/debug/aiplus doctor --quiet
NEEDS_FIX .aiplus/manifest.json exists
NEEDS_FIX manifest parses
NEEDS_FIX manifest installer is aiplus (manifest missing or invalid)
NEEDS_FIX manifest schemaVersion supported (manifest missing or invalid)
NEEDS_FIX .aiplus/AGENTS.aiplus.md exists
NEEDS_FIX .aiplus/REFRESH_PROMPT.txt exists
NEEDS_FIX nl_role_triggers=FAIL_MISSING_AGENTS_CATALOG (run `aiplus install <runtime>` to refresh managed role-trigger catalog)
DOCTOR_STATUS=NEEDS_FIX
```

Bug 3 fixed:

```text
OPENAI_API_KEY=test AIPLUS_AUTOSUMMON_INTENT_URL=file://$tmp/openai-yes.json \
  ./target/debug/aiplus agent route --score-only "实现支付接口"
Would staff: [pm,architect,engineer-a,engineer-b,reviewer,qa,security-reviewer,ai-integration]
Auto-summoned experts: [security-reviewer,ai-integration]
```

Regression tests:

```text
cargo test -p aiplus-cli --test userland_bugfix
cargo test: 5 passed (1 suite, 0.64s)

cargo test
cargo test: 579 passed, 1 ignored (47 suites, 41.49s)

cargo fmt --check
PASS

cargo clippy --workspace --all-targets -- -D warnings
cargo clippy: No issues found
```
