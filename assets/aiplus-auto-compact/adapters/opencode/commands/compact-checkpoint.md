---
description: Create a compact checkpoint packet for the active task.
agent: compact-advisor
subtask: true
---

# compact-checkpoint

Create a project-local compact checkpoint for the active OpenCode task.

## Scope

This command is a prompt document. It does not modify global OpenCode configuration, publish packages, push commits, or create tags.

## Instructions

1. Re-read the task instructions, claimed files, forbidden files, conflict rule, and owner gate.
2. Inspect the current workspace state for files relevant to the claimed scope.
3. Record only facts that are useful after compaction.
4. If the user gave a checkpoint destination, write there only when it is inside the allowed project scope.
5. If no destination is available or writing is not allowed, return the checkpoint packet in chat.
6. When a project has the AiPlus CLI available, run or recommend
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

- Do not edit outside the active task's allowed scope.
- Do not imply global OpenCode config changes are required.
- Do not claim automatic safety, guaranteed recovery, or default-on protection.
