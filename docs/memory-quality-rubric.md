# Memory Quality Rubric

## Purpose

Measure AiPlus Agent Memory trustworthiness, usefulness, and safety across all commands and data flows. Use this rubric to score current state, set v2.1 goals, and validate fixes.

## Scoring

Each dimension is scored 0–10. A score below 5 is a release blocker for that dimension.

---

## Dimensions

### 1. Redaction Safety (P0)

| Score | Definition |
|-------|------------|
| 10 | Zero sensitive values can be written to or read from memory. Every write path has mandatory redaction validation. Every read path re-validates before display. Fails closed. |
| 8 | Write paths validated. Read paths trust stored redaction flag. Minor gap: redaction flag could be tampered. |
| 5 | Write paths validated. Read paths skip re-validation for performance. |
| 0 | No redaction at all. |

**Current Score:** 7
- `reject_sensitive_memory_text()` validates on write (append, rewrite, propose, profile sync)
- `memory_context()` re-validates on read and shows [REDACTED]
- `snapshot.rs` re-validates on read
- **Gap:** `memory_doctor()` checks for sensitive patterns but does not deeply scan every active record's redaction status
- **Gap:** No defensive redaction before writing context capsule (GLM LOW finding)

**v2.1 Target:** 9
- Add deep scan to `memory doctor`: validate every active record summary against redaction rules
- Add redaction validation in `save_context_capsule()` before serialization

---

### 2. Context Accuracy (P0)

| Score | Definition |
|-------|------------|
| 10 | `memory context` returns exactly the records an Owner would expect: accepted, non-sensitive, scope-matched, time-relevant, deduplicated. |
| 8 | Filters rejected/forgotten/stale/expired. Sorts by priority. Budget respected. No scope filtering yet. |
| 5 | Basic filtering works. Budget sometimes exceeded due to formatting overhead. |
| 0 | Returns all records including rejected and sensitive. |

**Current Score:** 7
- `select_records()` filters rejected/forgotten/stale/expired correctly
- Sorts by `type_priority()` (owner_gate > decision > risk > preference > ...)
- Budget in characters is respected (line length check)
- **Gap:** No scope filtering (session/project/profile/global) — all scopes mixed
- **Gap:** No deduplication of similar records
- **Gap:** No time-relevance weighting (recent records not boosted)

**v2.1 Target:** 9
- Add `--scope` filter to `memory context`
- Add recent-record boost (within last N sessions)
- Add deduplication for records with identical summary + type

---

### 3. Doctor Depth (P0)

| Score | Definition |
|-------|------------|
| 10 | Doctor performs a full health audit: schema validation, redaction scan, conflict detection, stale detection, orphan detection, index consistency, session linkage integrity. |
| 7 | Validates file existence, JSONL format, sensitive warnings. No deep record analysis. |
| 5 | File existence + basic format check only. |
| 0 | No doctor command. |

**Current Score:** 6
- Checks file existence and JSONL parse-ability
- Checks for sensitive pattern warnings
- **Gap:** Does not call `detect_conflicts()` or `detect_stale()`
- **Gap:** Does not validate every record against redaction rules
- **Gap:** Does not check index consistency
- **Gap:** Does not report actionable remediation steps

**v2.1 Target:** 9
- Integrate conflict and stale detection into doctor output
- Add per-record redaction scan
- Add remediation suggestions ("Run `aiplus memory conflicts` to see details")

---

### 4. Forget Behavior (P1)

| Score | Definition |
|-------|------------|
| 10 | Forget is permanent, auditable, and recoverable from backup. User understands what was removed. |
| 6 | Status changed to "rejected" but record remains in file. Audit trail present. |
| 3 | Record deleted without audit trail. |
| 0 | No forget command. |

**Current Score:** 6
- `memory_forget()` sets status to "rejected" and rewrites file
- Audit entry appended
- Record still exists in JSONL (just filtered out)
- **Gap:** No physical deletion or compaction
- **Gap:** No "show me what I'll forget" preview

