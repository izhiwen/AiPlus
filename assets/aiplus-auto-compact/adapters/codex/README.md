# Codex Adapter

This adapter preserves the proven Codex compact/resume workflow from
`codex-compact-protocol` while aligning ordinary users with the AiPlus CLI
(`aiplus`).

## What It Does

- Provides a Codex plugin manifest at `.codex-plugin/plugin.json`.
- Provides a Codex skill at `skills/compact-protocol/SKILL.md`.
- Provides project-local guidance that can be installed by `aiplus install codex`.
- Keeps legacy Node helper references available for migration review.

## What It Cannot Do

This adapter cannot force Codex UI compact, click UI controls, or call
`/compact` for the user. It prepares checkpoint/resume state only. The user
still manually runs `/compact` or uses the Codex UI compact control after
review.

## Local Use

From a target project:

```bash
aiplus install codex
aiplus compact validate
aiplus compact checkpoint
```

Only recommend manual compact after `checkpoint` returns `SAFE_TO_COMPACT` and every Owner gate is explicitly `APPROVED`. `UNKNOWN_PENDING` remains `UNKNOWN_NEEDS_REVIEW`; `DENIED` blocks compact recommendation.

When checkpoint state is ready, tell the user:

```text
建议现在 compact。AiPlus checkpoint 已准备好。compact 后如果宿主继续把控制权交给我，我会自动恢复；如果工具等待你发消息，随便说“继续”“刷新”“continue”“resume”或类似意思即可。
```

After compact, if Codex returns control automatically:

```bash
aiplus compact resume
```

If Codex waits for the user, explicit AiPlus continuation messages such as
`AiPlus 刷新`, `刷新 AiPlus`, `aiplus refresh`, `aiplus status`,
`AiPlus status`, `继续 AiPlus`, or `resume AiPlus` should restart the resume
flow. Generic messages such as `继续`, `刷新`, `refresh`, `continue`, `resume`,
`go on`, or `接着` should try AiPlus first when possible; when project-specific
refresh rules conflict, report AiPlus status before project status. This is
best-effort; the adapter cannot wake Codex by itself.

## Shared Core

The Codex adapter intentionally does not duplicate the core implementation. Shared assets live in:

```text
../../core/schemas/
../../core/templates/
../../core/docs/
```

This keeps Codex behavior aligned with Claude Code and OpenCode adapters.

`../../core/scripts/compactctl.mjs` remains available as a legacy standalone
helper for audits and compatibility tests, but it is not the ordinary-user
AiPlus CLI (`aiplus`) path.
