---
description: Resume a Claude Code task from a compact checkpoint packet.
---

# compact-resume

Resume work from an AiPlus compact checkpoint.

## Scope

This is a project-local command document. It reads checkpoint context and current project state; it does not modify global Claude Code configuration.

## Instructions

1. Read the latest checkpoint packet supplied by the user or stored in the project-local checkpoint location.
2. Re-check the active task instructions, claimed files, and forbidden files.
3. Inspect the current workspace state before editing.
4. Compare checkpoint `changed_files`, `not_done`, and `recommended_next_action` against the actual workspace.
5. Continue only if the claimed scope still permits the next action.
6. If unrelated edits are present in the claimed directory and the task says to stop on conflict, report blocked.
7. When work continues after compact and the AiPlus CLI is available, run or
   recommend `aiplus compact resume`.

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
- Do not claim this command can wake Claude Code if Claude Code is waiting for a
  user message.
