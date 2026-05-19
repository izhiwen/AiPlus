# Autoflow Coverage 1 Implementation Notes

Goal: G-AT-AUTOFLOW-COVERAGE-1
Worktree: `aiplus-public.autoflow-coverage`
Branch: `feat/autoflow-coverage`

## Phase 1 Design

### Ownership Boundary

Session A owns:

- `crates/aiplus-cli/src/mcp_server.rs`: descriptions for the 11 MCP tools not
  already covered by v0.6.9.
- `assets/aiplus-agent-team/adapters/<runtime>/skills/aiplus/SKILL.md`: add a
  `Use These Tools First` section covering all 14 MCP tools and six non-MCP
  feature categories.
- `crates/aiplus-cli/src/main.rs`: extend only the existing discovery preamble
  intent list.
- Tests and this implementation note.

Session B owns and this branch must not touch:

- `Dispatch Flow`
- `Multi-turn Patterns`
- Project-root preamble dispatch-flow subsection

Baseline note: v0.6.9 SKILL files have `Prefer MCP Tools Over CLI Subcommands`
and `Example Flows`, but no separate `Use These Tools First` heading. This
branch adds that section immediately after the MCP preference section and leaves
the existing example flows intact.

### MCP Tool Descriptions

Each description stays under 400 characters and keeps the three v0.6.9 tools
unchanged.

| Tool | Final description |
|---|---|
| `agent_route` | PREFERRED programmatic surface for dispatching a task to a role. Use this MCP tool instead of `aiplus agent route <role> "<task>"` CLI when the user asks to assign work. Writes dispatch history, active-role state, and any configured worktree artifacts. |
| `agent_status` | PREFERRED programmatic surface for team status queries. Use this MCP tool instead of `aiplus agent status` CLI when the user asks about team configuration, active roles, or current state. Returns structured JSON for active team, role state, and worktrees. |
| `agent_set_team` | PREFERRED programmatic surface for switching active teams. Use this MCP tool instead of `aiplus agent set-team <name>` CLI when the user asks to switch to agent-team or AiEconLab. Preserves team snapshots and returns structured JSON. |
| `agent_list` | PREFERRED programmatic surface for role roster queries. Use this MCP tool instead of `aiplus agent list` CLI when the user asks who is available, invited, disabled, or functional. Returns structured JSON for active-team roles. |
| `agent_doctor` | PREFERRED programmatic surface for agent health checks. Use this MCP tool instead of `aiplus agent doctor` CLI when the user asks whether the agent layer, personas, runtime mirrors, or catalogs are healthy. Returns structured JSON diagnostics. |
| `agent_invite` | PREFERRED programmatic surface for inviting a role into the active team. Use this MCP tool instead of `aiplus agent invite <role>` CLI when the user asks to bring in an expert or dormant role. Logs an Owner-visible event. |
| `agent_dismiss` | PREFERRED programmatic surface for dismissing a role from the active team. Use this MCP tool instead of `aiplus agent dismiss <role>` CLI when the user asks to remove an invited expert after scope completion. Logs the event. |
| `agent_disable` | PREFERRED programmatic surface for disabling a role. Use this MCP tool instead of `aiplus agent disable <role>` CLI when the user asks to prevent dispatch to a problematic role. Persists until explicitly re-enabled. |
| `agent_enable` | PREFERRED programmatic surface for re-enabling a disabled role. Use this MCP tool instead of `aiplus agent enable <role>` CLI when the user asks to restore a role for dispatch. Returns structured JSON status. |
| `agent_integrate` | PREFERRED programmatic surface for integrating a role worktree. Use this MCP tool instead of `aiplus agent integrate <role>` CLI when the user asks to merge a completed role branch back to main and prune its worktree. |
| `agent_talk` | PREFERRED programmatic surface for role-focused conversation setup. Use this MCP tool instead of `aiplus agent talk <role>` CLI when the user asks to talk with one role. Returns the command to run in a separate terminal. |

### SKILL.md `Use These Tools First`

Use one shared section for Claude Code, Codex, and OpenCode:

