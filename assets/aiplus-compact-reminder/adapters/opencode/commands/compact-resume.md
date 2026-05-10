---
description: Resume a task from a compact checkpoint packet.
agent: compact-advisor
subtask: true
---

# compact-resume

Resume work from an AiPlus compact checkpoint in an OpenCode session.

## Scope

This command reads checkpoint context and current project state. It does not modify global OpenCode configuration.

## Instructions

1. Read the latest checkpoint packet supplied by the user or stored in a project-local checkpoint location.
2. Re-check the active task instructions, claimed files, forbidden files, conflict rule, and owner gate.
3. Inspect current workspace state before editing.
4. Compare checkpoint `changed_files`, `not_done`, and `recommended_next_action` against the actual workspace.
5. Continue only if the claimed scope still permits the next action.
6. If unrelated edits are present in a claimed directory and the task says to stop on conflict, report blocked.
7. When a project has the AiPlus CLI available and the host has returned
   control, run or recommend `aiplus compact resume`.

## Expected Resume Summary

Return a brief summary before continuing:

- active `task_id`
- files still in scope
- completed work
- remaining work
- immediate next action
- blockers, if any

## Guardrails

- Do not assume the checkpoint is complete or current.
- Do not revert user or teammate changes.
- Do not publish, push, tag, or alter global configuration.
- Do not claim the command can wake OpenCode if OpenCode is waiting for a user
  message.
