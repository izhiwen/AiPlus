# AiPlus Compact Protocol

Use this prompt as shared guidance for OpenCode compact checkpoint and resume work.

## Purpose

The protocol preserves enough task state to continue after context compaction.
It supports best-effort resume after compact, but it does not provide safety
assurances, guaranteed recovery, or default-on protection.

## Required State

Track:

- task id and current status
- claimed files and forbidden files
- changed files
- commands run
- verification evidence
- risks or open questions
- incomplete work
- handoff notes
- owner gate requirement
- recommended next action

## Project Scope

- Respect the user's current task scope over adapter defaults.
- Inspect workspace state before writing or recommending edits.
- Stop and report blocked when the active task requires stopping on unrelated edits in claimed files.
- Do not revert user or teammate changes.
- Do not publish, push, tag, or change global configuration unless the user explicitly asks and the active scope permits it.

## OpenCode Usage

- Prefer project-local `.opencode/opencode.json`, `.opencode/commands/`, and `.opencode/agents/`.
- Do not modify `~/.config/opencode/` by default.
- Treat Plan style work as analysis, checkpoint preparation, and validation.
- Treat Build style work as implementation only after scope and conflict checks pass.
- Where a project includes the AiPlus CLI, use `aiplus compact validate`,
  `aiplus compact checkpoint`, and `aiplus compact resume` when appropriate.

## Packet Shape

Return checkpoint and handoff packets with:

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