**v2.1 Target:** 8
- Add `--dry-run` preview to forget
- Add `memory compact` command to physically remove rejected/forgotten records (with backup)

---

### 5. Stale/Conflict Actionability (P2)

| Score | Definition |
|-------|------------|
| 10 | Every stale or conflict report includes: why it matters, what to do, and a one-command fix. |
| 6 | Reports list stale records and conflicts with IDs. No remediation guidance. |
| 3 | Reports only counts. |
| 0 | No stale or conflict detection. |

**Current Score:** 6
- `memory conflicts` outputs IDs, types, related IDs
- `memory stale` outputs IDs and reasons
- **Gap:** No "how to resolve" guidance
- **Gap:** No one-command fix (e.g., `aiplus memory resolve <conflict_id>`)
- **Gap:** Conflicts and stale are separate commands; should be unified in doctor

**v2.1 Target:** 8
- Add resolution guidance to output
- Add `--auto-resolve` for safe cases (e.g., expired records -> forgotten)

---

### 6. Profile Sync Safety (P2)

| Score | Definition |
|-------|------------|
| 10 | Bidirectional sync with full conflict resolution, redaction at both ends, and Owner approval for every change. |
| 7 | Unidirectional (profile -> project) with redaction check and duplicate detection. Only preferences synced. |
| 4 | Syncs everything without filtering. |
| 0 | No sync. |

**Current Score:** 7
- `ProfileSync::sync_to_project()` only syncs preference-type records
- Redaction check before writing
- Duplicate detection by ID or (summary + type)
- **Gap:** No bidirectional sync
- **Gap:** No Owner approval gate
- **Gap:** Hardcodes private profile path `aiplus-work-with-zhiwen/profile-memory`

**v2.1 Target:** 8
- Add bidirectional sync option (project -> profile for decisions)
- Add `--dry-run` preview
- Make profile path configurable

---

### 7. Skill Candidate Signal-to-Noise (P3)

| Score | Definition |
|-------|------------|
| 10 | Skill candidates are high-signal, deduplicated, and only proposed when truly novel. Owner can accept/reject with one command. |
| 5 | Candidates generated from pattern grouping. Accept/reject commands exist but are stubs. May be noisy. |
| 0 | No skill candidate system. |

**Current Score:** 5
- `ConsolidationEngine::find_consolidation_candidates()` groups by pattern
- Threshold: 3+ occurrences for normal, 2+ for failures
- `memory accept/reject` are stubs (NOT_IMPLEMENTED)
- **Gap:** High noise potential — any repeated pattern becomes candidate
- **Gap:** No verification evidence required for low-risk candidates

**v2.1 Target:** 7
- Raise threshold or require verification evidence
- Implement `memory accept/reject` with real storage
- Add `--quiet` mode for consolidation

---

## Summary

| Dimension | Current | v2.1 Target | Blocker? |
|-----------|---------|-------------|----------|
| Redaction Safety | 7 | 9 | No |
| Context Accuracy | 7 | 9 | No |
| Doctor Depth | 6 | 9 | No |
| Forget Behavior | 6 | 8 | No |
| Stale/Conflict Actionability | 6 | 8 | No |
| Profile Sync Safety | 7 | 8 | No |
| Skill Candidate Signal-to-Noise | 5 | 7 | No |

**Overall Memory Product Score: 6.3/10**
**v2.1 Target: 8.3/10**

---

## Hermes-Like Gap

Hermes (Claude Code memory) provides:
- Automatic memory extraction from conversation
- Implicit context injection on every message
- Natural language memory search

AiPlus gaps:
- No automatic extraction (manual `memory add` only)
- No implicit injection (explicit `memory context` only)
- No semantic search (exact/keyword only)

See `hermes-like-memory-gap-analysis.md` for full comparison.
