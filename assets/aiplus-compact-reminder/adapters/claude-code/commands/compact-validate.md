---
description: Validate compact protocol artifacts and summarize readiness.
---

# compact-validate

Validate the Claude Code compact adapter files and any active checkpoint packet.

## Scope

This is a project-local command document. It may inspect project files and report findings, but it must not modify global Claude Code configuration.

## Checks

1. Confirm the plugin manifest exists at `.claude-plugin/plugin.json`.
2. Confirm the compact protocol skill exists at `skills/compact-protocol/SKILL.md`.
3. Confirm command docs exist in `commands/`:
   - `compact-checkpoint.md`
   - `compact-resume.md`
   - `compact-validate.md`
4. Validate JSON syntax for plugin and hook example files.
5. Confirm hooks are described as optional and review-before-use.
6. Confirm docs do not claim default-on hooks or full automatic safety.

## Output

Return:

- `status`
- `files_checked`
- `json_validation`
- `scope_violations`
- `missing_items`
- `risks_or_open_questions`
- `recommended_next_action`
