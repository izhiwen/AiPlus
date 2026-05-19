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
