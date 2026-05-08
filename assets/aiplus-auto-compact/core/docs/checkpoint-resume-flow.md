# Checkpoint And Resume Flow

AiPlus Auto Compact is local-first. It prepares evidence for a compact/resume
boundary; it does not press, call, or automate any host agent compact control.

## Installed Through AiPlus

From a target repository where `aiplus` is available:

```bash
aiplus compact validate
aiplus compact checkpoint
```

`validate` checks the local compact state. `checkpoint` writes a redacted JSON
checkpoint under `.codex/compact/checkpoints/` with validation status, Owner
gate state, review items, warnings, errors, dirty git summary when available,
and the next safe action.

Only recommend manual compact when checkpoint output is `SAFE_TO_COMPACT` and
all Owner gates are explicitly `APPROVED`.

## Recommended Compact Message

When checkpoint state is ready, the agent can say:

```text
建议现在 compact。AiPlus checkpoint 已准备好。compact 后如果宿主继续把控制权交给我，我会自动恢复；如果工具等待你发消息，随便说“继续”“刷新”“continue”“resume”或类似意思即可。
```

The user or host runtime still performs compact manually.

## Resume After Host Compact

If the host gives control back automatically, the agent should run:

```bash
aiplus compact resume
```

and continue from the checkpoint and local handoff files.

If the host waits for a user message, any natural continuation should be enough
to restart the agent workflow:

```text
继续
刷新
refresh
continue
resume
go on
接着
```

This is best-effort automatic resume. AiPlus Auto Compact can prepare the
checkpoint and tell the agent how to resume, but it cannot wake a host runtime
that requires a user message.

## Module-Only Or Legacy Reference

Advanced users who adopt only AiPlus Auto Compact can inspect `core/templates/`
and `core/docs/` directly. The legacy Node helper is retained for compatibility
audits:

```bash
node <PROJECT_ROOT>/core/scripts/compactctl.mjs init
node <PROJECT_ROOT>/core/scripts/compactctl.mjs validate
node <PROJECT_ROOT>/core/scripts/compactctl.mjs checkpoint
node <PROJECT_ROOT>/core/scripts/compactctl.mjs resume
```

This is not the ordinary beginner path.
