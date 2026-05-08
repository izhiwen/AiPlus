---
name: compact-protocol
description: Use when coordinating long Codex CLI tasks that need compact handoffs, resumable workflows, multi-agent continuity, decision logs, evidence ledgers, Owner gates, checkpoint validation, or safe resume after context compaction.
---

# Compact Protocol

Use this skill for long Codex CLI work where context compaction could interrupt continuity. The protocol prepares handoff files, validates structure, records decisions and evidence, preserves Owner gates, and prints a resume packet after compact.

This skill cannot trigger the Codex UI compact button. It only prepares, validates, checkpoints, and reminds. The user still manually runs `/compact` or clicks the UI compact control.

## Recommend Compact

Recommend a manual compact only after `aiplus compact checkpoint` returns
`SAFE_TO_COMPACT`, the handoff files have a current goal, current phase, next 3
actions, no unresolved critical blockers, and all Owner gates are explicitly
`APPROVED`. `UNKNOWN_PENDING` must remain `UNKNOWN_NEEDS_REVIEW` and must not be
used as a compact recommendation condition.

When checkpoint state is ready, use clear manual-compact wording:

```text
建议现在 compact。AiPlus checkpoint 已准备好。compact 后如果宿主继续把控制权交给我，我会自动恢复；如果工具等待你发消息，随便说“继续”“刷新”“continue”“resume”或类似意思即可。
```

## Do Not Recommend Compact

Do not recommend compact when validation fails, required files are missing, next actions are empty, an Owner gate is `DENIED` or `UNKNOWN_PENDING`, sensitive material appears in compact files, evidence is unreviewed for a high-risk decision, or the current task is in the middle of an irreversible operation.

## Required Files

All compact state lives under `.codex/compact/`:

- `current-handoff.md`
- `decision-log.md`
- `agent-state-ledger.md`
- `evidence-ledger.md`
- `compact-policy.json`
- `checkpoints/`

## Recovery Order

1. Read `current-handoff.md`.
2. Check `compact-policy.json`.
3. Review Owner Gates and Open Blockers.
4. Read `decision-log.md` for active decisions.
5. Read `agent-state-ledger.md` for delegated work state.
6. Read `evidence-ledger.md` for verification evidence.
7. Run `aiplus compact resume` when the host returns control after compact.
8. Continue only from the listed Next Safe Action.

## Owner Gates

Preserve every Owner gate exactly as `APPROVED`, `DENIED`, or `UNKNOWN_PENDING`. Do not convert unknown approval into approval. A `DENIED` gate blocks compact recommendation until the plan changes or the Owner explicitly approves a replacement path.

## Redaction Rules

Never write API keys, tokens, JWTs, cookies, Authorization headers, private keys, raw audio, full transcripts, provider request/response bodies, PII, device IDs, real billing/account IDs, exact private paths, Dropbox/iCloud private paths, unredacted logs, screenshots containing secrets, or real Owner approvals into compact files. Use `<REPO_ROOT>`, `<HOME>`, `<REDACTED_SECRET>`, `<REDACTED_TOKEN>`, `<REDACTED_PII>`, and `<ARTIFACT_ID>`.

## CEO/Multi-Agent Pattern

Use at most 5 review/discussion rounds. The CEO records the goal, current phase, decisions, Owner gates, delegated agent status, evidence, blockers, and next 3 actions before each compact checkpoint. Agents should report only status, evidence, blockers, and changed files.

## AiPlus CLI

Run from a target repository:

```bash
aiplus compact validate
aiplus compact checkpoint
aiplus compact resume
```

`validate` checks structure only; passing validation does not mean safe to
compact. `checkpoint` writes non-sensitive metadata and prints
`SAFE_TO_COMPACT`, `BLOCKED_DO_NOT_COMPACT`, or `UNKNOWN_NEEDS_REVIEW`.
`resume` prints the resumable state or `RESUME_BLOCKED`.

## After Compact

After compact, if Codex returns control automatically, first run
`aiplus compact resume`, then inspect the compact files in recovery order.
Continue only from the next safe action, and re-run `validate` before changing
gates or decisions. If Codex waits for the user, natural continuation messages
such as `继续`, `刷新`, `refresh`, `continue`, `resume`, `go on`, or `接着`
should restart the resume flow. This is best-effort; the skill cannot wake
Codex by itself.

## Legacy Node Helper

`compactctl.mjs` remains only as archived history and a compatibility test
fixture. It is not the ordinary AiPlus path and must not be used as a fallback if
`aiplus` is missing.

## Failure Behavior

If files are missing, validation fails, sensitive patterns are detected, or the next safe action is unclear, stop and report `BLOCKED_DO_NOT_COMPACT` or `RESUME_BLOCKED`. Ask the Owner only for the missing approval or clarification needed to proceed.
