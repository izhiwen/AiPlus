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
Ready to compact.

After compact:
- If I continue automatically, you do not need to do anything.
- If I do not reply, send: continue

I will resume from here.
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
7. Run `aiplus compact resume` after compact before continuing work.
8. Continue only from the listed Next Safe Action.

## Owner Gates

Preserve every Owner gate exactly as `APPROVED`, `DENIED`, or `UNKNOWN_PENDING`. Do not convert unknown approval into approval. A `DENIED` gate blocks compact recommendation until the plan changes or the Owner explicitly approves a replacement path.

## Redaction Rules

Never write API keys, tokens, JWTs, cookies, Authorization headers, private keys, raw audio, full transcripts, provider request/response bodies, PII, device IDs, real billing/account IDs, exact private paths, Dropbox/iCloud private paths, unredacted logs, screenshots containing secrets, or real Owner approvals into compact files. Use `<REPO_ROOT>`, `<HOME>`, `<REDACTED_SECRET>`, `<REDACTED_TOKEN>`, `<REDACTED_PII>`, and `<ARTIFACT_ID>`.

## CEO/Multi-Agent Pattern

Use at most 5 review/discussion rounds. The CEO records the goal, current phase, decisions, Owner gates, delegated agent status, evidence, blockers, and next 3 actions before each compact checkpoint. Agents should report only status, evidence, blockers, and changed files.

## AiPlus CLI

Natural language is the primary interface for ordinary users. If the user says
"prepare compact", "help me compact", "I want to compact", "save progress",
"checkpoint this", "get ready for compact", "我想 compact", "准备 compact",
"帮我准备 compact", "保存进度", "做个交接", or "我要 compact 了", run the
backend tool:

```bash
aiplus compact prepare
```

If `prepare` is unavailable, use the closest supported sequence:

```bash
aiplus compact validate
aiplus compact checkpoint
```

Advanced users and maintainers may also run:

```bash
aiplus compact score
aiplus compact checkpoint --level standard
aiplus compact resume
aiplus compact savings
aiplus pricing status
```

`prepare` evaluates readiness and creates a checkpoint when appropriate.
`score` prints explainable compact pressure. `validate` checks structure only;
passing validation does not mean safe to compact. `checkpoint` writes
non-sensitive metadata and prints readiness state. `resume` prints the resumable
state or `RESUME_BLOCKED`.

If the user asks "show compact savings", "how many tokens did compact save?",
"compact 帮我省了多少？", or "看一下 compact 收益", run:

```bash
aiplus compact savings
```

Savings output is an estimate only. It is not billing data or quality proof. Do
not ask the user to enter model prices. Do not upload prompts, project files,
checkpoints, savings ledgers, secrets, billing data, or usage history.

If the user says "update AiPlus", "update everything", "升级 AiPlus", or
"把 AiPlus 全部更新到最新版", report update scope and run:

```bash
aiplus update all
```

If the user says "only update this project's AiPlus" or "只更新这个项目的 AiPlus",
run `aiplus update`. If the user says "update the aiplus command" or
"更新 aiplus 命令", run `aiplus self update`. Before updating, say that you will
not edit global agent config or upload project data.

If the user asks "private profile status", "我的偏好生效了吗", or "检查我的
AiPlus profile", run `aiplus profile status` and keep the answer short. If the
user asks "secret 状态", "看看 secret", "检查 API key", or "API key 是否可用", run
`aiplus secret-broker status` or `aiplus secret-broker doctor`. Never print,
paste, compact, summarize, persist, or log secret values. If an explicit compact
support action needs a key, use `aiplus secret-broker run -- <command...>` so the
value enters only the child process environment.
The child command can still print, log, transmit, or store its environment. Use
`run --` only with trusted commands for the specific action.
Private profiles may install approved secret aliases; run `aiplus secret-broker list` for the current local table. Real Bitwarden checks require the `bws` CLI. If `bws` is unavailable, report that real Bitwarden smoke is unverified; do not print secret values or invent a fallback.

## After Compact

After compact, if the agent gets control automatically, or if the user says
"continue after compact", "resume after compact", `continue`, `resume`,
`refresh`, `继续`, `刷新`, or similar, first run `aiplus compact resume`, then
inspect the compact files in recovery order.
Continue only from the next safe action, and re-run `validate` before changing
gates or decisions. If Codex does not reply, natural continuation messages
such as `继续`, `刷新`, `refresh`, `continue`, `resume`, `go on`, or `接着`
should restart the resume flow. This is best-effort; the skill cannot wake
Codex by itself.

## Legacy Node Helper

`compactctl.mjs` remains only as archived history and a compatibility test
fixture. It is not the ordinary AiPlus path and must not be used as a fallback if
`aiplus` is missing.

## Failure Behavior

If files are missing, validation fails, sensitive patterns are detected, or the next safe action is unclear, stop and report `BLOCKED_DO_NOT_COMPACT` or `RESUME_BLOCKED`. Ask the Owner only for the missing approval or clarification needed to proceed.