```markdown
## Use These Tools First

### Cost / spending / token usage (MCP tools, returns structured JSON)
- `agent_token_cost` - token + USD rollups (1h / 8h / 24h windows)

### Planning / task preview / scoring (MCP)
- `agent_route_score_only` - pre-flight a task to see staffing + risk

### Audit / log integrity (MCP)
- `agent_audit_verify_log` - verify dispatch log hash chain

### Dispatching / role management (MCP)
- `agent_route` - assign a task to a specific role and start work
- `agent_invite` - bring a role into the active team
- `agent_dismiss` - remove a role from the active team
- `agent_disable` / `agent_enable` - temporarily disable / re-enable a role
- `agent_integrate` - merge a role's worktree back
- `agent_talk` - single-turn chat setup with one role

### Team status / configuration (MCP)
- `agent_status` - current team status, active roles, recent activity
- `agent_list` - list all available roles
- `agent_set_team` - switch active team, e.g. to AiEconLab
- `agent_doctor` - agent-specific health checks

### Memory / context (non-MCP CLI, also preferred over shell grep)
- `aiplus memory record` - store project conventions / naming rules / facts
- `aiplus memory context --runtime <runtime>` - build memory context
- `aiplus memory status` - see what's in memory

### Compact / session token efficiency (non-MCP CLI)
- `aiplus compact prepare` - build a handoff capsule before /compact
- `aiplus compact resume` - restore state after /compact
- `aiplus compact savings` - see token + cost savings from compact-prep

### Velocity / time tracking (non-MCP CLI)
- `aiplus velocity estimate --task-type <type> --human-estimate <hours>` - log an estimate
- `aiplus velocity report` - see calibrated p50 / p90 from history

### Identity / commit signing (non-MCP CLI)
- `aiplus identity setup-signing [--dry-run]` - set up Mac Secure Enclave commit signing

### Doctor (cross-cutting health)
- `aiplus doctor [--quiet] [--check-keyring]` - full health check
```

### Preamble Intent List Extension

Keep the existing three high-priority bullets first. Add compact bullets for:

- dispatch/role actions -> MCP tools
- team state/configuration -> MCP tools
- memory/context -> `aiplus memory`
- compact/session efficiency -> `aiplus compact`
- velocity/time tracking -> `aiplus velocity`
- identity/signing -> `aiplus identity setup-signing`
- health -> `agent_doctor` or `aiplus doctor`

No dispatch-flow subsection is added in this branch.

### Tests

- Extend `agent_autoflow_discovery.rs` to assert installed skills and preambles
  contain all 14 MCP tools plus representative non-MCP commands.
- Extend `mcp_server.rs` tests so the 11 newly covered tool descriptions:
  - start with `PREFERRED programmatic surface`
  - mention the CLI alternative
  - stay under 400 characters
- Keep the v0.6.9 three-tool description test unchanged.
- Run `cargo test --workspace`, `cargo fmt --check`, and
  `cargo clippy --workspace --all-targets -- -D warnings`.

### CHANGELOG Draft

```markdown
## Unreleased

- Expand AiPlus autoflow discovery beyond cost, planning, and audit: all 14
  agent MCP tools now advertise their preferred programmatic use cases, runtime
  skills map common natural-language intents to the right MCP or AiPlus CLI
  surface, and installed project preambles include memory, compact, velocity,
  identity, health, team, and role-management guidance.
```

## Phase 3 Evidence

Implementation summary:

- Enhanced descriptions for the 11 pre-existing MCP tools:
  `agent_route`, `agent_status`, `agent_set_team`, `agent_list`,
  `agent_doctor`, `agent_invite`, `agent_dismiss`, `agent_disable`,
  `agent_enable`, `agent_integrate`, and `agent_talk`.
- Left the three v0.6.9 descriptions unchanged:
  `agent_token_cost`, `agent_audit_verify_log`, `agent_route_score_only`.
- Added `Use These Tools First` to all three runtime SKILL assets.
- Extended only the existing project-root preamble intent list. No dispatch-flow
  subsection was added.

Per-runtime file summary:

| Runtime | File | Change |
|---|---|---|
| Claude Code | `assets/aiplus-agent-team/adapters/claude-code/skills/aiplus/SKILL.md` | Added full 14-tool + six non-MCP category `Use These Tools First` section. |
| Codex | `assets/aiplus-agent-team/adapters/codex/skills/aiplus/SKILL.md` | Added full 14-tool + six non-MCP category `Use These Tools First` section. |
| OpenCode | `assets/aiplus-agent-team/adapters/opencode/skills/aiplus/SKILL.md` | Added full 14-tool + six non-MCP category `Use These Tools First` section. |

Focused content tests:

```text
cargo test -p aiplus-cli --test agent_autoflow_discovery
cargo test: 1 passed (1 suite, 4.91s)

cargo test -p aiplus-cli mcp_server::tests::
cargo test: 15 passed, 364 filtered out (43 suites, 0.02s)
```

Full gates:

```text
cargo test --workspace
cargo test: 582 passed, 1 ignored (48 suites, 235.96s)

cargo fmt --check
PASS

cargo clippy --workspace --all-targets -- -D warnings
cargo clippy: No issues found
```

Scope check:

```text
Modified files are limited to:
- mcp_server.rs descriptions/tests
- main.rs preamble intent list
- three runtime SKILL.md files
- agent_autoflow_discovery.rs content/idempotency assertions
- this impl-notes document

No CONTRACT, adapter code, token-cost subtree, scoring rubric, calibration
fixture, Cargo.toml version, CHANGELOG actual, or install.sh changes.

`Dispatch Flow` and `Multi-turn Patterns` appear only in this notes file's
ownership-boundary text, not in runtime SKILL files or preamble code.
```
