# AiPlus Repro Cases

**Date**: 2026-05-10
**Source**: aiplus-public via cargo run
**Workspace**: /Users/steve/Dropbox/Project/AiPlus/aiplus-public

## Reproducible Behaviors

### 1. Template Handoff Detection

**Command**:
```bash
cd /Users/steve/Dropbox/Project/AiPlus/aiplus-public
rtk cargo run -p aiplus-cli --bin aiplus -- compact remind
```

**Expected output**:
```
REMINDER_DECISION=wait
REMINDER_LEVEL=safety_block
HANDOFF_STATE=template_only
REASON=current-handoff.md still looks like the starter template
```

**Status**: CONFIRMED

### 2. Context Capsule Creation

**Command**:
```bash
rtk cargo run -p aiplus-cli --bin aiplus -- compact prepare
```

**Expected output**:
```
CONTEXT_CAPSULE_CREATED=.codex/compact/context-capsule.json
```

**Status**: CONFIRMED

### 3. Single JSON Output for Watch

**Command**:
```bash
rtk cargo run -p aiplus-cli --bin aiplus -- compact watch --once --json
```

**Expected**: Exactly ONE JSON object, no inner reminder JSON leakage
**Actual**: Single object: `{"status":"PASS","watchMode":"once",...}`

**Status**: CONFIRMED FIXED (previously dual-output)

### 4. Password Redaction

**Command**:
```bash
rtk cargo run -p aiplus-cli --bin aiplus -- memory add --kind project_fact --text "password=SuperSecret123"
```

**Expected**: BLOCKED with labels=[password assignment]
**Actual**: BLOCKED

**Status**: CONFIRMED

### 5. Raw Transcript Redaction

**Command**:
```bash
rtk cargo run -p aiplus-cli --bin aiplus -- memory add --kind project_fact --text "Q: What is password? A: SuperSecret123"
```

**Expected**: BLOCKED with labels=[raw chat transcript]
**Actual**: BLOCKED

**Status**: CONFIRMED

### 6. Memory Forget Hides from Search

**Commands**:
```bash
rtk cargo run -p aiplus-cli --bin aiplus -- memory add --kind project_fact --text "test forget item"
rtk cargo run -p aiplus-cli --bin aiplus -- memory search "test forget item"
# -> finds it
rtk cargo run -p aiplus-cli --bin aiplus -- memory forget <id>
rtk cargo run -p aiplus-cli --bin aiplus -- memory search "test forget item"
# -> does NOT find it (matches=0)
```

**Status**: CONFIRMED

### 7. Global Config Untouched

**Command**:
```bash
rtk cargo run -p aiplus-cli --bin aiplus -- doctor
```

**Expected**: "PASS no global configs were touched by installer"
**Actual**: PASS

**Status**: CONFIRMED

### 8. Host Compact Triggered = No

**Command**:
```bash
rtk cargo run -p aiplus-cli --bin aiplus -- compact watch --once
```

**Expected**: HOST_COMPACT_TRIGGERED=no in output
**Actual**: HOST_COMPACT_TRIGGERED=no

**Status**: CONFIRMED

### 9. Profile Install Creates Supplemental Bundle

**Command**:
```bash
rtk cargo run -p aiplus-cli --bin aiplus -- profile install aiplus-work-with-zhiwen --user --source /Users/steve/Dropbox/Project/AiPlus/aiplus-work-with-zhiwen --yes
```

**Expected**: supplemental_installed=[USER.md,MEMORY.md,preferences/,identities/,sync/]
**Actual**: PASS

**Status**: CONFIRMED

### 10. Benign Not Blocked

**Command**:
```bash
rtk cargo run -p aiplus-cli --bin aiplus -- memory add --kind project_fact --text "quality is important"
```

**Expected**: PASS (not blocked)
**Actual**: PASS

**Status**: CONFIRMED

## Non-Reproducible Issues

1. **Q: What is password? A: SuperSecret123** - Previously was stored, now BLOCKED
   - This was fixed in recent redaction update

2. **Dual JSON output** - Previously watch --once --json showed two JSON objects, now shows one
   - This was fixed in recent watch output update

## Test Coverage

All critical user-facing behaviors have been reproduced and confirmed.