# AiPlus v0.5.x Release Scope

**Version:** 0.5.1  
**Status:** Ready for Owner Review  
**Last Updated:** 2026-05-10

---

## Included in v0.5.1

### Compact Reminder Reminder v2
- Compact remind/watch/prepare/resume/savings/checkpoint
- Context capsule creation with checksums
- Signal-safe watch loop (SIGTERM/SIGINT)
- JSON output mode for automation
- **QA:** GLM Rust Lead PASS, MiniMax Runtime QA PASS

### Profile Supplemental Bundle
- Install: USER.md, MEMORY.md, preferences/, identities/, sync/
- Status: Reports supplemental bundle presence
- Doctor: Validates identity files and bundle integrity
- Context: Profile metadata + file counts
- User context: Redacted USER.md snapshot
- **QA:** MiniMax Runtime QA PASS

### Agent Memory Foundation
- Project-local memory store
- Memory context with budget and redaction
- Identity context with role inheritance
- Skill candidate tracking
- Doctor integration
- Q/A transcript redaction (Q: A:, Q. A., Question: Answer:, line-by-line Q/A)
- **QA:** GLM + MiniMax PASS

### Documentation
- Profile Supplemental Bundle mechanics (README.md, README.zh-CN.md)
- Synthetic/redacted examples only
- Release readiness packets

---

## Excluded from v0.5.1

| Item | Reason | Target |
|------|--------|--------|
| AppModules product modules | Not in scope | Future |
| Cloud sync | Not in scope | Future |
| Vector database | Not in scope | Future |
| True daemon/launchd | Not in scope | Future |
| Full transcript auto-learning | Explicitly disabled | Future |
| Automatic approved skills | Owner gate required | Future |
| Payment/voice | Not in scope | Future |
| Telemetry | Explicitly prohibited | Never |
| Global config edits | Explicitly prohibited | Never |
| crates.io publish | Not approved | Future |
| npm publish | Not approved | Future |
| Homebrew release | Not approved | Future |
| Marketplace publish | Not approved | Future |

---

## Release Criteria

- [x] Code quality: fmt, clippy, tests PASS
- [x] Runtime QA: All commands verified
- [x] Secret/private boundary: Scanned and PASS
- [x] Global config: Untouched
- [x] Documentation: Updated
- [x] CHANGELOG: Current

## Post-v0.5.1 Backlog (v2.1)

1. Integrate `load_context_capsule()` into resume path
2. Implement `extract_decisions_from_ledger()`
3. Add defensive redaction before capsule write
4. Enhanced identity TOML schema validation
5. Extended user context redaction patterns
6. Sync policy file parsing validation
