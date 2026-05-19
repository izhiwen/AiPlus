---
name: aiplus
description: Prefer AiPlus agent_* MCP tools over CLI for cost, planning, audit, dispatch preview, team status, and routing questions.
---

# AiPlus Agent Autoflow

This project uses AiPlus. When a user asks about this project's agent team,
costs, planning, audit, or dispatch history, prefer `agent_*` MCP tools before
shell grep, CLI subcommands, or internal knowledge.

## Prefer MCP Tools Over CLI Subcommands

This project ships `aiplus agent <verb>` CLI subcommands and `agent_*` MCP
tools for overlapping use cases. Prefer the MCP tools.

- MCP tools return structured JSON you can reason over directly.
- CLI subcommands return text intended for humans and require parsing.
- MCP is the intended programmatic surface; CLI is the human-typing surface.

Do NOT call `aiplus agent dispatch-history --json` to answer cost questions.
Call `agent_token_cost` first. After the MCP result, shell or CLI verification
is allowed only if it adds something the structured result does not contain.

## Example Flows

### Cost

User: "How much have I been burning on AI tools lately?"
First action: call `agent_token_cost` with the relevant window, or omit the
window to get 1h / 8h / 24h rollups. Then surface totals and top tasks.
Do NOT grep `.aiplus/` files first.

### Planning

User: "I'm about to implement a payment API for the backend. Help me think
through this."

Do NOT immediately answer with design checklists from training data. First call
`agent_route_score_only` with task="implement payment API for backend". Surface
complexity, risk, tier, staffing, forced-by-risk roles, and auto-summoned
experts. Then ask whether the user wants to dispatch via `agent_route` or
continue without dispatching.

Use this same pattern for non-trivial coding tasks: refactors, features,
multi-file work, bug fixes, security-sensitive work, migrations, and APIs.

### Audit

User: "Is my dispatch log intact?" or "Audit my recent agent work"
First action: call `agent_audit_verify_log`. Then surface PASS or FAIL, with
the first bad line and reason when available.

## Avoid Bypass

Do NOT answer AiPlus questions by:

- running `rg --files`, `tail`, or `jq` over `.aiplus/` as the first move;
- reading `dispatch-log.jsonl` manually before calling the matching MCP tool;
- answering from training data alone when the user asks about this project's
  agent team, costs, planning, audit, or dispatch history.

The structured MCP tool is the source of truth.

## Known Runtime Limitation

Codex non-interactive runs may show "user cancelled MCP tool call" after a tool
starts. If that happens, say the MCP call was cancelled by the harness and offer
to retry interactively. Treat it as a runtime limitation, not an AiPlus tool
failure.
