# Compact Guide

AiPlus helps you compact at the right time with the right context. It does not replace the host compact button or `/compact`.

## Key Concept: AiPlus Cannot Trigger Host Compact

AiPlus prepares, reminds, and saves state, but the host agent (Codex, Claude Code, OpenCode) controls when compact actually happens. You or the host agent must press the compact button. AiPlus cannot force compact.

## Workflow

```
remind → prepare → checkpoint → [host compact] → resume
```

1. **Remind**: AiPlus checks if now is a good time to compact.
2. **Prepare**: Validates readiness, creates a context capsule.
3. **Checkpoint**: Saves handoff, decisions, and capsule.
4. **Host compact**: You or the agent triggers compact in the host.
5. **Resume**: AiPlus loads the checkpoint and capsule to continue.

## Commands

### Check if you should compact

```bash
aiplus compact remind
```

Output includes:

- `REMINDER_DECISION`: `remind_now`, `prepare_only`, `wait`, or `blocked`
- `REMINDER_LEVEL`: `ready`, `soft`, or `safety_block`
- `HANDOFF_STATE`: `current`, `stale`, or `template_only`
- `RECOVERY_CONFIDENCE`: `high`, `medium`, or `low`
- `ESTIMATED_TOKENS_SAVED`, `ESTIMATED_USD_SAVED`
- `SECRET_VALUES_PRINTED=no`

If `REMINDER_DECISION=remind_now`, it is a good time to compact.

### With an event trigger

```bash
aiplus compact remind --event phase-end
aiplus compact remind --event long-session
```

### Prepare for compact

```bash
aiplus compact prepare
```

Creates `.codex/compact/context-capsule.json` with:

- Objective, current state, next actions
- Hot/warm/cold tier items
- Owner gates, risks, decisions
- Deterministic checksum
- Safety markers: `secretValuesPrinted=false`, `rawTranscriptCaptured=false`

`READINESS_STATE` can be `READY_TO_COMPACT`, `UNKNOWN_NEEDS_REVIEW`, or `BLOCKED`.

### Save a checkpoint

```bash
aiplus compact checkpoint
```

Saves the current handoff, decisions, and state. Use `--level light|standard|full` for different detail levels.

### Resume after compact

```bash
aiplus compact resume
```

Loads the checkpoint and context capsule. The agent should continue from where it left off.

### Watch mode (automated)

```bash
aiplus compact watch --once --json
aiplus compact watch --interval 60s --json
```

Watch mode runs `compact remind` at intervals. `--once` runs a single check. `--json` emits one JSON object per iteration. The process handles SIGINT (Ctrl+C) and SIGTERM cleanly.

### Savings estimate

```bash
aiplus compact savings
```

Shows token and USD savings for the current compact and all-time totals. Estimates use bundled public pricing data; no API keys or billing access required.

### Validate compact state

```bash
aiplus compact validate
```

Checks structural validity of compact files. Passing validation does not mean safe to compact — use `prepare` for readiness.

### Initialize compact files

```bash
aiplus compact init
```

Creates `.codex/compact/` with template files. Safe to re-run.

## Context Capsule

The context capsule (`.codex/compact/context-capsule.json`) is created by `compact prepare` and contains everything the agent needs to resume:

| Field | Purpose |
|---|---|
| `objective` | Current goal from handoff |
| `currentState` | Current phase |
| `hot` | Must-keep items (max 20) |
| `warm` | Important items (max 50) |
| `cold` | Reference items (max 100) |
| `decisions` | Key decisions (currently stub, planned for v2.1) |
| `ownerGates` | Actions needing owner approval |
| `redaction` | Safety markers |

## Snooze

```bash
aiplus compact remind --snooze 30m
aiplus compact remind --clear-snooze
```

Snooze suppresses reminders for the specified duration.

## Safety

- `HOST_COMPACT_TRIGGERED=false` — AiPlus never triggers host compact
- `SECRET_VALUES_PRINTED=no` — never in output
- `RAW_TRANSCRIPT_CAPTURED=no` — never stored
- Context capsule text is sourced from project-local handoff files only
