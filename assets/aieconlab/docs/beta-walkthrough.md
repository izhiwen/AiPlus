# Beta Walkthrough — Synthetic Treaty Ports Kickoff

A live trace of running AEL through a synthetic applied-econ paper
kickoff. Captured 2026-05-13 against aiplus v0.5.9 + AEL HEAD on a
clean `/tmp/treaty-ports-test/` directory. This isn't a battle-test on
the maintainer's real research data — that's the next exercise (G1 in
the punch list). This is the *closest* AEL has been pushed without
real research data, and it surfaces the design-vs-reality friction
points an end user will hit.

If you're considering using AEL on a real paper, read this first. It's
the most honest signal available about what works today vs what
doesn't.

## Setup

```bash
mkdir /tmp/treaty-ports-test && cd /tmp/treaty-ports-test
git init -q -b main
aiplus install codex
aiplus add aieconlab
git add -A && git commit -qm "init"
```

All four commands succeed. Total: ~3 seconds. `aiplus doctor` reports
`DOCTOR_STATUS=PASS`.

## Test 1 — Status

```
$ aiplus agent status
AiPlus Agent Team v0.1
Project root: /private/tmp/treaty-ports-test
Active team: aieconlab  (switch with `aiplus agent set-team agent-team`)

Team Roster:
  Active roles: [] (32 core roles configured; no dispatches recorded
    yet — run `aiplus agent route <role>` to mark one active)
  Total agents: 37
```

✅ **Works**: clear "Active team: aieconlab" line, helpful next-step
hint, sane summary.

⚠️ **Friction**: 32 + 5 stubs = 37 "agents". Most of those 32 are
roles from BOTH `agent-team` (SWE: ceo, architect, engineer-a/b,
reviewer, qa) AND `aieconlab` (PI, theorist, ra-stata/python, referee,
replicator) co-installed because `aiplus install codex` auto-installs
`agent-team`. From the user's perspective the count is confusing —
"I'm an econ user, why do I have CEO and engineer-a?"

**Recommendation**: when `Active team: aieconlab`, the roster should
default to showing only AEL roles (advisor + pi + theorist + pm +
ra-stata + ra-python + referee + replicator + 12 experts). The SWE
roles should be collapsed behind a `--show-all` flag. Or, more
ambitiously: don't auto-install `agent-team` for users who explicitly
`aiplus add aieconlab` if there's no prior dispatch history on SWE
roles.

## Test 2 — Dispatch to PI (kickoff)

```
$ aiplus agent route pi "kickoff Treaty Ports paper"
Routing task to pi: kickoff Treaty Ports paper
  Dispatch recorded: .aiplus/agents/dispatch-log.jsonl
```

✅ **Works**: dispatch recorded with timestamp.

⚠️ **Friction**: no worktree created for PI. Looking at the persona,
PI is an owner-facing role with `needs_worktree = false`, so this is
*intentional*. But the user has no signal that this was deliberate vs
a bug — there's no "PI doesn't get a worktree (owner-facing role)"
message.

⚠️ **Bigger friction**: no consultant team fired. The DESIGN.md says
HEAVY tier (complexity ≥ 5, or risk ≥ 0.7) should fire the consultant.
"Kickoff a new paper" with brand-new dataset is textbook HEAVY: novel
identification (+1 complexity), new dataset (+1 complexity),
first_paper_on_dataset (+1 risk). But the consultant didn't fire — no
output_artifacts were emitted, no Owner gate packet was surfaced.

**Recommendation**: `aiplus agent route pi <task>` should:
1. Score the task per the consultant scaling rules
2. If MEDIUM or HEAVY, mention "consultant fired (3 seats)" or
   "consultant fired (all 5 seats + personas)" in the output
3. Generate the output_artifact files in `.aiplus/agents/_consultant/`
   so the user can read them before the PI actually starts

The current behavior is "dispatch to PI, log it, done" — the
consultant integration is in the persona prose but not in the CLI
machinery yet. This is a design-vs-reality gap.

## Test 3 — Dispatch to Theorist (identification)

```
$ aiplus agent route theorist "write identification note for treaty-port IV strategy"
Routing task to theorist: write identification note for treaty-port IV strategy
  Creating worktree for theorist...
Created worktree for theorist at /private/tmp/treaty-ports-test.theorist
  (Branch: agent/theorist)
  Dispatch recorded: .aiplus/agents/dispatch-log.jsonl

  ℹ  Task tier: MEDIUM (task description contains a specification /
     robustness / identification / review keyword).
```

