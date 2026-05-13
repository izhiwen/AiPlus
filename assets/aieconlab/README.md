# AiEconLab (AEL)

> **A permanent virtual research team for applied economists.**
> Eight core agents (Advisor, PI, Theorist, PM, RA-Stata, RA-Python, Referee, Replicator)
> plus an twelve-specialist expert directory. Default toolchain: Python + Stata + LaTeX.

[![License: Apache 2.0](https://img.shields.io/badge/License-Apache_2.0-blue.svg)](LICENSE)

[中文 README](README.zh-CN.md)

## Prerequisites

AiEconLab is built on top of the [AiPlus](https://github.com/izhiwen/AiPlus)
agent substrate. Install AiPlus first:

```bash
# Install AiPlus (>= 0.5.2)
# Follow https://github.com/izhiwen/AiPlus

# Then add the three substrate modules AiEconLab depends on:
aiplus add agent-memory          # per-agent project-local memory
aiplus add compact-reminder      # token-saving compact + structured resume
aiplus add auto-team-consultant  # consult-before-plan layer
# (velocity calibration is built into the aiplus CLI; no separate add needed)
```

AiEconLab is intentionally a separate project (`github.com/izhiwen/AiEconLab`)
with its own release cycle and audience, even though it uses the AiPlus
substrate for memory, compact, velocity, and consult-before-plan.

## The pain

You ask the agent to scope a paper, then clean the data, then run identification,
then write the introduction, then prepare a referee response. By the third task
it has **drifted**: the same prompt history now contains research design
intuition, Stata syntax, lit-review fragments, and rebuttal language, and the
output is none of them well.

Worse, the shared context **pollutes** across roles. A theorist's framing leaks
into the empirical RA's regression spec. Lit-review notes get buried under
debug printlns from data cleaning. Robustness checks age out of the window
because of irrelevant scratch.

You try to compensate by giving the agent more hats. But one agent wearing
PI, Theorist, Econometrician, RA, Referee, and Replicator does each hat
**shallowly**. Real research projects divide labor because the work *is* that
structured.

### Not these other pains

AiEconLab is specifically about **role separation and execution**
for applied-economics research. Other AiPlus plugins solve adjacent but
different problems:

| Plugin | Pain it solves | Why it is not AiEconLab |
|---|---|---|
| [AiPlus-Agent-Team](https://github.com/izhiwen/AiPlus-Agent-Team) | Software-engineering role drift | Same architecture, different roles — that one ships SWE roles, this one ships research roles |
| [AiPlus-Agent-Memory](https://github.com/izhiwen/AiPlus-Agent-Memory) | **amnesia** — agent forgets context between sessions | Gives one agent a memory; does not split roles |
| [AiPlus-Auto-Team-Consultant](https://github.com/izhiwen/AiPlus-Auto-Team-Consultant) | **overlooks** — agent misses pitfalls at plan time | Advises *before* planning; does not execute or persist roles |
| [AiPlus-Compact-Reminder](https://github.com/izhiwen/AiPlus-Compact-Reminder) | **token waste** — long sessions burn tokens reloading the same context | Compact + structured resume saves tokens for one agent; does not separate roles |
| [AiPlus-Agent-Velocity](https://github.com/izhiwen/AiPlus-Agent-Velocity) | **mis-bills** — estimates anchor on human hours | Calibrates one agent's estimates; does not structure a team |

AiEconLab and [AiPlus-Agent-Team](https://github.com/izhiwen/AiPlus-Agent-Team)
are siblings — they can coexist in the same project (e.g. a researcher who
also maintains a replication package as a software repo).

## What we do about it

**Replace single-agent drift with a permanent research team.**

AiEconLab installs a permanent virtual team of eight core roles
into your project: Advisor, PI, Theorist, PM, RA-Stata, RA-Python, Referee,
and Replicator. Each role has its own persona, workspace, and memory
namespace. The Owner (you, the lead author) talks only to Advisor and PI; the
PI orchestrates the rest.

The records cover:

- **Role isolation** — each agent loads only its own persona and personal
  memory. An RA does not see the Theorist's reasoning, and vice versa.
- **Git worktree workspaces** — code-touching roles get isolated working
  directories so RA-Stata and RA-Python can work in parallel without
  stepping on each other. Conflicts surface through git, not silent overwrites.
- **Three-layer memory** — personal (per-agent), team (PI-shared), and
  project (existing `.aiplus/memory/`). Project memory wins on conflict, so
  a team-of-the-day decision never overrides durable project consensus.
- **Expert directory** — twelve specialists (Lit Reviewer, Writer,
  Econometrician, Reproducibility Engineer, Historical Sources, Job Talk
  Coach, and others) sit dormant until the PI summons them for tasks that
  match their triggers.
- **Adaptive routing** — the PI scores each task (LIGHT, MEDIUM, HEAVY) and
  staffs only the roles that are needed. A quick coding fix gets a single
  RA; a draft submission gets the full council.

Default toolchain is **Python + Stata + LaTeX**. R and Julia are supported
when the project declares them.

No daemon. No cloud sync. No upload. Each agent is state-level permanent —
its files live on disk, but the process is ephemeral, spawned only when the
PI routes a task.

## Install

Add the module to your project:

```bash
cd MyResearchProject
aiplus add aieconlab
aiplus install codex          # or: claude-code, opencode, all
```

`aiplus add aieconlab` does three things:

1. Installs all 8 core role configs + personas (Advisor, PI, Theorist, PM,
   RA-Stata, RA-Python, Referee, Replicator).
2. Installs all 12 expert configs (9 shipped, 3 v0.2 stubs).
3. **Replaces** the default SWE consultant team
   (`.aiplus/consultant-team.toml` from `AiPlus-Auto-Team-Consultant`)
   with `consultant-team.aieconlab.toml` — 5 expert seats designed from
   first principles for applied-econ research at plan time, 3 user
   personas, 5 owner gates mirroring AEL DESIGN §16 STOP-gates, LIGHT
   tier skips consult by design.

If you also have `aiplus-agent-team` (SWE) installed in the same project,
the AEL consultant config overwrites the SWE one — coexistence of both
consultant configs is on the v0.2 roadmap.

## Quick start

```bash
aiplus agent status              # Show team roster, active experts, warm bench
aiplus agent route ra-stata      # Assign task to RA-Stata
aiplus agent integrate ra-stata  # Merge RA-Stata's branch back into main
aiplus agent audit run           # Run acceptance audit
```

Route a task through the PI:

```text
aiplus agent route "estimate the main IV spec with cluster-robust SEs"
```

The PI scores the task, picks the right team members, and reports back.

Other everyday commands:

```bash
aiplus agent doctor            # validate configs, worktrees, memory layout
aiplus agent list              # list all roles (core + expert)
aiplus agent talk theorist     # direct conversation with one role
aiplus agent invite lit-reviewer       # add an expert to the active team
aiplus agent dismiss lit-reviewer      # remove expert from active team
aiplus agent transcript        # show recent activity for audit
aiplus agent prune-worktrees   # clean up stale worktrees
```

## Architecture overview

```
                  aieconlab             ← orchestration layer
                           ↓ uses
               AiPlus-Auto-Team-Consultant           ← decision-support layer
                           ↓ uses
    AiPlus-Agent-Memory  AiPlus-Compact-Reminder  AiPlus-Agent-Velocity
               ←——————— shared infrastructure layer ———————→
```

AiEconLab is the orchestration layer. It sits on top of the four
existing AiPlus plugins and uses them as shared infrastructure:

- **AiPlus-Agent-Memory** — each agent gets a namespaced memory under
  `.aiplus/agent-memory/<role>/`
- **AiPlus-Compact-Reminder** — each long-running agent runs its own token-saving compact
  cycle; PI tracks compact state per agent
- **AiPlus-Agent-Velocity** — each agent has its own velocity records, with
  research-specific units (regression-spec, table, figure, paper-section)
- **AiPlus-Auto-Team-Consultant** — PI fires consultant before MEDIUM and
  HEAVY tasks; consultant findings flow into the staffed team's brief

### Five core design decisions

1. **Permanent core team of 8 roles** — installed automatically when the
   plugin is added to a project.
2. **Expert directory** — 12 specialist roles available on-demand, only
   summoned when triggers match.
3. **State-level permanence + warm bench** — agent identity lives on disk;
   process is ephemeral, spawned only when PI routes a task.
4. **Git worktree workspaces** — each code-touching role gets an isolated
   working directory so RA-Stata and RA-Python can work in parallel without
   silent overwrites.
5. **Three-layer memory** — personal (per-agent), team (PI-shared), and
   project (existing `.aiplus/memory/`). Project memory wins on conflict.

See [`DESIGN.md`](DESIGN.md) for the full design rationale, routing protocol,
memory model, worktree policy, and acceptance criteria.

## What's inside

- `core/templates/` — TOML configs for all 8 core roles plus the
  team-wide `econ-team.toml` and the AEL research-tuned
  `consultant-team.aieconlab.toml`
- `core/templates/personas/` — role persona prompts (advisor, pi, theorist,
  pm, ra-stata, ra-python, referee, replicator) and 9 shipped expert
  personas
- `core/templates/personas/_stubs/` — 3 v0.2 expert stubs
  (survey-experiment, computation, coauthor-liaison)
- `core/templates/experts/` — 12 expert role configs (9 shipped + 3 stub)
  including the new **LLM-as-Measurement Specialist** paired with the
  consultant team's seat 5
- `adapters/codex/` — Codex plugin and skill assets
- `adapters/claude-code/` — Claude Code project-local commands and agents
- `adapters/opencode/` — OpenCode project-local config, commands, and prompts
- `examples/` — synthetic examples for all three runtimes
- `tests/acceptance.test.sh` — 15 structural invariants (passes on every push)
- `.aiplus/aieconlab/acceptance/v0.1.0/schema.yaml` — binding acceptance schema

## Contributing

We welcome contributions that stay within the plugin's scope (role separation
and execution for applied-economics research, not software engineering and
not advisory consulting).

1. **Open an issue first** for anything larger than a typo fix — the
   `aieconlab` scope is tightly bounded.
2. **Follow the existing TOML + markdown persona pattern** — per-agent
   config lives in `.aiplus/agents/<role>.toml`, persona prompt in
   `.aiplus/agents/personas/<role>.md`.
3. **Add adapter parity** — if you change CLI surface, update all three
   adapters (`adapters/codex/`, `adapters/claude-code/`, `adapters/opencode/`).
4. **Run `aiplus agent doctor`** after config changes to validate worktrees,
   memory layout, and TOML schema.
5. **Acceptance criteria** are binding — see
   `.aiplus/aieconlab/acceptance/v0.1.0/schema.yaml`. Any behavioral change
   must update the schema and its sibling `.test.sh`.

## Safety boundaries

AiEconLab does not:

- upload agent state, persona, memory, or transcript to any service
- run as a background daemon or persistent process
- store secrets, IRB-protected paths, or restricted archive locations in
  any agent's persona, memory, or workspace
- modify global agent configuration (~/.codex, ~/.claude, etc.)
- modify another project's `.aiplus/`
- automatically approve Owner-gated actions (submit to journal, send referee
  response, share data, push paper to public archive, claim authorship order)
- introduce new network calls beyond what the host runtime already makes

## More

- Main platform: [AiPlus](https://github.com/izhiwen/AiPlus)
- Sibling module: [AiPlus-Agent-Team](https://github.com/izhiwen/AiPlus-Agent-Team)

## License

[Apache-2.0](LICENSE)
