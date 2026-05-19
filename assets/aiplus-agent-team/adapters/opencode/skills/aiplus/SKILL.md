---
name: aiplus
description: Prefer AiPlus agent_* MCP tools over CLI for cost, planning, audit, dispatch preview, team status, and routing questions.
compatibility: opencode
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