✅ **Works**: worktree created on `agent/theorist` branch. **And**
the auto-tier detection fired — "identification" keyword detected,
task tagged MEDIUM. Visible to the user in the route output.

⚠️ **Friction**: MEDIUM tier fires consultant per AEL DESIGN.md, but
the actual consultant didn't fire (same gap as Test 2). The tier
detection is there, the consultant config is there, but the wiring
between them isn't.

**Recommendation**: when tier is MEDIUM or HEAVY, the route command
should:
1. Print which consultant seats fired and why (the matched triggers)
2. Write the output_artifact stubs into
   `.aiplus/agents/_consultant/<role>/<output_artifact>.md` so the
   PI agent can read them before producing their plan
3. Surface any Owner gates the plan path touches

## Test 4 — Dispatch to LLM-Measurement Specialist

```
$ aiplus agent route llm-measurement "design validity protocol for
  scoring 19th-century Chinese documents across 5 LLMs"
Routing task to llm-measurement: ...
  Creating worktree for llm-measurement...
Created worktree for llm-measurement at /private/tmp/treaty-ports-test.llm-measurement
  (Branch: agent/llm-measurement)
  Dispatch recorded: .aiplus/agents/dispatch-log.jsonl
```

✅ **Works**: worktree created on `agent/llm-measurement` branch.
This is the **moment of truth** for the new 12th expert. It works
end-to-end: persona installed, worktree provisioned, dispatch logged.

⚠️ **Friction**: no tier inference message this time. The task
description contains "LLM" and "validity" — both keywords that
*should* fire the consultant's `ai_integration` seat. But the tier
detection regex doesn't catch them.

**Recommendation**: extend the tier-detection keyword set to include
`LLM`, `GPT`, `Claude`, `validity`, `measurement`, `score`,
`hand-coded`, `held-out`, `multi-LLM`. Or even better: read the
consultant config's `triggers` arrays and use them for tier detection
(eliminates duplication between consultant config and CLI tier logic).

## Test 5 — Dispatch log integrity

```
$ cat .aiplus/agents/dispatch-log.jsonl
{"schemaVersion":"0.1.0","timestamp":"2026-05-13T23:33:41.486Z",
 "role":"pi","task":"kickoff Treaty Ports paper",
 "reversibility":"unspecified","source":"aiplus agent route"}
{...theorist...}
{...llm-measurement...}
```

✅ **Works**: 3 valid JSON lines, one per dispatch. Each has timestamp,
role, task, reversibility, source.

⚠️ **Friction**: `reversibility: "unspecified"` for every dispatch.
The AEL design distinguishes reversible / semi-reversible /
irreversible decisions. Currently neither the CLI nor the user is
asked to tag this. So the reversibility class never actually informs
downstream behavior.

