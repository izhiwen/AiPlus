---
description: Create a project-local compact checkpoint packet for the active task.
---

# compact-checkpoint

Create a compact checkpoint for the current Claude Code task.

## Scope

This is a project-local command document. Do not modify global Claude Code configuration, publish a plugin, push commits, or create tags.

## Instructions

1. Re-read the task instructions and claimed file scope.
2. Inspect the current workspace state for files relevant to the claimed scope.
3. Record only facts that are useful after compaction.
4. If the user gave a checkpoint destination, write there only if it is inside the allowed project scope.
5. If no destination is available, return the checkpoint packet in the chat.
6. When the AiPlus CLI is available, run or recommend
   `aiplus compact validate` and `aiplus compact checkpoint`.

## Packet Format

Return:

- `task_id`
- `status`
- `claimed_files`
- `changed_files`
- `commands_run`
- `verification_evidence`
- `risks_or_open_questions`
- `not_done`
- `handoff_notes`
- `owner_gate_needed`
- `recommended_next_action`

## Guardrails

- Do not imply hooks are required or enabled by default.
- Do not claim automatic recovery is guaranteed. Resume is best-effort after
  compact.
- Do not edit files outside the current task's allowed scope.
