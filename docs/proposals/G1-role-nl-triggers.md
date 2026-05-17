# G1: Natural-language role-trigger auto-binding

**Status**: APPROVED — Owner signed off on D1–D6 on 2026-05-15. Ready for CEO pickup.
**Drafted by**: Advisor (Claude Code session, paired with CEO codex)
**Owner**: Steve (izhiwen@icloud.com)
**Date**: 2026-05-15 (drafted), 2026-05-15 (approved)

---

## TL;DR

In any AiPlus-installed project (AiPlus, AEL, future ones), when Owner says natural language like **"你是 CEO"** / **"you are PI"** / **"以 CEO 的视角看一下"** in a codex/claude-code/opencode session, the session must auto-execute:

1. **Detect** the role-bind intent (vs rhetorical mention) — semantic intent classification, not rigid phrase match
2. **Check** for existing pinned role in session — if already bound, **refuse** with helpful error (per D3)
3. **Activate** identity (`aiplus identity context --role <X>`)
4. **Load memory** at the scope dictated by D2 (role personal + team for builders; + project for coordinators)
5. **Acknowledge** in one line with structured `ROLE_ACTIVATED` payload so Owner + downstream tools know it fired

Zero CLI typing required from Owner. Works across **codex, claude-code, AND opencode** (D4).

Ships for **AiPlus team-agent roles AND AEL roles simultaneously** (D5).

## Why this matters

- **Friction kill**: Owner currently must remember `aiplus identity context --role X`. Forgetting it leaves the session in a non-role default state — and most users won't notice. Result: AEL/AiPlus's signature feature (role separation) silently fails.
- **Dogfood**: NL triggers are AiPlus's product story. If our own product doesn't ship them cleanly, we can't ask others to.
- **Cross-project consistency**: same trigger surface across AiPlus self-dev AND AEL research. Future projects inherit for free.
- **Opencode gap closure**: prior N=15 trigger study showed opencode 0/5 — AGENTS.md ignored entirely. D4 closes this gap as part of v1; we cannot ship a "all-runtimes" story while one runtime is broken.

## Owner decisions (signed off)

### D1: Trigger detection — semantic intent classifier (Medium tier baseline + LLM intent understanding)

**Owner decision**: Medium phrase coverage AS A FLOOR, with the agent expected to apply semantic intent understanding above that floor. The phrase list is the minimum guaranteed match; the agent should also recognize paraphrases the Owner did not anticipate.

Floor phrase list (must match exactly):
- `你是 <role>` / `you are <role>`
- `开 <role>` / `做 <role>` / `take <role>` / `take the <role> role`
- `转 <role>` / `switch to <role>`

Semantic ceiling (agent should also catch — examples, not exhaustive):
- `以 CEO 的视角看一下`
- `let me hear from the PI`
- `戴上 CEO 帽子`
- `我要 advisor 的意见`

**Negative examples (must NOT trigger)**:
- `你是 CEO 吗？` (rhetorical question)
- `> 你是 CEO` (inside quote block)
- `the CEO said X` (third-person reference)
- `CEO 这个角色其实有点鸡肋` (talking ABOUT the role, not requesting it)

**How to implement** (CEO + engineer-a + ai-integration to spec):
- The AGENTS.md / CLAUDE.md / opencode-equivalent prompts the agent with: (1) the floor list as hard match, (2) examples + negative examples as in-context learning, (3) instruction: "when uncertain whether user wants role bind, ASK once before binding"
- No regex / rule engine — rely on the LLM's own intent understanding, primed by the catalog text

### D2: Memory load scope on activation

- **Builders** (engineer-a, engineer-b, qa, reviewer, architect, pm, ra-stata, ra-python, theorist, referee, replicator, etc.): load **role-personal + team memory**
- **Coordinator roles** (CEO, PI, Advisor): load **role-personal + team + project memory**

Concretely: after `aiplus identity context --role X` returns PASS, agent must call:
- `aiplus memory list --role X --scope personal` (always)
- `aiplus memory list --scope team` (always)
- `aiplus memory list --scope project` (only if X ∈ {ceo, pi, advisor})

Output goes into agent's working context. The `ROLE_ACTIVATED` ack message must report counts: `memory_personal=N memory_team=M memory_project=P|null`.

### D3: Mid-session role switch — REFUSE

If session has already activated a role and a new role-bind intent is detected, the agent must:
1. NOT silently switch
2. NOT silently ignore
3. Emit: `ROLE_BIND_REFUSED current_role=<X> requested_role=<Y> reason=session_already_bound`
4. Tell Owner: "Already in `<X>` mode. To switch to `<Y>`: (a) reopen session, or (b) run `aiplus identity context --role <Y>` to override manually."

