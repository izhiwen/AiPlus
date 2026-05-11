# Subproduct Drift Report

**Date:** 2026-05-10  
**Mainline:** aiplus-public v0.5.1

---

## Subproduct Inventory

| Subproduct | Status | Version | Relationship to Mainline |
|-----------|--------|---------|------------------------|
| aiplus-compact-reminder | Stable | v0.4.6 | Embedded in aiplus-public as module |
| aiplus-agent-memory | Stable | v0.5.1 | Embedded in aiplus-public as module |
| aiplus-auto-team-consultant | Stable | v0.4.6 | Embedded in aiplus-public as module |
| aiplus-work-with-zhiwen | Private | v0.3.0 | Installed via profile install, never embedded |

---

## Drift Analysis

### No Critical Drift Detected

All subproducts are either:
1. **Embedded in mainline** (compact-reminder, agent-memory, auto-team-consultant) — versions match
2. **Private profile** (work-with-zhiwen) — correctly kept separate

### Source of Truth

| Component | Source of Truth |
|-----------|----------------|
| Rust CLI + Core | `aiplus-public/` |
| Compact Reminder logic | `aiplus-public/crates/aiplus-core/src/compact_state.rs` |
| Agent Memory logic | `aiplus-public/crates/aiplus-core/src/memory*.rs` |
| Private profile content | `aiplus-work-with-zhiwen/` (local only) |

---

## Legacy Directories

| Directory | Status | Action |
|-----------|--------|--------|
| `aiplus-rust/` | Legacy | Do not delete (not in scope) |
| `aiplus-cli/` (Node) | Legacy | Do not delete (reference only) |
| `auto-team-consultant/` | Legacy | Do not delete (not in scope) |
| `work-with-zhiwen` | Symlink | Keep as convenience link |

---

## Recommendations

1. **No drift concerns for v0.5.1 release**
2. Consider consolidating standalone subproduct repos into mainline docs in v2.1
3. Archive truly obsolete legacy directories only with explicit Owner approval
