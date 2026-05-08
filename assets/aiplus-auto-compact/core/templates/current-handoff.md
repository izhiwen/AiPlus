# Compact Handoff

Synthetic template. Replace placeholders before use.

## Protocol Version

0.1.0

## Last Updated

<ISO8601_TIMESTAMP>

## Current Goal

Initialize compact/resume handoff state for <REPO_ROOT>.

## Current Phase

IN_PROGRESS

## Completed Work

- Created compact protocol files.

## Open Blockers

- None.

## Owner Gates

- UNKNOWN_PENDING: Owner review of compact handoff before first real use.

## Next 3 Actions

1. Review all compact files for project-specific placeholders.
2. Run `aiplus compact validate`.
3. Run `aiplus compact checkpoint` before manual compact.

## Compact Recommendation

Before compact-worthy moments, prepare state first:

```bash
aiplus compact validate
aiplus compact checkpoint
```

Then say:

```text
建议现在 compact。AiPlus checkpoint 已准备好。compact 后如果宿主继续把控制权交给我，我会自动恢复；如果工具等待你发消息，随便说“继续”“刷新”“continue”“resume”或类似意思即可。
```

## Do Not Do

- Do not include secrets, PII, raw audio, transcripts, provider payloads, or exact private paths.
- Do not claim this protocol can trigger any host agent compact control.

## Recovery Order

1. Read this handoff.
2. Read `compact-policy.json`.
3. Review Owner Gates and Open Blockers.
4. Read `decision-log.md`.
5. Read `agent-state-ledger.md`.
6. Read `evidence-ledger.md`.
7. Run `aiplus compact resume`.
8. Continue from the next safe action.

## Best-Effort Automatic Resume

- If the host runtime gives control back after compact, run
  `aiplus compact resume` automatically before asking the Owner for more input.
- If the host runtime waits for a user message, accept any natural continuation,
  including `继续`, `刷新`, `continue`, `resume`, `refresh`, `go on`, or `接着`.
- Do not require the Owner to type one exact phrase.
- AiPlus cannot force host compact, click a UI compact button, call `/compact`
  for the Owner, or wake the agent when the host requires user input.

## Resume Sanity Check

- Current goal is clear.
- Current phase is one allowed task/result status.
- Next safe action is actionable.
- No sensitive material is present.