Rationale: clean identity boundary, no role state drift mid-session.

### D4: All three runtimes (codex + claude-code + opencode)

| Runtime | Baseline (N=15 study) | v1 target |
|---|---|---|
| claude-code | 5/5 NL recognition | 5/5 (no regression) |
| codex | 4/5 (mixed hard/soft) | ≥ 4/5 (no regression) |
| opencode | **0/5 (main agent ignores AGENTS.md)** | **≥ 3/5 (close the gap)** |

**D4 v1 local verification snapshot (2026-05-17, current main worktree):**
- Codex positives: **10/10 PASS**.
- Claude positives: **10/10 PASS**.
- Claude markdown blockquote false-positive is fixed by commit `8b60eb0`
  (`fix(g1): skip markdown blockquote in role-trigger detector`). The detector
  now skips markdown blockquote lines before floor-phrase matching, and
  source tests preserve positive trigger behavior.
- G1 catalog stale state is cleared. Source-built `aiplus doctor` reports
  `nl_role_triggers=PASS` after `8b60eb0`; current doctor status can still be
  `NEEDS_FIX` for unrelated registry JSON repair, but the G1 catalog check is
  PASS.
- OpenCode `opencode run` remains a known non-interactive limitation per memory
  `mem_1778993719589_f268405566e57f36`: do **not** claim a non-interactive
  OpenCode PASS from `opencode run`, because that path does not load the
  instructions array the same way as the interactive TUI.
- OpenCode interactive/TUI: Owner manually verified OpenCode TUI `1.14.48`
  works with the G1 role-trigger instructions. Automated G1.1 TUI harness
  coverage exists in `crates/aiplus-cli/tests/opencode_tui_role_trigger.rs`,
  but it remains deferred because QA still sees all live cases as
  `SKIP_UNSUPPORTED_TUI` due ready-marker capture. The latest ready-marker fix
  at commit `17295ed` is not enough to convert the harness to non-skipped PASS
  evidence. Treat this as a **G1.1-DEFERRED-V1.1** automation gap, not a v1
  blocker, unless Owner reopens D4's original automated OpenCode bar.

**G1.1-DEFERRED-V1.1 note:** G1 v1 has current local evidence for Codex,
Claude, catalog freshness, and source-built doctor `nl_role_triggers=PASS`.
The remaining OpenCode item is not the role-trigger instruction path itself:
it is automated interactive/TUI evidence capture. The G1.1 harness is present,
env-gated, and intentionally separate from `opencode run`; it should be
stabilized in v1.1 by fixing ready-marker detection/transcript capture until QA
gets non-skipped PASS counts. Until then, public-facing claims must avoid
phrases that imply automated non-interactive OpenCode PASS.

**Outward-facing docs gate:** three D-C drafts remain internal and need Owner
review before any outward-facing docs change:
`D-C-DRAFT-README`, `D-C-DRAFT-AIPLUS-DEV`, and `D-C-DRAFT-ZHIWEN-SITE`.

**Opencode plan** (CEO + researcher + ai-integration to investigate before T3 estimate):
- Phase 1: Root-cause why opencode main agent ignored AGENTS.md in the N=15 study. Hypotheses:
  - opencode reads `.opencode/` but not project-root AGENTS.md
  - opencode requires an explicit `instructions:` field in its session config
  - opencode main agent skips subagent files when no subagent type is named
- Phase 2: Pick the right injection point. Candidates:
  - `.opencode/instructions.md` or equivalent
  - `aiplus install opencode` writes a session-bootstrap file opencode does read
  - System-prompt-prefix via opencode CLI flag (if any)
  - Last resort: wrapper script `aiplus opencode` that injects bootstrap then exec's opencode
- Phase 3: Implement + test. **If opencode 3/5 is not achievable after 5 engineering days**, escalate to Owner — Owner decides: (a) extend scope, (b) cut opencode from v1 and ship two-runtime version, (c) accept lower opencode bar (e.g., 1/5 with structured `aiplus opencode-fallback` doc message).

### D5: Parallel AiPlus + AEL ship

