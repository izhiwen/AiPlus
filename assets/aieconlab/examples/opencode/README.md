# OpenCode example — AiEconLab

A synthetic walkthrough of a project-kickoff sequence routed through the
AiEconLab under the OpenCode runtime.

## Scenario

Owner says: "I'm starting a new paper on missionary exposure and long-run
human capital. Set the project up."

## Expected routing

1. **Owner → Advisor**: framing pass on whether this is a viable QJE-shaped paper.
2. **Advisor → Owner**: recommendation, including framing risks and recommended next steps.
3. **Owner → PI**: launch the project per Advisor's recommendation.
4. **PI scores the kickoff as HEAVY**: new paper + new dataset + first paper on this archive → complexity ≥ 5. PI fires the **AEL consultant team** before any role is dispatched.
5. **Consultant (HEAVY tier)** delivers 5 plan-time `output_artifact`s:
   - `design-credibility-check.md` (Design Credibility seat) — yes/no on setting fit, estimator choice, falsifiable assumption
   - `contribution-frame.md` (Contribution Framing seat) — 3-5 closest comparables + one-sentence differential claim + target tier
   - `day-1-scaffold-checklist.md` (Day-1 Reproducibility seat) — Makefile / env / seed / archive provenance requirements
   - `irb-gate-check.md` (IRB / Disclosure Gate seat) — protocol-scope yes/no + small-cell risk
   - `llm-validity-protocol.md` (LLM-as-Measurement seat) — fires only if LLMs enter as measurement
   Plus the 3 user personas (Top-Tier Referee / Job-Market Audience / External Replicator) weigh in because risk ≥ 0.7.
6. **PI → Theorist**: write the identification note using the Design Credibility output as input.
7. **PI → PM**: scope the 6-week kickoff plan with milestones, including consultant flags as plan checkpoints.
8. **PI → Lit Reviewer (expert summon)**: build the initial lit map using the Contribution Framing comparables as seed.
9. **PI → Historical Sources (expert summon)**: catalog the archive's coverage and biases.
10. **PI → Reproducibility Engineer (expert summon)**: build the project scaffold per `day-1-scaffold-checklist.md`.
11. **PI → LLM-as-Measurement Specialist (expert summon)**: if the paper uses LLMs to score archival text, design the validity protocol per `llm-validity-protocol.md` before any scoring runs.
12. **PI → RA-Python (activate)**: data ingestion pipeline once Theorist's note, Historical Sources brief, and (if applicable) LLM validity protocol are ready.
13. **PI → Owner**: kickoff status report with 6-week Gantt, consultant flags, and next decisions.

## Files this example would touch

- `paper/identification_note_v1.md` (Theorist, on `agent/theorist`)
- `paper/lit_map.md` (Lit Reviewer, on `agent/lit-reviewer`)
- `data/source_brief.md` (Historical Sources, on `agent/historical-sources`)
- `Makefile`, `environment.yml`, `ado/` (Reproducibility Engineer)
- `pipelines/prefecture_decade/` (RA-Python, on `agent/ra-python`)
- `.aiplus/agent-memory/_team/submission_queue.md` (PI: empty for new paper)
- `.aiplus/agent-memory/_team/active_experts.md` (PI: lit-reviewer, historical-sources, reproducibility now active)

## What this example demonstrates

- Advisor gates the project before PI touches it.
- **HEAVY tier fires the consultant team** with 5 plan-time output_artifacts —
  the consultant flags become inputs to the execution roles, not parallel work.
- Three to four experts get summoned at kickoff; the others stay dormant.
- RA-Python is activated (was dormant by default) because this is a
  data-heavy archival project.
- The Theorist's identification note precedes any RA work — a paper
  cannot start by running regressions.
- If the project uses LLMs as a measurement instrument (text-as-data), the
  LLM-as-Measurement Specialist designs the validity protocol BEFORE the
  first scoring run — retrofit validity is the most common reason
  LLM-measurement papers get desk-rejected.

## Status

The opencode runtime adapter is shipped (v0.3+). This README documents
the intended routing; see
[`adapters/opencode/README.md`](../../adapters/opencode/README.md) for
current adapter details.
