---
name: aiplus
description: Use AiPlus agent MCP tools for token cost, spending, task planning, dispatch preview, audit, log integrity, team status, and agent routing questions.
---

# AiPlus Agent Autoflow

This project uses AiPlus. When the user asks a natural-language question about
the agent team, costs, planning, audit, or dispatch history, prefer the
project's `agent_*` MCP tools before shell grep or internal knowledge.

## Use These Tools First

- Cost, spending, token consumption, USD burn, or recent AI usage:
  call `agent_token_cost`.
- Planning a non-trivial code task, especially payment, auth, security,
  refactoring, multi-file work, or new features:
  call `agent_route_score_only` first to show the coordinator's would-staffing
  plan, then ask whether the user wants to dispatch.
- Log integrity, audit, tampering, hash-chain, or dispatch-history verification:
  call `agent_audit_verify_log`.
- Dispatching real work to roles:
  call `agent_route` or `agent_invite`.
- Team status, setup, health, or available roles:
  call `agent_status`, `agent_doctor`, or `agent_list`.

## Avoid Bypass

Do not answer AiPlus cost, audit, planning, or team-status questions by only
grepping `.aiplus/` files or relying on built-in knowledge. Shell inspection can
be useful after the MCP result, but the structured AiPlus tool is the source of
truth.

When unsure which AiPlus capability fits the request, inspect the available
`agent_*` MCP tools and tell the user which one you will use.
