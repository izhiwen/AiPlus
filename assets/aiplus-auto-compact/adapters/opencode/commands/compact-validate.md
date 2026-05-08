---
description: Validate compact adapter files and checkpoint readiness.
agent: compact-reviewer
subtask: true
---

# compact-validate

Validate the OpenCode compact adapter files and any active checkpoint packet.

## Scope

This is a project-local command document. It may inspect project files and report findings, but it must not modify global OpenCode configuration.

## Checks

1. Confirm the example config exists at `opencode.json.example`.
2. Confirm agent docs exist in `agents/`:
   - `compact-advisor.md`
   - `compact-reviewer.md`
3. Confirm command docs exist in `commands/`:
   - `compact-checkpoint.md`
   - `compact-resume.md`
   - `compact-validate.md`
4. Confirm the protocol prompt exists at `prompts/compact-protocol.md`.
5. Validate JSON syntax for `opencode.json.example`.
6. Confirm docs describe project-local OpenCode setup and do not require modifying `~/.config/opencode/` by default.
7. Confirm docs do not claim default-on automation, full automatic safety, or complete recovery.

## Output

Return:

- `status`
- `files_checked`
- `json_validation`
- `scope_violations`
- `missing_items`
- `risks_or_open_questions`
- `recommended_next_action`
