# G2: Dispatch gate — verb-object semantic parse

**Status**: DRAFT — Owner approval pending.
**Drafted by**: Advisor (Claude Code session, paired with CEO codex)
**Owner**: Steve (izhiwen@icloud.com)
**Date**: 2026-05-16

---

## TL;DR

Current `aiplus agent route` dispatch gate engine triggers `errorReason=owner_gate_pending` whenever the task text **mentions** gate-keywords (publish, release, deploy, global config, external accounts, secret exposure), regardless of whether the text is **describing** those gates or **intending to cross** them.

**Evidence**: 30+ G1 dispatches between 2026-05-15 and 2026-05-16; **~30% canceled with `owner_gate_pending`** purely because the prose talked about the gates. CEO had to paraphrase every time. Real friction during G1 execution.

Goal: change gate engine from **keyword-presence** detection to **verb-object intent** semantic parse.

## Why this matters

- **30% dispatch-redo rate** is a real productivity tax. Every retry costs an LLM round-trip and CEO attention.
- **Worse**: paraphrasing to avoid trigger words means task instructions get **less accurate** over retries — semantic information lost.
- **Worst**: trains CEO to think "if my dispatch fails, just water down the wording" — eroding the gate's actual safety value over time.
- This is a **dogfood** bug: AiPlus shipped a safety mechanism that's so dumb it teaches users to game it.

## Owner decisions (small set, default-recommend)

### D1: Implementation approach

| Option | Tech | Pros | Cons |
|---|---|---|---|
| Rules table ★ | Action-verb-near-gate-word heuristic (e.g. `publish` triggers ONLY if preceded by imperative verb or "I'll/we'll/let me/please") | Fast, deterministic, testable, no extra deps | Requires building negative-example set; some edge cases miss |
| Mini-LLM | Send task text to a small classifier model | Highest accuracy | Adds dependency, latency, cost; failure modes harder to debug |
| Hybrid | Rules first, LLM fallback when rules uncertain | Best of both | Complexity |

**Advisor recommends Rules table** — ships fastest, deterministic, debuggable. If false-positive rate stays > 10% after 2 weeks of dogfood, revisit for LLM.

### D2: Scope of fix

| Option | Coverage |
|---|---|
| Just `aiplus agent route` ★ | Fix the one entry point hit in G1 dispatches |
| All gate-enforcing commands | Broader — but most users only hit `route` |

**Advisor recommends just `route`** for v1. Other entry points get the same fix in follow-up if needed.

### D3: Negative-example test set source

| Option | Source |
|---|---|
| Mine from G1 dispatch-log ★ | Real false-positives from 2026-05-15/16 dispatches |
| Hand-craft ten new ones | Synthetic |

**Advisor recommends mining from G1 dispatch-log** — real-world test set, captures actual paraphrase patterns CEO used.

## Implementation outline (CEO refines)

| Card | Description | Dispatch to | Owner-gates |
|---|---|---|---|
| GT1 | Mine G1 dispatch-log for ≥ 10 false-positives + ≥ 10 true-positives. Build test fixture in `crates/aiplus-cli/tests/`. | qa | none |
| GT2 | Architect spec: verb-object pattern rules table; specify which verbs count as "imperative intent" near gate words; specify negation handling ("never publish") | architect | none |
| GT3 | Implementation: edit gate detection in aiplus-cli source. Add unit tests from GT1 fixture. | engineer-a | none |
| GT4 | Doctor check: add `dispatch_gate=PASS\|FAIL` to `aiplus doctor` output. | engineer-a | none |
| GT5 | Reviewer + QA pass on the implementation, gate-against-fixture verification | reviewer + qa | none |
| GT6 | Integration: commit + push to main. Internal release notes draft. | engineer-a | none |

## Acceptance criteria

1. ✅ False-positive set from G1 dispatch-log (≥ 10 entries): zero re-trigger after fix
2. ✅ True-positive set (≥ 10 real intents): all still gate as expected
3. ✅ `aiplus doctor` shows new check `dispatch_gate=PASS` after fix
4. ✅ Unit tests in repo (red-then-green discipline)
5. ✅ No regression: existing aiplus tests still pass
6. ✅ Verification with one sample heavy dispatch text from G1 that previously canceled — should now succeed without paraphrase

## Owner-gates during G2 execution

- Not gated (CEO + builders can proceed): drafting, implementation, internal commits, internal docs
- Gated (Owner sign-off): version bump for the release containing the fix; release notes external draft

## Out of scope (v1)

- Smarter intent parsing using LLM (revisit in v2 if rules table insufficient)
- Refactor of other gate-enforcing commands (`aiplus install`, `aiplus add`, etc.) — separate goal if needed
- UI for previewing "will this trigger gate?" before sending dispatch

## How CEO picks this up — dispatch command

After Owner sign-off on D1–D3, paste to CEO codex session:

```
aiplus agent route ceo "Implement G2 per spec at docs/proposals/G2-dispatch-gate-semantic.md. Owner signed off on D1-D3 defaults. Break into cards GT1-GT6 per outline. D6 still active: dispatch all builder work, do not write code yourself. Start with GT1 dispatched to qa (mine G1 dispatch-log for false-positive test fixture). Block GT2 on GT1 fixture ready. Iterate GT3 -> GT4 -> GT5 per DAG. GT6 internal commit needs reviewer co-sign before push. Report milestones via team-memory."
```

## Advisor's continuing role

Available for spec clarification when CEO or builders read G2 and have questions. Reviews verification-gate results before claiming done. Does not write code.
