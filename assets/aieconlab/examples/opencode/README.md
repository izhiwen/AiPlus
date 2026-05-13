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
4. **PI → Theorist**: write the identification note before any RA touches data.
5. **PI → PM**: scope the 6-week kickoff plan with milestones.
6. **PI → Lit Reviewer (expert summon)**: build the initial lit map.
7. **PI → Historical Sources (expert summon)**: catalog the archive's coverage and biases.
8. **PI → Reproducibility Engineer (expert summon)**: build the project scaffold (`Makefile`, `environment.yml`, `ado/`, CI).
9. **PI → RA-Python (activate)**: data ingestion pipeline once Theorist's note and Historical Sources brief are ready.
10. **PI → Owner**: kickoff status report with 6-week Gantt and next decisions.

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
- Three experts get summoned at kickoff; the others stay dormant.
- RA-Python is activated (was dormant by default) because this is a
  data-heavy archival project.
- The Theorist's identification note precedes any RA work — a paper
  cannot start by running regressions.

## v0.1 status

The opencode runtime adapter is a placeholder in v0.1. This README
documents the *intended* routing; the actual integration with OpenCode
project-local agents lands in v0.2.
