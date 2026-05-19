# Agent Autoflow Discovery Implementation Notes

Status: Phase 3 complete. Discovery triggered expected MCP tools in 3/3 live
Codex prompts; Codex non-interactive execution cancelled the MCP calls before
tool results returned.

## 1. Per-Runtime Mechanism

Claude Code:

- Project-local skill path: `.claude/skills/<skill-name>/SKILL.md`.
- Current AiPlus install already writes Claude Code slash commands and subagents
  under `.claude/commands/` and `.claude/agents/`.
- Current AiPlus install also writes `CLAUDE.md` with an AiPlus managed block.
- Decision: write `.claude/skills/aiplus/SKILL.md` and add a separate
  `<!-- aiplus-discovery-block:start -->` managed block to `CLAUDE.md`.

Codex:

- Current AiPlus install writes root `AGENTS.md` with an AiPlus managed block
  pointing to `.aiplus/AGENTS.aiplus.md`.
- The most reliable session-start channel is `AGENTS.md`; this is already the
  runtime bridge used by AiPlus.
- Decision: write a discovery managed block to root `AGENTS.md`. Also write
  skill-format content to `.codex/skills/aiplus/SKILL.md` and
  `.agents/skills/aiplus/SKILL.md` as project-local/portable skill artifacts.

OpenCode:

- Current AiPlus install writes `.opencode/opencode.json` with an `instructions`
  entry pointing to `.opencode/instructions/aiplus.md`.
- OpenCode project skill path: `.opencode/skills/<name>/SKILL.md`; current
  AiPlus already uses OpenCode's `instructions` channel.
- Decision: write `.opencode/skills/aiplus/SKILL.md`, keep the existing
  `.opencode/instructions/aiplus.md` channel, and rely on the shared root
  `AGENTS.md` discovery block as the project-root preamble.

## 2. Current Install Writes

Codex install:

- `AGENTS.md` with `<!-- BEGIN AIPLUS MANAGED BLOCK -->`.
- `.aiplus/AGENTS.aiplus.md`, `.aiplus/manifest.json`, module assets, and
  runtime-independent substrate files.

Claude Code install:

- Everything Codex gets via `update_agents_md`.
- `.claude/commands/aiplus-refresh.md`
- `.claude/commands/aiplus-route.md`
- `.claude/agents/aiplus-advisor.md`
- `.claude/agents/aiplus-memory.md`
- `.claude/agents/aiplus-compact.md`
- `.claude/agents/aiplus-velocity.md`
- `.claude/agents/aiplus-team-consultant.md`
- `.claude/settings.local.json` hooks.
- `CLAUDE.md` with the AiPlus managed block.
- Agent-team mirrored subagents/commands when agent-team is active.

OpenCode install:

- Everything Codex gets via `update_agents_md`.
- `.opencode/opencode.json` with AiPlus instruction path and primary agent.
- `.opencode/instructions/aiplus.md`
- `.opencode/commands/aiplus-refresh.md`
- `.opencode/commands/aiplus-route.md`
- `.opencode/agents/aiplus-advisor.md`
- `.opencode/prompts/aiplus.md`
- `.opencode/prompts/aiplus-route.md`
- Agent-team mirrored subagents/commands when agent-team is active.

Install logic is in `crates/aiplus-cli/src/main.rs`:

- `command_install`
- `install_runtime_adapter`
- `update_agents_md`
- `update_claude_md`
- `install_opencode_config`
- `install_opencode_instructions`

## 3. Design Decisions

Skill content:

- One shared source text, installed into runtime-specific paths.
- YAML frontmatter uses `name: aiplus` and a high-signal description naming
  cost, spending, token usage, planning, dispatch, audit, log integrity, and
  tamper detection.
- Body tells the agent to prefer MCP tools before shell grep/internal knowledge:
  `agent_token_cost`, `agent_route_score_only`, `agent_audit_verify_log`,
  `agent_route`, `agent_invite`, `agent_status`, `agent_doctor`, `agent_list`.

Preamble content:

- Separate managed block, not folded into the existing AiPlus managed block.
- Sentinels:
  - `<!-- aiplus-discovery-block:start -->`
  - `<!-- /aiplus-discovery-block -->`
- Reinstall behavior: replace existing discovery block; append when absent;
  never duplicate.
- Existing user content remains untouched.

File targets:

- Claude Code: `.claude/skills/aiplus/SKILL.md` + `CLAUDE.md` discovery block.
- Codex: `.codex/skills/aiplus/SKILL.md`, `.agents/skills/aiplus/SKILL.md`,
  and `AGENTS.md` discovery block.
- OpenCode: `.opencode/skills/aiplus/SKILL.md`,
  `.opencode/instructions/aiplus.md` discovery section, and root `AGENTS.md`
  discovery block.

## 4. Test Plan

- Fresh `aiplus install all --yes` writes all discovery files.
- Re-running `aiplus install all --yes` does not duplicate discovery sentinel
  blocks and reports skip-identical/managed-update rather than conflicts.
- Existing `AGENTS.md` and `CLAUDE.md` user content is preserved.
- Skill files have required frontmatter and mention all three new MCP tools.
- `cargo test` PASS.
- `cargo clippy --workspace --all-targets -- -D warnings` PASS.

## 5. CHANGELOG Draft

```markdown
## Unreleased

- Add an AiPlus discovery layer for agent runtimes: `aiplus install` now writes
  project-local skill instructions and managed preamble blocks so natural
  prompts about cost, planning, dispatch, audit, and team status steer agents
  toward the `agent_*` MCP tools instead of shell-grep bypasses.
```

## 6. Phase 3 Evidence

Setup:

```text
rm -rf /tmp/discovery-test
mkdir -p /tmp/discovery-test/project /tmp/discovery-test/bin /tmp/discovery-test/codex-home /tmp/discovery-test/home /tmp/discovery-test/xdg
cp target/debug/aiplus /tmp/discovery-test/bin/aiplus
HOME=/tmp/discovery-test/home XDG_CONFIG_HOME=/tmp/discovery-test/xdg CODEX_HOME=/tmp/discovery-test/codex-home /tmp/discovery-test/bin/aiplus install all --yes --allow-version-skew
CODEX_HOME=/tmp/discovery-test/codex-home /tmp/discovery-test/bin/aiplus mcp-register --runtime codex --force
```

Install evidence:

```text
agents_discovery=1
claude_skill=yes
codex_skill=yes
opencode_skill=yes
```

Live Codex prompt outcomes:

| Prompt | Expected tool | Observed tool behavior | Verdict |
|---|---|---|---|
| `How much money am I spending on AI tools this week?` | `agent_token_cost` | Codex read `.agents/skills/aiplus/SKILL.md`, then emitted `mcp: aiplus/agent_token_cost started`; Codex non-interactive runtime cancelled the tool call before result. | PASS for discovery |
| `I am about to implement a payment API for the backend. Can you help me think through this before I start?` | `agent_route_score_only` | Codex read `.agents/skills/aiplus/SKILL.md`, then emitted `mcp: aiplus/agent_route_score_only started`; Codex non-interactive runtime cancelled the tool call before result. | PASS for discovery |
| `Is my dispatch log intact?` | `agent_audit_verify_log` | Codex read `.codex/skills/aiplus/SKILL.md`, then emitted `mcp: aiplus/agent_audit_verify_log started`; it retried once and both attempts were cancelled by Codex non-interactive runtime before result. | PASS for discovery |

Acceptance interpretation:

- Stage 6 failure mode was "Codex bypasses MCP and uses shell/internal
  knowledge." That is fixed: 3/3 prompts selected the expected `agent_*` MCP
  tool.
- Residual deviation: the live `codex exec` harness cancelled MCP execution
  after starting the tool, so it did not surface structured tool results. This
  appears to be Codex non-interactive MCP approval/runtime behavior, not a
  discovery failure.

Regression gates:

```text
cargo test
cargo test: 580 passed, 1 ignored (48 suites, 43.69s)

cargo fmt --check
PASS

cargo clippy --workspace --all-targets -- -D warnings
cargo clippy: No issues found
```
