# AiEconLab

[![License: Apache 2.0](https://img.shields.io/badge/License-Apache_2.0-blue.svg)](LICENSE)

[中文 README](README.zh-CN.md)

AiEconLab gives AI-assisted economics projects a research-team structure.
Instead of asking one chat to be PI, RA, theorist, referee, and replicator at
once, AEL gives each role a separate persona, workspace boundary, and set of
responsibilities.

```bash
curl -fsSL https://raw.githubusercontent.com/izhiwen/AiEconLab/main/install.sh | bash
cd MyPaperProject
ael install
ael talk advisor "What is your role?"
```

The first command installs the `ael` CLI. Inside a paper or replication
project, `ael install` sets up the economics research team for your local AI
runtime. `ael talk advisor ...` opens a role-specific conversation.

## I'm New — Start Here

If you've never used Claude Code, Codex, or OpenCode before, do these
three things first (in this order). It'll save you an hour.

### Step 1: Install ONE AI coding agent first

Pick ONE to start (you can add more later):

- **Claude Code** (recommended for most researchers) — install from
  [claude.com/download](https://claude.com/download). Comes with
  Claude Pro; no separate API key needed.
- **Codex** — OpenAI's CLI. Requires a paid OpenAI account.
- **OpenCode** — open source, runs local or remote models.

Confirm the agent works on its own first (open it, ask "hi") before
adding AEL on top.

### Step 2: Install AEL

Open the macOS Terminal app (or Linux / Windows terminal), paste this
**one line**, and press Enter:

```bash
curl -fsSL https://raw.githubusercontent.com/izhiwen/AiEconLab/main/install.sh | bash
```

If it says "command not added to PATH," follow the one-line fix it
prints. This is a one-time setup per machine.

### Step 3: Try it in a paper project

Make a folder for a paper (or use an existing one), then:

```bash
cd MyPaperProject       # the folder where your paper lives
ael install             # auto-detects your AI agent
ael talk advisor "I'm starting a paper on X. What should I think about first?"
```

Advisor will respond with 3-5 things to think about (identification
strategy, data feasibility, contribution framing). You go from there.

### Common first-day questions

- **"Do I need an API key?"** Not if you already have Claude Pro or
  ChatGPT Plus desktop. Only needed for batch / unattended runs.
- **"Will it touch my real paper files?"** No — read-only by default.
  Each role gets its own isolated workspace under `.aiplus/`.
- **"How do I undo the install?"** `aiplus uninstall --yes` removes
  everything cleanly. Only touches `.aiplus/` and adapter dirs in
  your project.
- **"Is my data uploaded anywhere?"** No. All local. Roles log to
  `.aiplus/` in your project, never to a server.
- **"It says `NEEDS_FIX` — what now?"** Run `aiplus doctor`. The
  most common fix is `aiplus install <runtime>` to refresh the
  adapter. If you're still stuck, open a GitHub issue with the
  doctor output.

---

## Demo

![AiEconLab demo](demo.gif)

## What AEL Adds

AEL is built for applied economists who use AI assistants across long paper
projects: data cleaning, Stata regressions, Python merges, identification
debates, literature positioning, seminar revisions, replication packages, and
referee responses.

It gives you:

- **Advisor** for strategic second opinions on framing, identification risk,
  and publication tradeoffs.
- **PI** for scoping tasks, dispatching roles, integrating results, and keeping
  the project coherent.
- **Theorist** for identification strategy, mechanisms, instruments, and model
  logic.
- **RA-Stata** for Stata analysis, regression tables, robustness checks, and
  reproducible `.do` workflows.
- **RA-Python** for data cleaning, scraping, matching, GIS, and Python
  pipelines.
- **Referee** for pre-submission critique before a draft leaves the team.
- **Replicator** for clean-room reruns and replication-package failures.
- **PM** for deadlines, scope, blockers, and milestone discipline.

There are also specialist roles for literature review, writing, econometrics,
LLM-as-measurement validation, reproducibility engineering, historical sources,
IRB/sensitive-data review, visualization, computation, survey experiments,
degrees-of-freedom auditing, R&R strategy, job talks, and coauthor coordination.

## How the Team Works in Your Runtime

- **Switch roles in plain language.** Mid-session, say "you are PI",
  "take the referee role", or "switch to RA-Stata" and the agent
  responds as that role, with that role's research memory loaded.
  No CLI command. Works in Codex, Claude Code, and OpenCode
  interactive mode.

- **Intent-aware guardrails when PI delegates.** Before PI hands
  off anything risky to an RA — deleting files, modifying live
  data, publishing changes — the coordinator understands what
  you're actually asking for, not just the words you typed.
  Rephrasing or putting things in quotes can't slip a destructive
  command through. Especially useful when replication scripts
  touch shared archives or paper drafts.

- **Parallel review and QA for fast PI → RA → Referee cycles.**
  Review and QA steps run side by side, and each role's workspace
  stays warm between tasks. A typical robustness-table iteration
  lands in ~8-10 min instead of ~15-20, same quality bar. AEL
  inherits this from the underlying AiPlus.

## Install

Install the CLI:

```bash
curl -fsSL https://raw.githubusercontent.com/izhiwen/AiEconLab/main/install.sh | bash
```

If the installer says the target directory is not on `PATH`, add it:

```bash
export PATH="$HOME/.local/bin:$PATH"
```

Then install AEL into a project:

```bash
cd MyPaperProject
ael install
```

By default AEL picks an available runtime in this order: Codex, Claude Code,
OpenCode. You can choose explicitly:

```bash
ael install codex
ael install claude-code
ael install opencode
```

Verify the project setup:

```bash
ael status
ael doctor
```

## Daily Use

Talk to the Advisor:

```bash
ael talk advisor "Is this identification strategy credible enough for a top-field submission?"
```

Route work through the PI:

```bash
ael route pi "scope the next robustness table and dispatch the right RA"
```

Talk to implementation roles when the task is already clear:

```bash
ael talk ra-stata "Sketch the Stata plan for the main IV table."
ael talk ra-python "Plan the merge checks for the county-level panel."
ael talk referee "Give me the harsh pre-submission read of this abstract."
```

Bring in an expert:

```bash
ael invite llm-measurement
ael talk llm-measurement "Review my text-as-data validation plan."
```

## Why Roles Matter

One long-lived AI chat tends to blur responsibilities. The same assistant that
debugged a Stata loop starts drafting prose with code-shaped habits. The same
assistant that helped frame the intro becomes too invested to act like a
skeptical referee.

AEL keeps those jobs separate:

- RA memories stay focused on data, variables, and code decisions.
- Theorist and Referee critiques do not get diluted by execution context.
- PI owns integration instead of letting parallel work collide silently.
- Replicator gets a clean-room mandate rather than sharing the builder's
  assumptions.

The result is not "more agents" for its own sake. It is a project structure
that matches how serious research teams already work.

## LLM-as-Measurement

AEL includes an LLM-as-measurement specialist for projects that use language
models to score archival text, survey responses, open-ended documents, or other
unstructured sources. This role focuses on validation design: multi-model
agreement, held-out human labels, inter-rater statistics, prompt-version
stability, and measurement-error implications for the empirical result.

Companion example:
[Multi-LLM-Validation-Demo](https://github.com/izhiwen/Multi-LLM-Validation-Demo).

![Pairwise LLM correlation heatmap (294 archival docs × 5 frontier LLMs, mean ρ ≈ 0.92)](https://raw.githubusercontent.com/izhiwen/Multi-LLM-Validation-Demo/main/figures/multi_llm_correlation_heatmap.png)

## Safety

AEL stays local to your project. It does not:

- upload project files, memory, or transcripts
- run as a background daemon
- store restricted-data paths or secrets in role personas
- modify unrelated projects
- auto-approve Owner-gated actions such as journal submission, public posting,
  referee-response sending, data sharing, or authorship changes

The CLI installs project files under local project state and uses your selected
runtime to answer as the requested role.

## Release Build

For maintainers:

```bash
git submodule update --init --recursive
scripts/build-ael.sh --package
```

The release workflow publishes platform tarballs and SHA256 sidecars for the
installer.

## Advanced

AEL is built on the AiPlus agent substrate; the supported user-facing product
surface is the `ael` CLI and this repository.

## License

Apache-2.0. See [LICENSE](LICENSE).
