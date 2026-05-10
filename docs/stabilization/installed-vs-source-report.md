# AiPlus Installed vs Source Report

**Date**: 2026-05-10
**Source**: aiplus-public
**Installed Binary**: ~/.local/bin/aiplus v0.5.1

## Purpose

Document differences between installed binary (~/.local/bin/aiplus) and current source (aiplus-public cargo run).

## Version Comparison

| Binary | Version | Source |
|--------|---------|--------|
| ~/.local/bin/aiplus | 0.5.1 | Pre-built |
| cargo run -p aiplus-cli | 0.5.1 | aiplus-public |

## Command Differences

### Memory Subcommands

**Installed binary** (`~/.local/bin/aiplus`):
```
Available: status, doctor, init, context, add, list, recent, search, forget, conflicts
Missing: auto-capture, session, show-used, snapshot (build|profile)
```

**Source binary** (`cargo run -p aiplus-cli`):
```
Available: status, doctor, init, context, add, list, recent, search, forget, conflicts,
          auto-capture, session, show-used, snapshot build
```

### Compact Watch

**Installed binary**:
```
$ aiplus compact --help
compact init|validate|prepare|score|checkpoint|resume|savings [--json] [--level light|standard|full]
# NO --once, --interval flags
```

**Source binary**:
```
$ aiplus compact --help
compact init|validate|prepare|score|checkpoint|resume|remind|savings [--json] [--level light|standard|full]
# HAS --once, --interval flags
```

## Specific Command Comparisons

### `aiplus compact watch --once`

**Installed**: `error: unexpected argument '--once' found`
**Source**: Returns watch output with WATCH_MODE=once

### `aiplus memory auto-capture --text "test"`

**Installed**: Command not found
**Source**: MEMORY_AUTO_CAPTURE, id=auto_*

### `aiplus memory session add-card --summary "test" --text "test"`

**Installed**: Command not found
**Source**: MEMORY_SESSION, id=sess_*

### `aiplus memory show-used`

**Installed**: Command not found
**Source**: MEMORY_SHOW_USED with memory_ids and session_ids

## Implications

1. **Testing Policy**: When testing memory/compact watch features, MUST use `cargo run -p aiplus-cli` from aiplus-public source, not installed binary.

2. **Release Note**: When cutting release, ensure the published binary matches the source that passed QA.

3. **Feature Availability**: auto-capture, session, show-used, watch --once, watch --interval are only in source, not installed binary.

## Verification Commands

```bash
# Check installed version
~/.local/bin/aiplus --version

# Check source version (must run from aiplus-public)
cd /Users/steve/Dropbox/Project/AiPlus/aiplus-public
rtk cargo run -p aiplus-cli --bin aiplus -- --version

# Test memory auto-capture (source only)
cd /Users/steve/Dropbox/Project/AiPlus/aiplus-public
rtk cargo run -p aiplus-cli --bin aiplus -- memory auto-capture --text "test"

# Test watch --once (source only)
cd /Users/steve/Dropbox/Project/AiPlus/aiplus-public
rtk cargo run -p aiplus-cli --bin aiplus -- compact watch --once --json
```

## Conclusion

Source and installed binary are the same version (0.5.1), but installed binary is an older build without watch/interval and some memory features. Always test from source for feature verification.