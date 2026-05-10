# Agent Memory v2.1 Plan

## Goal

Improve Agent Memory trust, usefulness, and Owner experience for v2.1. Focus on low-risk, local changes that strengthen the product without adding cloud, vector DB, daemon, or telemetry.

## Current State (v0.5.1)

- Memory commands: `add`, `search`, `forget`, `context`, `doctor`, `conflicts`, `stale`, `propose`, `review`, `accept`, `reject`, `snapshot`, `profile-sync`, `migrate`
- Storage: JSONL files in `.aiplus/memory/`
- Redaction: Mandatory `reject_sensitive_memory_text()` on write
- Session tracking: SQLite `sessions.sqlite` with FTS
- Profile sync: Unidirectional (profile → project), preferences only

## v2.1 Priorities

### P0: Memory Doctor Deep Scan (Trust)

**Problem:** `memory doctor` checks file existence and format but does not deeply scan active records for redaction, conflicts, or staleness.

**Solution:**
- Integrate `detect_conflicts()` and `detect_stale()` into doctor output
- Add per-record redaction scan: validate every active record summary against `reject_sensitive_memory_text()`
- Add remediation suggestions:
  - "Run `aiplus memory conflicts` to see N unresolved conflicts"
  - "Run `aiplus memory stale` to review N stale records"
  - "N records failed redaction scan — review immediately"

**Files to modify:**
- `crates/aiplus-cli/src/main.rs`: `memory_doctor()`

**Tests:**
- Doctor detects stale records
- Doctor detects conflicts
- Doctor flags redaction failures

---

### P0: Defensive Redaction in Context Capsule (Safety)

**Problem:** `save_context_capsule()` writes capsule to disk without explicit redaction pass. While current code does not embed secrets, a defensive pass would be safer.

**Solution:**
- Before serializing capsule, scan all string fields (objective, resume_prompt, next_safe_action, tier items) for sensitive patterns
- If found, redact to [REDACTED] and log warning
- This is a hardening measure, not a bug fix

**Files to modify:**
- `crates/aiplus-cli/src/main.rs`: `save_context_capsule()`

**Tests:**
- Capsule with sensitive content is redacted before save

---

### P1: Memory Context Scope Filtering (Accuracy)

**Problem:** `memory context` returns all scopes mixed together (session, project, profile, global). No way to filter.

**Solution:**
- Add `--scope session|project|profile|global` flag to `memory context`
- Default to `project` (most common use case)
- Update `select_records()` to accept optional scope filter

**Files to modify:**
- `crates/aiplus-cli/src/main.rs`: `memory_context()`, CLI args
- `crates/aiplus-core/src/memory_context.rs`: `select_records()`

**Tests:**
- `--scope project` returns only project records
- `--scope session` returns only session records

---

### P1: Memory Context Deduplication (Accuracy)

**Problem:** Similar records with identical summary + type may appear multiple times in context output.

**Solution:**
- In `select_records()`, skip records that have identical (summary, record_type) to already-selected records
- Prefer newer record (by updated_at)

**Files to modify:**
- `crates/aiplus-core/src/memory_context.rs`: `select_records()`

**Tests:**
- Duplicate records are deduplicated
- Newer record is preferred

---

### P1: Forget Dry-Run Preview (UX)

**Problem:** `memory forget` immediately changes status to rejected with no preview.

**Solution:**
- Add `--dry-run` flag to `memory forget`
- Show what would be forgotten (ID, summary, type)
- No file modification in dry-run mode

**Files to modify:**
- `crates/aiplus-cli/src/main.rs`: `memory_forget()`, CLI args

**Tests:**
- `--dry-run` shows preview without modifying file

---

### P2: Conflict Resolution Guidance (Actionability)

**Problem:** `memory conflicts` lists conflicts but provides no guidance on how to resolve them.

**Solution:**
- Add resolution guidance to output:
  - `conflict_group_divergence`: "Records in group X have different summaries. Review and decide which is correct, then run `aiplus memory accept <id>`"
  - `missing_superseded`: "Record claims to supersede Y which does not exist. Remove supersede claim or create Y."
  - `circular_supersede`: "Records A and B supersede each other. Break the cycle by removing one supersede claim."
- Add `--auto-resolve-safe` flag: automatically resolve safe cases (e.g., remove missing supersede claims)

