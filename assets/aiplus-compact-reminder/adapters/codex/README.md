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
aiplus compact remind
aiplus compact remind --event phase-end
aiplus compact prepare
aiplus compact validate
aiplus compact checkpoint
```

Compact Reminder's core value is proactive reminder timing. For HEAVY tasks, run
`aiplus compact remind --event long-session` at least every 30 minutes and at
major phase boundaries, before review/QA, before many subagents, before release
prep, and before Owner handoff. For MEDIUM tasks, run it at phase boundaries and
before review/QA. For LIGHT tasks, run it only on user request or an obvious
handoff point.

If `REMINDER_DECISION=prepare_only`, update handoff/checkpoint first. If
`REMINDER_DECISION=wait` or `blocked`, explain the safety reason and keep
working. If `REMINDER_DECISION=remind_now`, run or confirm `aiplus compact
prepare`, then suggest manual host compact.

Only recommend manual compact after `checkpoint` returns `SAFE_TO_COMPACT` and every Owner gate is explicitly `APPROVED`. `UNKNOWN_PENDING` remains `UNKNOWN_NEEDS_REVIEW`; `DENIED` blocks compact recommendation.

When checkpoint state is ready, tell the user:

```text
Ready to compact.

After compact:
- If I continue automatically, you do not need to do anything.
- If I do not reply, send: continue

I will resume from here.
```

## Watch Mode

For continuous monitoring during long sessions:

```bash
aiplus compact watch --once
aiplus compact watch --interval 10m
aiplus compact watch --once --json
```

`watch` evaluates the same decision logic as `remind` but records state to
`.codex/compact/reminder-state.json`. It does not trigger host compact automatically.
Use `--once` for a single evaluation or `--interval` for repeated checks.

## Context Capsule

After `aiplus compact prepare`, a `context-capsule.json` is created with
hot/warm/cold context tiers, importance scoring, Owner gates, decisions, and
recovery metadata. This capsule is redacted (no secrets, no raw transcript) and
includes checksums for integrity. It helps resume work after compact by preserving
the objective, current state, and next safe action.

## Safety Boundaries

- AiPlus never triggers host compact automatically.
- AiPlus never captures raw transcript or secret values.
- All state files are project-local (`.codex/compact/`).
- No global agent config edits, no telemetry, no network calls from watch/remind.

After compact:

```bash
aiplus compact resume
```

If Codex does not reply, explicit AiPlus continuation messages such as
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

`../../core/scripts/compactctl.mjs` remains only as archived history and a
compatibility test fixture. It is not the ordinary-user AiPlus CLI (`aiplus`)
path and must not be used as a fallback if `aiplus` is missing.
