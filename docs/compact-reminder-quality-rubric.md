# Compact Reminder Quality Rubric

## Purpose

Measure Auto Compact Reminder effectiveness: does it prompt at the right time, provide useful guidance, avoid noise, and improve resume success?

## Scoring

Each dimension is scored 0–10. A score below 5 is a release blocker.

---

## Dimensions

### 1. Remind Timing (P0)

| Score | Definition |
|-------|------------|
| 10 | Remind fires exactly when Owner would want: after significant work, before context loss, or on explicit request. Never interrupts flow. Never misses critical moments. |
| 7 | Manual remind works well. Watch mode provides periodic checks but requires explicit invocation. |
| 4 | Remind is noisy or misses important moments. |
| 0 | No remind. |

**Current Score:** 7
- `aiplus compact remind` evaluates handoff freshness, readiness, and recommends action
- Watch mode supports `--once` and `--interval` but is not automatic (must be manually started)
- Snooze mechanism exists but not persisted across invocations
- **Gap:** No automatic trigger after N tokens or N commands
- **Gap:** No integration with session index to detect "significant work"

**v2.1 Target:** 8
- Add session-based trigger: remind after session with >N decisions or >M files changed
- Persist snooze state across invocations

---

### 2. Watch Value vs Noise (P1)

| Score | Definition |
|-------|------------|
| 10 | Watch is a valuable background assistant: tells you exactly when to compact, why, and what you'll save. Never repeats the same message. Respects Owner preferences. |
| 6 | Watch outputs readiness state and recommendation every interval. No deduplication. No preference learning. |
| 3 | Watch outputs raw JSON with no interpretation. |
| 0 | No watch. |

**Current Score:** 6
- `compact watch --once --json` outputs clean single JSON
- `compact watch --interval` runs repeatedly with full output
- **Gap:** No deduplication of identical recommendations across intervals
- **Gap:** No escalation (soft -> standard -> full) based on repeated checks
- **Gap:** No "quiet unless changed" mode

**v2.1 Target:** 8
- Add `--quiet-unless-changed` mode: only output when recommendation changes
- Add escalation tracking across watch iterations
- Add cost savings accumulator display

---

### 3. Context Capsule Resume Value (P1)

| Score | Definition |
|-------|------------|
| 10 | Resume reads from context capsule and perfectly reconstructs Owner's working state: decisions, risks, next actions, and blockers. Better than reading handoff markdown. |
| 4 | Capsule is created with correct schema but resume ignores it, reading handoff markdown instead. Decisions array is always empty. |
| 0 | No capsule. |

**Current Score:** 4
- `compact prepare` creates `context-capsule.json` with valid schema
- `load_context_capsule()` implemented but `#[allow(dead_code)]`
- `compact resume()` reads from `current-handoff.md`, not capsule
- `extract_decisions_from_ledger()` is a stub returning empty Vec
- **Gap:** Resume does not use capsule at all (GLM LOW finding)

**v2.1 Target:** 9
- `compact resume` reads from `context-capsule.json` as primary source
- `extract_decisions_from_ledger()` parses `decision-log.md`
- Populate capsule decisions, risks, verification from parsed sources
- Add `--source=handoff|capsule` option for gradual migration

---

### 4. Host Compact Deduplication (P2)

| Score | Definition |
|-------|------------|
| 10 | AiPlus compact complements host compact perfectly: adds Owner gates, context capsule, and savings tracking without duplicating host's core function. One clear workflow. |
| 6 | AiPlus provides additional structure but some overlap exists. Owner may be confused about when to use host vs AiPlus compact. |
| 3 | Heavy duplication. Owner does host compact then repeats steps in AiPlus. |
| 0 | No integration. |

**Current Score:** 6
- Host compact (Codex/Claude) creates checkpoint + summary
- AiPlus adds: handoff markdown, readiness scoring, Owner gates, context capsule, savings ledger
- **Gap:** No automatic integration (AiPlus does not trigger host compact)
- **Gap:** Workflow is sequential but not clearly documented: host compact first, then AiPlus prepare
- **Gap:** Savings estimation is manual, not linked to actual host compact

**v2.1 Target:** 8
- Document clear two-step workflow in `host-compact-vs-aiplus-compact.md`
- Add `aiplus compact trigger-host` command (dry-run safe)
- Link savings estimation to actual host compact metadata

---

### 5. AiPlus Unique Value Clarity (P2)

| Score | Definition |
|-------|------------|
| 10 | Every Owner immediately understands why AiPlus compact exists and what it adds that host compact cannot. Value proposition is one sentence. |
| 6 | Value is documented but scattered. Owner gates and capsule are the unique additions. |
| 3 | Value unclear — seems like a wrapper around host compact. |
| 0 | No perceived value. |

**Current Score:** 6
- Unique features: Owner gates, structured handoff, context capsule, savings tracking
- Documented in README and protocol docs
- **Gap:** Value prop not stated in one clear sentence
- **Gap:** No side-by-side comparison with host compact

**v2.1 Target:** 8
- Add one-line value prop to all compact outputs
- Create `host-compact-vs-aiplus-compact.md` comparison doc

---

### 6. Savings Estimation Accuracy (P3)

| Score | Definition |
|-------|------------|
| 10 | Savings estimates are based on actual token usage, accurate pricing, and real before/after measurements. |
| 5 | Estimates use cached pricing and heuristics. Reasonable order of magnitude but not precise. |
| 2 | Estimates are wild guesses. |
| 0 | No estimation. |

**Current Score:** 5
- `compact_savings()` uses cached pricing catalog
- Estimates token reduction based on heuristic (context size before/after)
- **Gap:** No actual before/after measurement
- **Gap:** Pricing cache may be stale
- **Gap:** Model detection confidence is basic

**v2.1 Target:** 7
- Add actual token count measurement (if API allows)
- Add pricing cache age warning
- Improve model detection

---

## Summary

| Dimension | Current | v2.1 Target | Blocker? |
|-----------|---------|-------------|----------|
| Remind Timing | 7 | 8 | No |
| Watch Value vs Noise | 6 | 8 | No |
| Context Capsule Resume | 4 | 9 | **Yes — resume ignores capsule** |
| Host Compact Deduplication | 6 | 8 | No |
| Unique Value Clarity | 6 | 8 | No |
| Savings Estimation | 5 | 7 | No |

**Overall Compact Product Score: 5.7/10**
**v2.1 Target: 8.0/10**

---

## Critical v2.1 Fix

**Resume must read from context capsule.** This is the single highest-impact change for v2.1.

Current flow:
1. Host compact → checkpoint
2. `aiplus compact prepare` → creates capsule (but resume ignores it)
3. `aiplus compact resume` → reads handoff.md

Desired flow:
1. Host compact → checkpoint
2. `aiplus compact prepare` → creates capsule with decisions, risks, next actions
3. `aiplus compact resume` → reads capsule as primary source, falls back to handoff
