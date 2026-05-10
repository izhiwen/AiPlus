# Compact Recovery

Compact Recovery Notes help a new session resume after context compaction.

Use `core/templates/compact-recovery-note.md` when a session needs a compact state summary:

- product name
- current goal
- session role
- workflow level
- decisions made
- files changed
- files to verify
- Owner gates
- next action
- do-not-do items

In AiPlus v2.1, `aiplus compact prepare` also creates a **context capsule**
(`.codex/compact/context-capsule.json`) with checksum validation. `aiplus compact resume`
reads the capsule first and falls back to the legacy handoff file if the capsule is missing
or invalid. The capsule carries the same fields as the recovery note plus structured
decisions and owner-gate arrays.
