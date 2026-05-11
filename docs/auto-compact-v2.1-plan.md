# Compact Reminder v2.1 Plan

## Goal

Improve Compact Reminder Reminder effectiveness, reduce duplication with host compact, and make resume truly useful by reading from the context capsule. Focus on low-risk, local changes.

## Current State (v0.5.1)

- Commands: `remind`, `watch`, `prepare`, `resume`, `checkpoint`, `score`, `savings`, `init`, `validate`
- Handoff: Markdown `current-handoff.md` with structured sections
- Capsule: JSON `context-capsule.json` with hot/warm/cold tiers
- Resume: Reads from handoff.md (ignores capsule)
- Savings: Estimated based on heuristics, not actual measurements

## v2.1 Priorities

### P0: Resume Reads Context Capsule (Critical)

**Problem:** `compact resume` reads from `current-handoff.md`, not `context-capsule.json`. The capsule is created but unused. This is the highest-impact v2.1 fix.

**Solution:**
1. Implement `extract_decisions_from_ledger()` to parse `decision-log.md`:
   - Extract decisions with timestamps, descriptions, and approval status
   - Populate `capsule.decisions` array
2. Implement `extract_risks_from_handoff()` to parse risks section
3. Implement `extract_verification_from_ledger()` to parse evidence
4. Modify `compact_resume()`:
   - Try to load `context-capsule.json` first
   - If present, read objective, current_state, decisions, risks, next_safe_action from capsule
   - Fall back to `current-handoff.md` if capsule missing or malformed
   - Add `--source=handoff|capsule|auto` flag (default: auto)

**Files to modify:**
- `crates/aiplus-cli/src/main.rs`: `compact_resume()`, `extract_decisions_from_ledger()`, `build_context_capsule_from_handoff()`

**Tests:**
- Resume reads from capsule when present
- Resume falls back to handoff when capsule missing
- Decisions are extracted from decision-log.md

---

### P0: Decision Extraction from Ledger (Critical)

**Problem:** `extract_decisions_from_ledger()` is a stub returning `Vec::new()`. The capsule always has empty decisions.

**Solution:**
- Parse `.codex/compact/decision-log.md`:
  - Look for lines starting with `- [DECISION]` or `- [APPROVED]` or `- [PENDING]`
  - Extract: description, timestamp (if present), status (APPROVED/PENDING/DENIED)
- Parse `.aiplus/memory/decisions.jsonl`:
  - Read MemoryRecord entries with type `project_decision`
  - Map to `CapsuleDecision` format

**Files to modify:**
- `crates/aiplus-cli/src/main.rs`: `extract_decisions_from_ledger()`

**Tests:**
- Decisions are extracted from markdown ledger
- Decisions are extracted from JSONL
- Empty ledger returns empty Vec

---

### P1: Watch Deduplication and Escalation (UX)

**Problem:** `compact watch --interval` outputs the same recommendation repeatedly. No escalation or quiet mode.

**Solution:**
- Add `--quiet-unless-changed` flag: only output when recommendation differs from last
- Track last recommendation in memory (in-memory only, no persistence)
- Add escalation logic:
  - If 3+ consecutive "wait" recommendations, escalate to "standard" level
  - If 5+ consecutive, escalate to "full" level
  - Reset on "proceed" or snooze
- Display savings accumulator: "Estimated savings so far: X tokens ($Y)"

**Files to modify:**
- `crates/aiplus-cli/src/main.rs`: `compact_watch()`, `compact_remind()`

**Tests:**
- `--quiet-unless-changed` suppresses identical output
- Escalation triggers after N consecutive waits
- Savings accumulator displayed

---

### P1: Snooze Persistence (UX)

**Problem:** Snooze state is not persisted across `aiplus` invocations.

**Solution:**
- Store snooze state in `.codex/compact/reminder-state.json`
- Check persisted snooze in `compact_remind()` before evaluating
- Clear snooze automatically when it expires

**Files to modify:**
- `crates/aiplus-cli/src/main.rs`: `compact_snooze_status()`, `compact_remind()`
- `crates/aiplus-core/src/compact_state.rs`: `ReminderState`

**Tests:**
- Snooze is persisted to file
- Snooze is respected across invocations
- Expired snooze is cleared

---

### P2: Host Compact Trigger Helper (Integration)

**Problem:** AiPlus does not trigger host compact. Owner must manually run `/compact` in agent, then `aiplus compact prepare`.

