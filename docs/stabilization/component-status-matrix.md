# Component Status Matrix

**Version:** 0.5.1  
**Date:** 2026-05-10

---

| Component | Status | Tests | QA | Owner | Notes |
|-----------|--------|-------|-----|-------|-------|
| **aiplus-core** | PASS | 116 pass | GLM+MiniMax | Memory+Compact | All modules stable |
| **aiplus-cli** | PASS | 26 parity + 5 continuity | GLM+MiniMax | Platform CEO | Main entry point |
| **Auto Compact** | PASS | Full coverage | GLM+MiniMax | Memory+Compact | v2 ready |
| **Profile Install** | PASS | Runtime verified | MiniMax | Platform CEO | Supplemental bundle |
| **Profile Status** | PASS | Runtime verified | MiniMax | Platform CEO | Shows all components |
| **Profile Doctor** | PASS | Runtime verified | MiniMax | Platform CEO | Identity validation |
| **Profile Context** | PASS | Runtime verified | MiniMax | Platform CEO | File counts only |
| **User Context** | PASS | Runtime verified | MiniMax | Platform CEO | Line-by-line redaction |
| **Memory Add** | PASS | Runtime verified | GLM+MiniMax | Memory+Compact | Q/A redaction fixed |
| **Memory Context** | PASS | Runtime verified | GLM+MiniMax | Memory+Compact | Budget + redaction |
| **Memory Doctor** | PASS | Runtime verified | GLM+MiniMax | Memory+Compact | Structural checks |
| **Identity Context** | PASS | Runtime verified | GLM+MiniMax | Memory+Compact | Role inheritance |
| **Secret Broker** | PASS | Runtime verified | GLM+MiniMax | Platform CEO | Metadata-only output |
| **Redaction Engine** | PASS | 11 tests | GLM+MiniMax | Memory+Compact | Q/A + chat + secret |
| **Docs (README)** | PASS | Spot checks | Docs Lead | Docs Lead | Both languages |
| **Release Automation** | PASS | Dry-run | Release Auto | Release Auto | Local only |

---

## Dependency Graph

```
aiplus-cli
├── aiplus-core
│   ├── memory
│   ├── identity
│   ├── redaction
│   ├── profile_sync
│   ├── compact_state
│   └── snapshot
├── clap (CLI parsing)
└── toml (profile parsing)
```

---

## Risk Heat Map

| Component | Code Risk | QA Risk | Release Risk |
|-----------|-----------|---------|--------------|
| Auto Compact | LOW | LOW | LOW |
| Profile Bundle | LOW | LOW | LOW |
| Agent Memory | LOW | LOW | LOW |
| Secret Broker | MEDIUM | LOW | LOW |
| Redaction | LOW | LOW | LOW |
| Docs | LOW | LOW | LOW |

All components rated LOW risk for v0.5.1 release.