**Recommendation**: either drop the field (don't ship something we
don't use), or add a `--reversibility` flag to `aiplus agent route` so
the dispatcher can mark a task as irreversible (submission, R&R) and
the CLI can require Owner confirmation.

## Test 6 — Doctor

```
$ aiplus agent doctor
Running agent team doctor...
Checking .aiplus/agents/ directory...
  Found 37 agent config(s)
  ...
  pi (PI)
  devops (DevOps / SRE)
    WARNING: worktree ../treaty-ports-test.devops does not exist
  ...
Doctor check complete.
```

✅ **Works**: doctor runs, validates all 37 agents.

⚠️ **Friction**: doctor warns about ~30 missing worktrees for every
non-dispatched role. This is the lazy-creation design (worktrees only
exist after a dispatch). But the WARNING level for an intentionally-
empty state is wrong — it makes the doctor output noisy.

**Recommendation**: change "missing worktree" from WARNING to INFO,
or skip it entirely for roles with no dispatch history.

## Test 7 — Transcript

```
$ aiplus agent transcript
Transcript feature not yet implemented in v0.1
```

❌ **Doesn't work**: feature is referenced in DESIGN.md §11 CLI
surface but not implemented.

**Recommendation**: either implement (it's just `cat
dispatch-log.jsonl | jq` essentially) or remove the command from the
help text until ready.

## Summary of findings

| Area | State | Friction |
|---|---|---|
| Install / opt-in flow | ✅ Works smoothly | None |
| Active team switching | ✅ Works | UX could highlight current team in agent list |
| Dispatch via `route` | ✅ Works | Doesn't fire consultant; doesn't surface owner gates |
| Worktree creation | ✅ Works | PI doesn't get one — intentional but not signposted |
| Persona file install | ✅ Works | — |
| Consultant config install | ✅ Works | AEL config replaces SWE default correctly |
| Tier auto-detection | ⚠️ Partial | Some keywords match (`identification`), others don't (`LLM`, `validity`) |
| Consultant team firing | ❌ Designed but not wired | Tier is computed but consultant integration is prose-only |
| Owner gate surfacing | ❌ Not wired | Gates declared in TOML, not checked at dispatch |
| `agent transcript` | ❌ Not implemented | Command exists in help but stub-only |
| Doctor noise | ⚠️ Acceptable | 30 missing-worktree warnings for lazy-init roles |

## The honest assessment

What's solid:

- The **structural pipeline** (install → add → dispatch → worktree)
  works end-to-end. Worktrees are real git branches. Personas are
  real Markdown files. Dispatch logs are real JSONL. None of this is
  smoke and mirrors.
- The **persona content** is genuinely useful. Reading `pi.md` and
  `llm-measurement.md` after a few weeks of distance, they hold up
  as real research-workflow guidance.
- The **consultant config** is well-designed. The 5 seats with their
  output_artifact contracts feel right for plan-time review.

What's not yet there:

- The **link** between the consultant config and the CLI's `route`
  command. The config says "fire consultant on MEDIUM/HEAVY"; the CLI
  doesn't read the config when routing. This is the v0.2 work that
  matters most.
- The **owner-gate enforcement**. Owner gates are declared in the
  consultant TOML but the CLI doesn't intercept on dispatch.
- The **persona embodiment**. `aiplus agent route` creates the
  artifact (worktree, dispatch log) but doesn't actually load the
  persona into a running agent. To use the PI persona, you still
  manually copy-paste it into Claude / Codex, or run
  `aiplus agent talk pi` which spawns a host runtime with the persona
  pre-loaded (per the v0.5.5 Phase D work, partially complete).

What I'd prioritize for v0.1.1 / v0.2:

1. **Wire the consultant into route**: parse the consultant config,
   detect matched triggers, fire the consultant seats, emit
   output_artifacts into `.aiplus/agents/_consultant/`. This is the
   highest-leverage change because it converts AEL from "documented
   methodology" to "executable methodology".

2. **Owner-gate enforcement at dispatch time**: if a dispatched task
   touches a STOP-gated path (e.g., contains "submit" + "journal"),
   require an explicit Owner confirmation via a prompt or `--yes`
   flag. Currently the gates are in prose only.

3. **Tier detection by reading consultant triggers**: instead of
   hardcoded keyword set in the CLI, parse the consultant TOML's
   `triggers` arrays. Single source of truth.

4. **Reduce doctor noise**: WARNING → INFO for lazy-init missing
   worktrees.

5. **Implement `agent transcript`** or remove from help.

## What this walkthrough does NOT test

- Long sessions where compact fires mid-task
- The persona's behavior when actually embodied by a host runtime
  (Codex / Claude Code / OpenCode) on a real research question
- Inter-agent coordination (Theorist → RA-Stata → Replicator chain)
- Recovery from a corrupt worktree or partial dispatch
- The acceptance flow under real R&R deadline pressure
- The IRB Gate seat behavior on real restricted data
- The LLM-as-Measurement Specialist on a real LLM scoring task

These are the **next** battle-tests, which require real research data
and longer runtime than a quick walkthrough.

## Reproducibility

```bash
# Run this walkthrough yourself:
mkdir /tmp/aiel-test && cd /tmp/aiel-test
git init -q -b main
aiplus install codex
aiplus add aieconlab
git add -A && git commit -qm "init"

aiplus agent status
aiplus agent route pi "kickoff Treaty Ports paper"
aiplus agent route theorist "write identification note for treaty-port IV strategy"
aiplus agent route llm-measurement "design validity protocol for scoring 19th-century Chinese documents across 5 LLMs"
cat .aiplus/agents/dispatch-log.jsonl
aiplus agent doctor 2>&1 | tail -30
```

Compare your output to the captured findings above. If anything
diverges, please file a GitHub issue with the difference — that's
the most useful contribution you can make right now.
