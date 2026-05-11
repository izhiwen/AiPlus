# First Action

When the user mentions "aiplus", "AiPlus", or any AiPlus subcommand in this conversation, your first action is to read `.aiplus/AGENTS.aiplus.md` and follow its translation table. Never pass a Chinese phrase as a literal CLI argument — always translate to the English subcommand first.

---
description: Use the AiPlus compact checkpoint and resume protocol before, during, and after context compaction.
disable-model-invocation: true
---

# Compact Protocol

Use this skill when a Claude Code session needs to preserve task state across manual or automatic context compaction.

## Operating Rules

- Treat the current project as the source of truth.
- Keep all checkpoint and resume notes project-local unless the user explicitly gives a different storage location.
- Before compaction, capture the active task, claimed files, changed files, commands run, verification evidence, blockers, and next action.
- After compaction or resume, re-read the checkpoint before continuing work.
- Do not claim full automatic safety. Hooks can help remind or collect context, but the agent must still review the checkpoint and current workspace state.
- Do not modify global Claude Code settings from this adapter.

## Checkpoint Packet

When creating a checkpoint, include:

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

## Slash Commands

This plugin exposes project-local command docs:

- `/aiplus-compact-reminder:compact-checkpoint`
- `/aiplus-compact-reminder:compact-resume`
- `/aiplus-compact-reminder:compact-validate`

Use these commands as structured prompts. They document the expected protocol and do not install global config or enable hooks automatically.
