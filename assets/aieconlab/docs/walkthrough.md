# AiEconLab Walkthrough — One Full Research Loop

A 5–10 minute read showing what AiEconLab actually does on a real applied-economics task. For the in-place run-trace with friction notes, see [`beta-walkthrough.md`](beta-walkthrough.md). For the 30-second demo, see [`demo-script.md`](demo-script.md) and [`demo-lines.txt`](demo-lines.txt).

This doc is the **conceptual walkthrough** — what each role does, what gets handed off, and what an actual paper-kickoff loop looks like. Use it if you're trying to decide whether AEL is for you before you `aiplus add aieconlab`.

---

## The setting

You are starting a new applied-economics paper. You have:
- A research question (rough)
- An empirical setting (some dataset in mind)
- No code yet
- No literature review yet
- A target submission venue in mind (or you'll figure it out)

In the single-agent world, you'd dump all of this into one chat. The agent would partially scope it, partially start coding, partially Google references, and produce a mediocre stew. By task three you'd be re-explaining context that was buried four scrolls up.

AiEconLab gives you a permanent team. You talk to **Advisor** (strategic feedback) and **PI** (orchestrator). Everyone else is dispatched by the PI.

---

## Beat 1 — Owner → Advisor: "is this a paper?"

```
$ aiplus agent talk advisor
```

You describe the question. Advisor asks two or three sharp questions to surface:
- Whether the question is already settled in the literature
- Whether the empirical setting actually identifies what you think it does
- Whether the venue/scope matches the available data

Advisor's persona is intentionally conservative — its job is to kill bad papers before they soak up six months of your life. Output: a one-page "go / don't go / sharpen this first" note.

**What didn't happen:** Advisor didn't start coding. Didn't write a lit review. Didn't suggest 12 robustness checks. It stayed in scope.

---

## Beat 2 — Owner → PI: "we're going. kick off the team."

```
$ aiplus agent route pi "kickoff the Treaty Ports paper"
```

The PI:
1. Reads Advisor's go-note from the project's `.aiplus/agent-memory/advisor/` namespace.
2. Scores the task (LIGHT / MEDIUM / HEAVY). A full paper kickoff is HEAVY.
3. Fires **Auto-Team-Consultant** before doing anything else — the consult-before-plan layer surfaces pitfalls at plan time: "did you check whether this identifying variation has been used before? what's the data licensing situation? have you talked to your IRB?" Output of consult flows into PI's brief.
4. Routes sub-tasks to the right roles:
   - **Theorist** — formalize the conceptual model
   - **PM** — set up the project skeleton (`paper/`, `data/`, `code/`, `results/`, `docs/`)
   - **RA-Stata** + **RA-Python** — each gets their own git worktree so they can scaffold data-loading code in parallel without stepping on each other
   - **Lit Reviewer** (expert) — invited to assemble a literature inventory

Each of these runs in its own agent process, in its own worktree, with its own memory namespace. They don't see each other's reasoning by default — the PI does.

---

## Beat 3 — Theorist's pass

```
$ aiplus agent talk theorist
```

Theorist returns a 3-page conceptual note:
- Stylized model of the mechanism you want to estimate
- Where identification comes from (the *theoretical* claim about exogeneity)
- What moments you'd need to recover the parameter of interest
- Predictions you can falsify against the data

**This note becomes the binding spec for the empirical roles.** RA-Stata and RA-Python don't get to make up what they're estimating — they implement what Theorist's note specifies.

---

## Beat 4 — RAs scaffold in parallel

Both RAs work in separate git worktrees (`/tmp/my-paper.ra-stata` and `/tmp/my-paper.ra-python`). They:
- Load the data
- Replicate the descriptives Theorist's note assumes
- Build the table-1 summary statistics
- Each writes a preliminary `do/` or `.py` script implementing the baseline spec

When both finish, PI runs `aiplus agent integrate ra-stata` and `aiplus agent integrate ra-python` to merge their branches back into `main`. Conflicts (e.g., they disagree on variable naming) surface as git merge conflicts — **not** as silent overwrites in a single shared chat.

---

## Beat 5 — Referee's adversarial pass

```
$ aiplus agent route referee "first-cut critique"
```

Referee reads:
- Theorist's note
- RA outputs (table 1, baseline regression)
- The consult's pre-registration check

…and writes a journal-style referee report against your own paper. This is the highest-value role in AEL: it surfaces what reviewers will ask before you waste two months down the wrong road.

The reports are sharp by design. Most users find the first one demoralizing. That's the point.

---

## Beat 6 — LLM-as-Measurement Specialist (when the data is text)

If your empirical setting involves coding unstructured text (legal opinions, historical memorials, regulatory filings, customer-service transcripts, ...), the PI invites the LLM-as-Measurement Specialist:

```
$ aiplus agent invite llm-measurement
```

This expert codifies the validation methodology I built for my own JMP, which is now public at [Multi-LLM-Validation-Demo](https://github.com/izhiwen/Multi-LLM-Validation-Demo) — 294 archival documents scored across 5 frontier LLMs with mean pairwise correlation 0.92.

The specialist's protocol:
1. Hand-code a gold-standard subset (you do this; the agent doesn't).
2. Generate prompts for the full corpus, optionally with chain-of-thought.
3. Score the corpus independently across **at least three** frontier model families (e.g., Claude + GPT + open-weights).
4. Compute pairwise score correlations; surface model-disagreement cases for human re-review.
5. Output: a validated dataset + a one-page methodological appendix you can drop into the paper.

If the cross-model correlations are weak, the specialist tells you the measurement isn't ready for the right-hand-side of a regression — and what to do about it (revise prompts, restrict the corpus, hand-code more).

---

## Beat 7 — Replicator's check

```
$ aiplus agent route replicator "verify table 1 reproduces"
```

Replicator clones the repo into a fresh worktree, runs every script from scratch on a synthetic clean machine, and checks that all numbers in your tables / figures come out the same. If they don't, it tells you which step broke. This is the reproducibility-package work that you'd otherwise do at the eleventh hour for AEA P&P submission.

---

## What you (the Owner) actually did

You:
- Talked to Advisor (10 min)
- Routed a kickoff to PI (1 command)
- Spot-checked Theorist's note (15 min)
- Spot-checked the merged baseline (30 min)
- Hand-coded the gold-standard text subset (a few hours, real work)
- Read Referee's adversarial report and decided which critiques to take seriously (30 min)

The team did the rest. Total Owner wall time: ~4-5 hours of focused engagement, spread over a few days. Single-agent equivalent: 2-3 days of half-attention back-and-forth with one increasingly drift-y chat.

---

## Where this breaks today (honest)

- **First-time setup friction.** You have to install AiPlus first, then AEL, then pick a runtime (Codex / Claude Code / OpenCode). The README's two install paths help; still ~15 min of fiddling.
- **PI's task scoring is imperfect.** LIGHT/MEDIUM/HEAVY heuristics sometimes route a quick fix to the full council. Override with explicit role hints when this happens.
- **No native LaTeX agent yet.** Drafting the actual paper text is still mostly the Owner's job. Writer expert is on the v0.2 roadmap.
- **R / Julia support is declared-only.** The shipped RA roles are Stata + Python. If your stack is R, you pay a small per-task hint cost.

For the most honest log of what works today vs. what doesn't, read [`beta-walkthrough.md`](beta-walkthrough.md) — it's an in-place trace against a synthetic treaty-ports kickoff, with friction notes inline.

---

## When AiEconLab is **not** the right tool

- **You're shipping software, not research.** Use [AiPlus-Agent-Team](https://github.com/izhiwen/AiPlus-Agent-Team) — same architecture, SWE roles.
- **You don't have a separable team-of-roles workflow.** If your work is "one researcher, one task, one chat" indefinitely, single-agent + Agent-Memory is enough.
- **You don't yet have a research question.** Advisor will return "go think more" and you'll waste a cycle. Come back when the question is closer to formed.

---

## Where to read more

- `DESIGN.md` — full architectural rationale, routing protocol, memory model, worktree policy, acceptance criteria
- `core/templates/personas/` — the actual persona prompts each role uses
- `examples/` — synthetic worked examples for all three runtimes
- [`demo-script.md`](demo-script.md) — 30-second recording script for the README hero GIF
- [`beta-walkthrough.md`](beta-walkthrough.md) — honest in-place run trace
