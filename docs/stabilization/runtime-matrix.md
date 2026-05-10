# AiPlus Runtime QA Matrix

**Date**: 2026-05-10
**Source**: aiplus-public via `cargo run -p aiplus-cli`
**Installed Binary**: ~/.local/bin/aiplus v0.5.1 (reference only)
**Workspace**: /Users/steve/Dropbox/Project/AiPlus/aiplus-public

## Test Results Summary

| Test | Status | Notes |
|------|--------|-------|
| aiplus --help | PASS | Shows all commands |
| aiplus status | PASS | Reports installed modules |
| aiplus refresh | PASS | Shows AiPlus status |
| aiplus doctor | PASS | 37 checks, all PASS |
| install codex --dry-run | PASS | No files changed |
| install claude-code --dry-run | PASS | No files changed |
| install opencode --dry-run | PASS | No files changed |
| install all --dry-run | PASS | No files changed |

## Auto Compact Results

| Test | Status | Notes |
|------|--------|-------|
| compact remind | PASS | REMINDER_DECISION=wait, template handoff detected |
| compact remind --json | PASS | Single JSON object |
| compact watch --once | PASS | WATCH_MODE=once |
| compact watch --once --json | PASS | Single JSON object |
| compact validate | PASS | VALIDATION_PASS |
| compact prepare | PASS | CONTEXT_CAPSULE_CREATED |
| compact checkpoint | PASS | Checkpoint created |
| compact resume | PASS | RESUME_READY |

**Key findings**:
- HOST_COMPACT_TRIGGERED=no in all outputs
- SECRET_VALUES_PRINTED=no in all outputs
- RAW_TRANSCRIPT_CAPTURED=no in watch output
- Context capsule created at .codex/compact/context-capsule.json
- Template handoff correctly returns wait decision

## Profile Bundle Results

| Test | Status | Notes |
|------|--------|-------|
| profile install | PASS | USER.md, MEMORY.md, preferences/, identities/, sync/ installed |
| profile status | PASS | All components present |
| profile doctor | PASS | 7 identity files validated |
| profile context | PASS | Shows v0.3.0, owner=Zhiwen |
| user context --profile | PASS | USER.md content shown |
| identity list | PASS | advisor, ceo, reviewer, builder |
| identity context --role ceo | PASS | Correct role contract |

## Memory Results

| Test | Status | Notes |
|------|--------|-------|
| memory init --project | PASS | Creates .aiplus/memory, identities, skills, restore |
| memory status | PASS | 0 records active (clean project) |
| memory doctor | PASS | All 7 files present |
| memory add | PASS | Adds project_fact |
| memory search | PASS | Finds added records |
| memory context | PASS | Shows sources and records |
| memory auto-capture | PASS | id=auto_*, risk_level=low |
| memory session add-card | PASS | id=sess_* |
| memory show-used | PASS | Shows memory_ids and session_ids |
| memory snapshot build | PASS | Creates .aiplus/memory/MEMORY.md |
| memory stale | PASS | 0 stale records |
| memory conflicts | PASS | 0 unresolved |

## Redaction Results

| Test | Status | Notes |
|------|--------|-------|
| password=SuperSecret123 | BLOCKED | password assignment |
| password: SuperSecret123 | BLOCKED | password assignment |
| passwd=SuperSecret123 | BLOCKED | password assignment |
| secret=SuperSecret123 | BLOCKED | secret assignment |
| User: ... Assistant: ... | BLOCKED | raw chat transcript |
| Human: ... AI: ... | BLOCKED | phone pii, raw chat transcript |
| Q: ... A: ... | BLOCKED | raw chat transcript |
| Question: ... Answer: ... | BLOCKED | raw chat transcript |
| quality is important | PASS | Not blocked (benign) |

## Regression Tests

| Test | Status | Notes |
|------|--------|-------|
| parity (21 tests) | PASS | All passed |
| continuity (5 tests) | PASS | All passed |
| redaction (11 tests) | PASS | All passed |

## Safety Boundaries

- globalAgentConfigEdits=none confirmed
- secretValues=none confirmed
- No raw transcript stored
- No host compact triggered
- No global config edits

## Commands Run

All commands executed with:
```bash
cd /Users/steve/Dropbox/Project/AiPlus/aiplus-public
rtk cargo run -p aiplus-cli --bin aiplus -- <command>
```