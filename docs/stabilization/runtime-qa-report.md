# AiPlus Runtime QA Report - Overnight Session

**Date**: 2026-05-10
**Source**: aiplus-public via cargo run
**Installed Binary**: ~/.local/bin/aiplus v0.5.1 (reference only)
**QA Lead**: Runtime QA

## Executive Summary

All critical paths verified PASS from current source. Auto Compact v2, Profile Supplemental Bundle, and Agent Memory Foundation all function correctly with proper redaction, no global config edits, and no secret exposure.

## Test Matrix

### 1. Install/Status/Doctor (PASS)

All install dry-runs pass. Doctor confirms:
- Module manifest present
- Runtime adapters (codex, opencode) supported
- AGENTS.md contains exactly one managed block
- .opencode/opencode.json is valid strict JSON
- No global configs touched

### 2. Auto Compact (PASS)

Commands tested:
- `compact remind` - returns wait with template handoff detection
- `compact remind --json` - single JSON object, no secrets
- `compact watch --once` - single iteration, no transcript capture
- `compact watch --once --json` - single JSON object
- `compact validate` - PASS
- `compact prepare` - creates context-capsule.json
- `compact checkpoint` - creates checkpoint
- `compact resume` - RESUME_READY

Key verified:
- HOST_COMPACT_TRIGGERED=no
- SECRET_VALUES_PRINTED=no
- RAW_TRANSCRIPT_CAPTURED=no
- CONTEXT_CAPSULE_STATUS=updated

### 3. Profile Bundle (PASS)

Commands tested:
- `profile install` - installs USER.md, MEMORY.md, preferences/, identities/, sync/
- `profile status` - all 5 components present
- `profile doctor` - 7 identity files validated
- `profile context` - shows v0.3.0, owner=Zhiwen
- `user context --profile` - shows USER.md content
- `identity list` - advisor, ceo, reviewer, builder
- `identity context --role ceo` - correct role contract

Verified:
- Secret values: none
- Global agent config edits: none
- Supplemental bundle fully installed

### 4. Agent Memory (PASS)

Commands tested:
- `memory init --project` - creates project structure
- `memory status` - shows 0 records in clean project
- `memory doctor` - PASS, all files present
- `memory add` - successfully adds project_fact
- `memory search` - finds added records
- `memory context` - shows sources and records_used
- `memory auto-capture` - id=auto_*, risk_level=low
- `memory session add-card` - id=sess_*
- `memory show-used` - shows memory_ids and session_ids
- `memory snapshot build` - creates MEMORY.md
- `memory stale` - 0 stale records
- `memory conflicts` - 0 unresolved

### 5. Redaction (PASS)

All sensitive patterns blocked:
- password=, password:, passwd= - BLOCKED (password assignment)
- secret= - BLOCKED (secret assignment)
- User:/Assistant: - BLOCKED (raw chat transcript)
- Human:/AI: - BLOCKED (phone pii, raw chat transcript)
- Q:/A: - BLOCKED (raw chat transcript)
- Question:/Answer: - BLOCKED (raw chat transcript)

Benign NOT blocked:
- "quality is important" - PASS

### 6. Regression Tests (PASS)

- parity: 26 passed
- continuity: 5 passed
- redaction: 11 passed

## Safety Verification

| Check | Result |
|-------|--------|
| global config touched | NO |
| secret values printed | NO |
| raw transcript stored | NO |
| host compact triggered | NO |
| telemetry | NO |
| external accounts | NO |

## Findings

1. All Auto Compact commands correctly report HOST_COMPACT_TRIGGERED=no
2. All memory commands correctly report secretValues=none
3. Watch --once --json outputs single JSON object (fixed from earlier dual-output issue)
4. Context capsule correctly created at .codex/compact/context-capsule.json
5. Profile supplemental bundle fully installed to ~/.config/aiplus/profiles/
6. Rejected/forgotten memory records correctly hidden from search

## Conclusion

VERDICT=PASS
All critical paths functional. No safety violations detected.