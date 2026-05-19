# Autoflow Multi-turn 1 Implementation Notes

## Phase 1 Plan

Goal: add explicit multi-turn dispatch guidance to installed AiPlus runtime
skills and project-root discovery preambles without changing Session A's
coverage work.

## Ownership Boundary

Read:

- `docs/decisions/autoflow-multiturn-1-impl-briefing.md`
- `docs/proposals/goal-G-AT-AUTOFLOW-MULTITURN-1.md`
- `docs/proposals/goal-G-AT-AUTOFLOW-COVERAGE-1.md`

Session B owns only:

- `assets/aiplus-agent-team/adapters/<runtime>/skills/aiplus/SKILL.md`:
  append `Dispatch Flow` and `Multi-turn Patterns`.
- `crates/aiplus-cli/src/main.rs`: append the dispatch-flow paragraph to the
  discovery preamble body after the intent list.
- Tests and this implementation note.

Do not touch Session A surfaces:

- `mcp_server.rs`
- SKILL.md `Use These Tools First`
- Preamble intent list

Do not touch forbidden files: CONTRACT, adapters, token-cost subtree, scoring,
calibration, Cargo.toml version, CHANGELOG actual, install.sh.

## SKILL.md Dispatch Flow Section

Final text:

```markdown
## Dispatch Flow

For non-trivial coding tasks, do not jump straight to implementation advice.
Use the AiPlus team dispatch loop:

1. Preview: call `agent_route_score_only` with the user's current task.
2. Surface: report tier, complexity, risk, would-staff roles, forced-by-risk
   roles, and auto-summoned experts.
3. Confirm: ask whether the user wants to dispatch that staffed team.
4. Dispatch: on "yes", "go", or equivalent, call `agent_route` with the task.
5. Integrate: when role work completes, call `agent_integrate <role>` for each
   completed role, then verify with `agent_status`.

Example:

User: "Help me refactor the user authentication module to support OAuth2."
You: call `agent_route_score_only` with
task="refactor user authentication module to support OAuth2".
Then say: "Coordinator scored this MEDIUM tier (complexity 3, risk 0.6).
Would staff engineer-a, reviewer, and security-reviewer. Proceed?"
User: "Yes."
You: call `agent_route` with the same task and report the dispatched roles.
Later, when work is ready, call `agent_integrate <role>` per completed role.
```

Rationale: compact enough for runtime instructions, but explicit about the
score-only first move, user confirmation, route dispatch, and integration.

## SKILL.md Multi-turn Patterns Section

Final text:

```markdown
## Multi-turn Patterns

### Follow-up Cost Question

Turn 1 user: "How much have I spent today?"
Turn 1 you: call `agent_token_cost`; report the total.
Turn 2 user: "What about by role?"
Turn 2 you: call `agent_token_cost` again with `by_role=true`; report per-role.

Do not grep dispatch logs between turns. MCP calls fetch fresh data.

### Mid-flight Scope Change

Turn 1 user: "Plan a payment API for me."
Turn 1 you: call `agent_route_score_only`, surface the plan, and ask whether to
dispatch.
Turn 2 user: "Actually, change it to refunds instead."
Turn 2 you: call `agent_route_score_only` with the new refunds task. Do not
dispatch the old payment task.

### Ambiguous Audit Intent

User: "Audit my project."
If multiple tools could apply, ask which audit they mean:

- code/runtime health: `agent_doctor`
- dispatch log integrity: `agent_audit_verify_log`
- current team or work state: `agent_status`

If unsure between two tools, list the options and ask the user to pick. Do not
silently call the wrong tool.
```

Rationale: covers follow-up refinement, changed scope, and ambiguous wording
without adding new protocol or tool behavior.

## Preamble Append Paragraph

Final paragraph, appended after the preamble intent list:

```markdown
**Dispatch flow**: For non-trivial coding tasks, do not answer directly. First
call `agent_route_score_only` to preview staffing, surface the plan and ask for
confirmation, then call `agent_route` only after confirm. When role work is
ready, call `agent_integrate <role>` and verify with `agent_status`.
```

It is four lines in source and does not edit Session A's intent list.

## Idempotency Plan

The install path already replaces the managed discovery block and overwrites
runtime skill assets from source templates. Re-running `aiplus install all
--yes` should therefore leave exactly one discovery managed block per preamble
file and one copy of each new SKILL.md section per installed skill file.

Tests will assert:

- each installed SKILL.md contains one `## Dispatch Flow`
- each installed SKILL.md contains one `## Multi-turn Patterns`
- each managed preamble contains one `**Dispatch flow**`
- the discovery begin/end sentinels still occur once after two install runs

## Test Plan

- `cargo test -p aiplus-cli --test agent_autoflow_discovery`
- `cargo test`
- `cargo fmt --check`
- `cargo clippy --workspace --all-targets -- -D warnings`

## CHANGELOG Draft

```markdown
## Unreleased

- Add multi-turn AiPlus autoflow guidance to installed runtime skills and
  project-root preambles, covering score-only preview, user confirmation,
  dispatch, integration, follow-up cost questions, scope changes, and ambiguous
  audit prompts.
```

## Phase 3 Evidence

Changed files:

- `assets/aiplus-agent-team/adapters/claude-code/skills/aiplus/SKILL.md`
- `assets/aiplus-agent-team/adapters/codex/skills/aiplus/SKILL.md`
- `assets/aiplus-agent-team/adapters/opencode/skills/aiplus/SKILL.md`
- `crates/aiplus-cli/src/main.rs`
- `crates/aiplus-cli/tests/agent_autoflow_discovery.rs`
- `docs/proposals/autoflow-multiturn-1-impl-notes.md`

Content verification:

- each runtime SKILL.md has one `## Dispatch Flow`
- each runtime SKILL.md has one `## Multi-turn Patterns`
- installed skill test verifies the sections after two `aiplus install all
  --yes` runs
- managed preamble test verifies one `**Dispatch flow**` paragraph after two
  install runs
- existing discovery begin/end idempotency assertions remain in place

Commands:

```text
cargo test -p aiplus-cli --test agent_autoflow_discovery
cargo test: 1 passed (1 suite, 7.30s)

cargo test
cargo test: 581 passed, 1 ignored (48 suites, 305.79s)

cargo fmt --check
PASS

cargo clippy --workspace --all-targets -- -D warnings
cargo clippy: No issues found
```

Scope check:

```text
No changes to mcp_server.rs, CONTRACT, adapters, token-cost subtree, scoring,
calibration, Cargo.toml version, CHANGELOG actual, or install.sh.
Existing v0.6.9 SKILL.md sections were left intact; new sections were appended
after existing content.
```
