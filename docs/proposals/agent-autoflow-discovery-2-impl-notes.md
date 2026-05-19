# Agent Autoflow Discovery v2 Implementation Notes

Status: Phase 3 complete. Multi-runtime live test produced 6/6 expected MCP
triggers; Codex non-interactive cancelled after tool start, OpenCode completed
all three tool calls.

## 1. v1 File Paths Confirmed

The v1 discovery layer exists on main at `651b25b` and writes:

- `assets/aiplus-agent-team/adapters/claude-code/skills/aiplus/SKILL.md`
- `assets/aiplus-agent-team/adapters/codex/skills/aiplus/SKILL.md`
- `assets/aiplus-agent-team/adapters/opencode/skills/aiplus/SKILL.md`

At install time these land in:

- `.claude/skills/aiplus/SKILL.md`
- `.codex/skills/aiplus/SKILL.md`
- `.agents/skills/aiplus/SKILL.md`
- `.opencode/skills/aiplus/SKILL.md`

Project-root preambles are managed by:

- `AGENTS.md` for Codex and OpenCode
- `CLAUDE.md` for Claude Code
- `.opencode/instructions/aiplus.md` for OpenCode's explicit instruction file

## 2. Final SKILL.md Content

The three runtime SKILL files will share the same content, except OpenCode keeps
its `compatibility: opencode` frontmatter field.

Required sections:

- `Prefer MCP Tools Over CLI Subcommands`
- `Example Flows`
- `Avoid Bypass`
- `Known Runtime Limitation`

Key final wording:

```markdown
## Prefer MCP Tools Over CLI Subcommands

This project ships `aiplus agent <verb>` CLI subcommands AND `agent_*` MCP
tools for overlapping use cases. Prefer the MCP tools. MCP returns structured
JSON for agent reasoning; CLI is the human-typing surface and returns text that
you would have to parse.

DO NOT call `aiplus agent dispatch-history --json` to answer cost questions.
Call `agent_token_cost` first.
```

Example planning flow explicitly says:

```markdown
User: "I'm about to implement a payment API for the backend. Help me think
through this."

Do NOT immediately answer with design checklists from training data. First call
`agent_route_score_only` with task="implement payment API for backend", surface
complexity/risk/staffing, then ask whether to dispatch with `agent_route` or
continue without dispatching.
```

## 3. MCP Description Final Forms

All are under 400 chars:

- `agent_token_cost` (232 chars): `PREFERRED programmatic surface for token cost/spend. Use this MCP tool instead of `aiplus agent dispatch-history` or token-cost CLI when answering cost queries; MCP returns structured JSON for 1h/8h/24h windows, per-role, top tasks.`
- `agent_audit_verify_log` (256 chars): `PREFERRED programmatic surface for log integrity. Use this MCP tool instead of `aiplus agent audit verify-log` CLI when answering audit/tamper queries; MCP returns structured JSON. Verifies dispatch-log hash chain and reports PASS/FAIL with first bad line.`
- `agent_route_score_only` (221 chars): `PREFERRED programmatic surface for planning. User says "implement X" -> call this with task="implement X" before answering from training data. Use instead of `aiplus agent route --score-only`; MCP returns structured JSON.`

## 4. Project-Root Preamble Final Form

Use the existing sentinels:

```markdown
<!-- aiplus-discovery-block:start -->
## This project uses AiPlus

When the user asks about token costs, agent team status, dispatch history, or
wants to plan a coding task, prefer aiplus `agent_*` MCP tools over shell grep
or `aiplus agent <verb>` CLI subcommands. MCP returns structured JSON.

Most important patterns:

- "How much am I spending" / cost / burn / USD -> `agent_token_cost`
- "Help me plan / implement X" -> `agent_route_score_only` FIRST, then surface to user
- "Is my log intact" / audit / verify -> `agent_audit_verify_log`

For coding tasks: do NOT answer from training data first. Score the task via
`agent_route_score_only`, surface the result, then ask whether to dispatch.

Full tool list: 11 existing `agent_*` tools + 3 from v0.6.7. Run `tools/list`
to enumerate.
<!-- /aiplus-discovery-block -->
```

## 5. OpenCode Phase 3 Setup

Isolation:

- Use `/tmp/discovery-v2-ratify/project`.
- Use isolated `HOME=/tmp/discovery-v2-ratify/home` and
  `XDG_CONFIG_HOME=/tmp/discovery-v2-ratify/xdg` when running install/register
  and OpenCode.
- Do not run OpenCode against the real home config.

Setup:

```bash
HOME=/tmp/discovery-v2-ratify/home \
XDG_CONFIG_HOME=/tmp/discovery-v2-ratify/xdg \
/tmp/discovery-v2-ratify/bin/aiplus install all --yes --allow-version-skew

HOME=/tmp/discovery-v2-ratify/home \
XDG_CONFIG_HOME=/tmp/discovery-v2-ratify/xdg \
/tmp/discovery-v2-ratify/bin/aiplus mcp-register --runtime opencode --force

HOME=/tmp/discovery-v2-ratify/home \
XDG_CONFIG_HOME=/tmp/discovery-v2-ratify/xdg \
/tmp/discovery-v2-ratify/bin/aiplus mcp-register --runtime opencode --config-dir .opencode --force
```

Run:

```bash
HOME=/tmp/discovery-v2-ratify/home \
XDG_CONFIG_HOME=/tmp/discovery-v2-ratify/xdg \
aiplus secret-broker run --aliases anthropic,openai -- \
  opencode run --dir /tmp/discovery-v2-ratify/project \
  --dangerously-skip-permissions "<prompt>"
```

The project-local `.opencode/opencode.json` contains both AiPlus instructions
and MCP server config, so no global OpenCode config edit is needed.

## 6. Test Plan

- `aiplus install all --yes` writes upgraded SKILL.md files.
- Reinstall remains idempotent and discovery block appears exactly once.
- Installed SKILL.md and preamble contain:
  - `Prefer MCP Tools Over CLI Subcommands`
  - `DO NOT immediately answer with design checklists`
  - `agent_token_cost`
  - `agent_route_score_only`
  - `agent_audit_verify_log`
- `tools/list` includes upgraded descriptions for only the three autoflow tools.
- Description lengths are each <= 400 chars.
- `cargo test`
- `cargo clippy --workspace --all-targets -- -D warnings`
- Phase 3: 6 live prompts, acceptance >= 4/6 expected MCP triggered.

## 7. CHANGELOG Draft

```markdown
## Unreleased

- Strengthen AiPlus agent autoflow discovery: installed runtime skills and
  project preambles now tell agents to prefer `agent_*` MCP tools over CLI
  subcommands for cost, planning, and audit queries, with concrete dialogue
  examples. The three v0.6.7 MCP tool descriptions now advertise themselves as
  the preferred structured programmatic surface.
```

## 8. Phase 3 Evidence

Setup:

```text
rm -rf /tmp/discovery-v2-ratify
mkdir -p /tmp/discovery-v2-ratify/{bin,project,codex-home,home,xdg}
cp target/debug/aiplus /tmp/discovery-v2-ratify/bin/aiplus
cp ~/.codex/auth.json /tmp/discovery-v2-ratify/codex-home/auth.json
cd /tmp/discovery-v2-ratify/project
git init -q && git commit --allow-empty -q -m "init / 初始化"
HOME=/tmp/discovery-v2-ratify/home XDG_CONFIG_HOME=/tmp/discovery-v2-ratify/xdg CODEX_HOME=/tmp/discovery-v2-ratify/codex-home AIPLUS_SKIP_VERSION_CHECK=1 /tmp/discovery-v2-ratify/bin/aiplus install all --yes
CODEX_HOME=/tmp/discovery-v2-ratify/codex-home /tmp/discovery-v2-ratify/bin/aiplus mcp-register --runtime codex --force
HOME=/tmp/discovery-v2-ratify/home XDG_CONFIG_HOME=/tmp/discovery-v2-ratify/xdg /tmp/discovery-v2-ratify/bin/aiplus mcp-register --runtime opencode --force
HOME=/tmp/discovery-v2-ratify/home XDG_CONFIG_HOME=/tmp/discovery-v2-ratify/xdg /tmp/discovery-v2-ratify/bin/aiplus mcp-register --runtime opencode --config-dir .opencode --force
```

Outcome table:

| Runtime | Prompt | Expected tool | Observed | Verdict |
|---|---|---|---|---|
| Codex | `How much money am I spending on AI tools this week?` | `agent_token_cost` | Read `.codex/skills/aiplus/SKILL.md`; emitted `mcp: aiplus/agent_token_cost started`; Codex harness then cancelled. | PASS-triggered-cancelled |
| Codex | `I am about to implement a payment API...` | `agent_route_score_only` | Emitted `mcp: aiplus/agent_route_score_only started`; Codex harness then cancelled. | PASS-triggered-cancelled |
| Codex | `Is my dispatch log intact?` | `agent_audit_verify_log` | Emitted `mcp: aiplus/agent_audit_verify_log started`; Codex harness then cancelled. | PASS-triggered-cancelled |
| OpenCode | `How much money am I spending on AI tools this week?` | `agent_token_cost` | Emitted `aiplus_agent_token_cost {"by_role":true,"top_n":5,"window":"8h"}` and returned cost summary. | PASS |
| OpenCode | `I am about to implement a payment API...` | `agent_route_score_only` | Emitted `aiplus_agent_route_score_only {"task":"Implement a payment API for backend including endpoints, integration, and security"}` and returned HEAVY/risk/staffing summary. | PASS |
| OpenCode | `Is my dispatch log intact?` | `agent_audit_verify_log` | Emitted `aiplus_agent_audit_verify_log` and returned PASS integrity result. | PASS |

Acceptance result:

- 6/6 expected MCP tool triggered.
- 3/6 completed successfully in OpenCode.
- 3/6 Codex cases hit the known non-interactive cancellation after the correct
  MCP tool started; per briefing this counts as PASS for discovery.

Regression gates:

```text
cargo test -p aiplus-cli --test agent_autoflow_discovery
cargo test: 1 passed (1 suite, 1.54s)

cargo test -p aiplus-cli mcp_server::tests::
cargo test: 14 passed, 364 filtered out (43 suites)

cargo test
cargo test: 581 passed, 1 ignored (48 suites, 40.36s)

cargo fmt --check
PASS

cargo clippy --workspace --all-targets -- -D warnings
cargo clippy: No issues found
```

Scope check:

```text
Touched only allowed implementation, docs, runtime skill assets, README, and
tests. No CONTRACT, adapter code, token-cost subtree, calibration, version,
CHANGELOG actual, or install.sh changes.
```