Same release ships both. Mechanism is shared (`aiplus identity context --role X` already supports both team's roles per CLI verification). AEL just registers its role names (PI / Theorist / RA-Stata / RA-Python / Referee / Replicator / PM / Advisor) in the trigger catalog. Marginal cost: data-only.

### D6: CEO must dispatch to builders — NO direct implementation

This is a **process constraint**, not a feature decision.

CEO's role in G1 execution is **coordinator only**:
- Break G1 into task cards T1–T7 (and refine if needed)
- Score each card LIGHT / MEDIUM / HEAVY
- Dispatch each card via `aiplus agent route <builder-role> "<task>"` to the right builder
- Track milestones in team memory
- Integrate result packets from builders
- Run verification gate (see below) and report PASS/FAIL to Owner

CEO must **NOT**:
- Write code directly in this session (use `aiplus agent route` to engineer-a/-b)
- Skip the dispatch step "just for small tasks"
- Bypass verification gate
- Self-approve owner_gates (publish / deploy / release / external accounts / secret exposure / global config)

Advisor (Claude Code session) remains available for spec clarification questions but does NOT write implementation code per `dev_roles.md` contract.

## Implementation outline (CEO refines)

CEO breaks into task cards. Suggested shape, **DAG with dependencies**:

```
T1 (trigger catalog spec)        ←  start here
  ↓
T2 (memory-load mechanism)       ←  blocked on T1 approval
  ↓
T3 (runtime adapter content)     ←  blocked on T1 + T2
   ├── T3a codex
   ├── T3b claude-code
   └── T3c opencode (deep work — see D4 phasing)
  ↓
T4 (confirmation contract / ack format)  ← can start parallel to T3 after T1
  ↓
T5 (test matrix)                 ← blocked on T3 + T4
  ↓
T6 (documentation)               ← blocked on T5 PASS
  ↓
VERIFY GATE                      ← Owner sign-off before merge / release
```

| Card | Description | Dispatch to | Owner-gates |
|---|---|---|---|
| T1 | Trigger catalog spec (D1): floor phrase list + positive/negative example sets, anti-pattern guardrails (quotes, code blocks, third-person), confirmation prompt for uncertain intent | architect (primary) + reviewer | none |
| T2 | Memory-load mechanism (D2): choose between (a) new flag `aiplus identity context --role X --with-memory` bundling output, or (b) AGENTS.md instruction to call `aiplus memory list` separately. Pick by token cost + reliability. Document trade-off. | engineer-a + ai-integration | none |
| T3a | codex adapter: update `aiplus install codex` bundle to include new ROLE TRIGGERS section in AGENTS.aiplus.md. Keep managed-block markers intact. | engineer-b | none |
| T3b | claude-code adapter: update `aiplus install claude-code` to inject same catalog into CLAUDE.md managed block. | engineer-b | none |
| T3c | **opencode adapter** (largest unknown): Phase 1 root-cause, Phase 2 injection-point pick, Phase 3 implement + test per D4 plan. Time-boxed: 5 engineering days. Escalate to Owner if 3/5 not achievable. | researcher (Phase 1) + ai-integration (Phase 2) + engineer-b (Phase 3) | escalation gate at 5 days |
| T4 | `ROLE_ACTIVATED` / `ROLE_BIND_REFUSED` payload schema. Versioned (`v1`). Format consumable by `aiplus doctor` + downstream tools. | architect | none |
| T5 | Test matrix: (3 runtimes × 2 projects × ≥ 5 roles each) = 30+ test cells. Reuse N=15 harness pattern. Pass bar: claude-code 5/5, codex ≥ 4/5, opencode ≥ 3/5 (per D4). Plus negative test set (10 rhetorical phrasings — none trigger). | qa | none |
| T6 | Docs: README badge / section + aiplus.dev page draft + zhiwen-wang.com tour update copy DRAFT (Owner approves before publish per below). | pm | publish gate |

## Acceptance criteria (verification gate)

CEO must run this gate and report PASS/FAIL to Owner before claiming G1 done.

1. ✅ Test matrix from T5 passes per per-runtime bars (claude 5/5, codex ≥4/5, opencode ≥3/5).
2. ✅ Negative test set: 10 rhetorical phrasings, zero false-triggers across all 3 runtimes.
3. ✅ Mid-session refuse test: in each runtime, bind to role X, then issue role-bind intent for role Y. Expected: `ROLE_BIND_REFUSED` emitted; no actual switch; Owner gets helpful instruction. 3/3 runtimes.
4. ✅ Memory load verification: for each tested role, `ROLE_ACTIVATED` payload reports non-zero `memory_team` (assuming team memory has ≥1 entry — set up fixture if needed). For coordinator roles, `memory_project` also reports a count.
5. ✅ `aiplus doctor` reports new check: `nl_role_triggers=PASS|FAIL_<reason>`.
6. ✅ Existing `aiplus identity context --role X` CLI command still works (regression check — run on every PR).
7. ✅ No regression in N=15 baseline (run prior harness against new build; claude-code stays 5/5, codex stays ≥4/5).
8. ✅ All changes have automated tests in the repo (added to `tests/` or `crates/aiplus-cli/tests/`).
9. ✅ All Owner-gated artifacts (release notes, version bump, public docs) drafted but **not published** — Owner approves before merge to main.

If any of 1–8 FAIL, CEO must **not** declare G1 done. Either fix (dispatch follow-up cards) or escalate to Owner with options + costs.

## Risks (advisor surfaces; CEO must read before T1)

**R1: Semantic intent classifier ambiguity.** D1's "agent semantically understands intent" is powerful but unprincipled. Could over-fit to Owner's specific phrasings, under-fit for new users. *Mitigation*: T1 must include a documented "trigger budget" — list the canonical floor phrases + ≥ 10 positive examples + ≥ 10 negative examples in the catalog, so the agent's prompt has a clear gradient.

**R2: Opencode unknown.** D4 commits to closing the 0/5 gap, but root cause hasn't been investigated. Worst-case opencode requires fork or wrapper script — much larger than T3c's 5-day budget. *Mitigation*: T3c Phase 1 is gated; Owner reviews findings before Phase 2 commits to engineering scope.

**R3: Memory-load token cost.** D2's "load + scope" model can pull substantial bytes into context. For a CEO session with 50+ team-memory entries + 30 project entries, the ROLE_ACTIVATED payload could be 5K+ tokens. *Mitigation*: T2 must specify a per-scope cap (e.g., 20 most recent entries per scope, with full-list available via separate command).

**R4: Refuse-vs-switch regret.** D3 refuses mid-session switch. If Owner uses this heavily, the friction may push them back to "open many sessions" — losing the cross-role coordination value. *Mitigation*: collect data after 1 week of dogfood. Owner can revisit D3 in G1.1.

**R5: Cross-runtime regression.** Existing claude-code 5/5 and codex 4/5 could be broken by catalog changes. *Mitigation*: T5 explicitly tests for no regression. Block merge on test failure.

**R6: Quota cost during G1 execution.** Owner's codex weekly quota is currently 98% used. CEO dispatching to multiple builders will draw down further. *Mitigation*: CEO defers T3c (the heavy one) until Owner quota resets; T1/T2/T4 are lighter and can proceed sooner.

## Owner-gates active during G1 execution

Per advisor identity context (`owner_gates=[publish,deploy,global config,external accounts,secret exposure]`), CEO must escalate before:

- 🔒 Bumping version tag / cutting release
- 🔒 Publishing release notes
- 🔒 Updating zhiwen-wang.com or aiplus.dev public copy
- 🔒 Touching `~/.aiplus/` global config (none planned, but verify)
- 🔒 Adding external service dependencies
- 🔒 Skipping the verification gate to ship faster

Not gated:
- ✅ Drafting catalog text, builder dispatch, test runs, internal docs, code changes within aiplus-public / aiplus-agent-team / aieconlab repos

## Out of scope (v1)

- Cross-session sync between Advisor and CEO sessions → separate goal G2 (drafted later)
- NL trigger for non-role commands (`compact` / `refresh` / `status`) → separate goal G3
- Voice/audio triggers → different runtime entirely
- Auto-detect-runtime-from-cwd-and-suggest-correct-trigger → nice-to-have, defer
- v1.1 features (relaxing D3 refuse to allow switch) → depends on dogfood data, not in v1

## How CEO picks this up — dispatch command

Owner pastes this into the CEO codex session:

```
aiplus agent route ceo "Implement G1 per spec at docs/proposals/G1-role-nl-triggers.md. Owner has signed off on D1–D6.

Constraints (read first):
- D6: you MUST dispatch all implementation to builders via 'aiplus agent route <builder> \"<task>\"'. Do not write code yourself.
- Owner gates active per advisor contract; escalate before publish/release/global-config.
- Quota constraint: defer T3c (opencode engineering) until Owner's codex quota resets. T1/T2/T4 can proceed sooner.

Step 1: Read G1 fully and write a team-memory note acknowledging spec receipt + your reading of D1–D6 + R1–R6.

Step 2: Refine the 7 task cards into the team's actual task-card format. Score each LIGHT/MEDIUM/HEAVY. Confirm DAG dependencies with architect.

Step 3: Dispatch T1 to architect (with reviewer co-signer). Wait for result packet.

Step 4: Iterate T2 → T3 → T4 → T5 per DAG.

Step 5: Run verification gate (acceptance criteria 1–9). Report PASS/FAIL to Owner.

If you hit a blocker, surface to Owner via team memory + a status report. Do not bypass the gate."
```

## Advisor's continuing role during G1

I (Advisor) stay available for:
- Spec clarification when CEO or a builder reads G1 and has questions about intent
- Re-review when CEO proposes scope changes (D1–D6 modifications, T3c escalation)
- Reading verification-gate results and giving second opinion before Owner approves release
- Helping Owner judge the "extend / cut / lower bar" decision if T3c escalates

I do **not** write code, dispatch builders directly, or merge anything. Per `dev_roles.md`.