**Files to modify:**
- `crates/aiplus-cli/src/main.rs`: `memory_conflicts()`
- `crates/aiplus-core/src/memory_conflict.rs`: `ConflictReport` (add guidance field)

**Tests:**
- Guidance is included in output
- `--auto-resolve-safe` resolves safe cases

---

### P2: Profile Sync Dry-Run (Safety)

**Problem:** `memory profile-sync` immediately writes to project memory with no preview.

**Solution:**
- Add `--dry-run` flag
- Show what would be synced (count, IDs, summaries)
- Show conflicts that would be skipped

**Files to modify:**
- `crates/aiplus-cli/src/main.rs`: `memory_profile_sync()`, CLI args
- `crates/aiplus-core/src/profile_sync.rs`: `ProfileSync::sync_to_project()`

**Tests:**
- `--dry-run` shows preview without writing

---

### P2: Profile Sync Bidirectional (Completeness)

**Problem:** Profile sync is only profile → project. Project decisions should flow back to profile.

**Solution:**
- Add `--direction project-to-profile` flag
- Sync `project_decision` and `owner_preference` records to profile memory
- Maintain redaction checks in both directions

**Files to modify:**
- `crates/aiplus-core/src/profile_sync.rs`: Add `sync_to_profile()` method
- `crates/aiplus-cli/src/main.rs`: `memory_profile_sync()`, CLI args

**Tests:**
- Bidirectional sync works
- Redaction is checked in both directions

---

### P3: Skill Candidate Threshold Tuning (Noise Reduction)

**Problem:** Skill candidates may be noisy — any repeated pattern becomes a candidate.

**Solution:**
- Raise default threshold: 4+ occurrences for normal, 3+ for failures
- Add `--threshold N` flag to `skill-candidate consolidate`
- Require at least one verification evidence record for low-risk candidates

**Files to modify:**
- `crates/aiplus-core/src/skill_candidate.rs`: `ConsolidationEngine`
- `crates/aiplus-cli/src/main.rs`: `skill_candidate_consolidate()`, CLI args

**Tests:**
- Higher threshold reduces candidate count
- Evidence requirement filters low-signal candidates

---

### P3: Session-Based Auto-Capture (Automation)

**Problem:** All memory addition is manual. No automatic extraction from session history.

**Solution:**
- Add `memory auto-capture` command
- Analyze recent sessions from SQLite index
- Propose records for sessions with:
  - >3 decisions
  - >5 files changed
  - >2 blockers resolved
- Output proposed records as JSON for Owner review
- Do NOT auto-insert — Owner must approve each proposal

**Files to modify:**
- `crates/aiplus-cli/src/main.rs`: New `memory_auto_capture_command()`
- `crates/aiplus-core/src/session.rs`: Add query methods

**Tests:**
- Auto-capture proposes records from significant sessions
- Proposals are output, not inserted

---

## Backlog (v0.6+)

- Semantic search (local embeddings evaluation)
- Real-time memory suggestions (session hook)
- Memory compaction (physically remove rejected records)
- Cross-project memory sharing
- Memory visualization (web UI)

## Explicitly Rejected

- Cloud vector DB
- LLM-based extraction
- Automatic memory insertion without approval
- Behavioral tracking / telemetry
- Daemon/launchd for background monitoring

---

## Success Metrics

| Metric | v0.5.1 | v2.1 Target |
|--------|--------|-------------|
| Memory Doctor depth score | 6/10 | 9/10 |
| Context accuracy score | 7/10 | 9/10 |
| Forget UX score | 6/10 | 8/10 |
| Conflict actionability score | 6/10 | 8/10 |
| Profile sync safety score | 7/10 | 8/10 |
| Skill candidate noise score | 5/10 | 7/10 |

---

## Files Expected to Change

- `crates/aiplus-cli/src/main.rs`
- `crates/aiplus-core/src/memory_context.rs`
- `crates/aiplus-core/src/memory_conflict.rs`
- `crates/aiplus-core/src/profile_sync.rs`
- `crates/aiplus-core/src/skill_candidate.rs`
- `crates/aiplus-core/src/session.rs`

## Estimated Effort

- P0 fixes: 1–2 days
- P1 fixes: 2–3 days
- P2 fixes: 3–4 days
- P3 fixes: 2–3 days
- **Total: 8–12 days**