**Solution:**
- Add `aiplus compact trigger-host --dry-run` command
- `--dry-run`: Shows what would be suggested to Owner ("Run `/compact` in your agent")
- Without `--dry-run`: Prints instructions and checks if host compact appears to have been run recently
- Do NOT actually trigger host compact (AiPlus has no access to agent internals)
- This is a workflow helper, not an automation

**Files to modify:**
- `crates/aiplus-cli/src/main.rs`: New `compact_trigger_host()`

**Tests:**
- `--dry-run` shows instructions without side effects
- Command detects recent host compact by checking checkpoint age

---

### P2: Savings Estimation Linked to Host Compact (Accuracy)

**Problem:** Savings estimates are heuristic, not linked to actual host compact metadata.

**Solution:**
- When host compact checkpoint exists, read its metadata (if available)
- Use actual checkpoint size/timestamp for "before" measurement
- Compare with current context size for "after" measurement
- If no host compact metadata, fall back to heuristic
- Add `savings_accuracy` field to output: `heuristic` | `measured`

**Files to modify:**
- `crates/aiplus-cli/src/main.rs`: `compact_savings()`

**Tests:**
- Measured savings when host compact metadata available
- Heuristic fallback when metadata unavailable

---

### P2: Owner Gate Preservation in Capsule (Trust)

**Problem:** Owner gates are extracted from handoff but not clearly preserved in capsule.

**Solution:**
- Ensure all Owner gates (APPROVED, PENDING, DENIED) are stored in `capsule.owner_gates`
- On resume, re-evaluate gates: if any gate status changed, warn Owner
- Add `gate_status_changed` flag to resume output

**Files to modify:**
- `crates/aiplus-cli/src/main.rs`: `extract_owner_gates_from_handoff()`, `compact_resume()`
- `crates/aiplus-core/src/capsule.rs`: `CapsuleOwnerGate`

**Tests:**
- All gate statuses preserved in capsule
- Resume warns on gate status change

---

### P3: Compact Workflow Documentation (Clarity)

**Problem:** The two-step workflow (host compact + AiPlus prepare) is not clearly documented in CLI output.

**Solution:**
- Add `compact workflow` command that prints the recommended workflow
- Include in `compact remind` output: "Next: run `/compact` in your agent, then `aiplus compact prepare`"
- Add one-line value prop to all compact outputs:
  "AiPlus adds Owner gates, structured handoff, and context capsule to host compact."

**Files to modify:**
- `crates/aiplus-cli/src/main.rs`: New `compact_workflow()`, `compact_remind()`

**Tests:**
- `compact workflow` prints clear instructions
- Value prop appears in remind output

---

## Backlog (v0.6+)

- Automatic watch daemon (evaluated, rejected for local-first principle)
- Cross-project capsule sharing
- Capsule visual diff (before/after)
- Integration with CI/CD for pre-deploy compact checks
- Team-shared Owner gates

## Explicitly Rejected

- True daemon/launchd (local-first principle)
- Cloud sync for capsules
- Automatic host compact trigger (AiPlus has no agent control)
- Telemetry on compact frequency or savings

---

## Success Metrics

| Metric | v0.5.1 | v2.1 Target |
|--------|--------|-------------|
| Resume uses capsule | No | Yes |
| Decisions in capsule | 0 | >0 (if ledger has decisions) |
| Watch quiet mode | No | Yes |
| Snooze persistence | No | Yes |
| Host compact integration | None | Helper command |
| Savings accuracy | Heuristic only | Heuristic + measured |
| Workflow clarity | Medium | High |

---

## Files Expected to Change

- `crates/aiplus-cli/src/main.rs`
- `crates/aiplus-core/src/compact_state.rs`
- `crates/aiplus-core/src/capsule.rs`

## Estimated Effort

- P0 fixes: 3–4 days
- P1 fixes: 2–3 days
- P2 fixes: 3–4 days
- P3 fixes: 1 day
- **Total: 9–12 days**

---

## Release Dependency

Compact Reminder v2.1 should be released **after** Agent Memory v2.1 stabilizes, because:
- Resume reads from capsule which may reference memory records
- Decision extraction depends on memory decision-log format
- Profile sync enhancements affect compact context injection

**Recommended sequence:**
1. Agent Memory v2.1 (8–12 days)
2. Compact Reminder v2.1 (9–12 days)
3. Combined QA and docs update (3–5 days)
4. Release (pending Owner approval)
