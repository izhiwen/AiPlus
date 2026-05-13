# AiPlus
[![CI](https://github.com/izhiwen/AiPlus/actions/workflows/ci.yml/badge.svg)](https://github.com/izhiwen/AiPlus/actions/workflows/ci.yml)
[![License: Apache 2.0](https://img.shields.io/badge/License-Apache_2.0-blue.svg)](LICENSE)

[中文 README](README.zh-CN.md)

I've been pair-programming with AI coding agents full-time for the better
part of a year — Codex one day, Claude Code the next, OpenCode for the
long-running stuff. About four months in, I caught myself explaining the
same architectural decision to the same agent for the fourth time that
week, and I realized I was losing hours every day to the same six
coordination failures: cross-session amnesia, post-compact context loss,
agents racing to lead the same task, estimates anchored to human-engineer
hours, plans that skipped security until release week, and one agent
wearing every hat in one context window. AiPlus is the five small Rust
modules I built to treat all six. The honest meta-frame: **I used AI agents
to build the toolchain that manages AI agents.** What's here works for my
workflow today; what isn't yet is in `docs/roadmap/`.

![AiPlus 30-second tour](docs/demo.gif)

## The pains we are tired of

If you spend your days driving AI coding agents, these probably feel familiar:

1. **The agent forgets everything between sessions.** Monday you teach it your
   naming conventions. Wednesday it asks again. By Friday you have explained
   the same architectural decision four times.
2. **Long tasks lose context after compact.** You hit the token wall mid-feature.
   Compact happens. The agent comes back asking the question you answered
   forty minutes ago, and the half-finished plan is gone.
3. **Multiple agents step on each other's feet.** No one defined who is the
   CEO, who reviews, who builds. Three agents try to lead the same task.
4. **Estimates anchor on human-engineer hours.** The agent says "five hours"
   for a refactor. Twenty minutes later it is done. Next week, same task, same
   five-hour quote. No one keeps the score.
5. **When agents plan, they overlook the things that matter most** — onboarding
   ease, security and privacy, real execution pitfalls, AI integration
   considerations. You only find out at release week, or worse, from a user
   complaint.
6. **One agent wears every hat.** CEO, reviewer, builder, advisor — all crammed
   into the same context window. Roles **drift**, context **pollutes** across
   hats, and the agent does each hat **shallowly**. Real engineering teams
   divide labor because the work *is* that structured.

AiPlus is five small modules that, together, fix all six.

## What you get

**Agent Memory** — The agent stops forgetting. Project conventions, naming
rules, architectural decisions are stored as local JSONL under
`.aiplus/memory/`. Twelve redaction patterns strip secrets before any record
is written, so you can capture preferences without leaking them.

**Compact Reminder** — The agent stops blanking out after compact. It tells
you when it is a good time to compact (not too early, not too late), prepares
a structured handoff before compaction, and auto-resumes from a verified
capsule afterwards. The agent picks up where it left off, not from zero.

**Auto Team Consultant** — The agent stops overlooking the important stuff.
A virtual team (5 expert members + your project's user personas, sitting at
the same table) is consulted before every non-trivial plan. A coordinator
scales the consult by complexity and risk so you get the value of a real
review team without paying the cost on every commit.

**Agent Velocity** — The agent stops mis-billing its own work. Every estimate
and actual completion time is logged as local JSONL. Human-time bias is
detected automatically. Future estimates use AI-native p50 and p90 numbers
calibrated from your own history.

**Agent Team** — Replace single-agent **drift** with a permanent team.
Advisor, CEO, Architect, PM, two Engineers, Reviewer, and QA — each with its
own persona, workspace, and memory namespace. A coordinator routes work to the
right role, keeps transcripts, and prunes stale worktrees so your project stays
clean. No more role pollution, no more shallow-each-hat.

Everything stays inside `.aiplus/` in your project. Nothing uploads. Nothing
syncs to a cloud. Nothing edits your global agent config.

## Install

Install the `aiplus` command:

```bash
curl -fsSL https://raw.githubusercontent.com/izhiwen/AiPlus/main/install.sh \
  | bash
```

If you'd rather inspect the script before running it:

```bash
curl -fsSL https://raw.githubusercontent.com/izhiwen/AiPlus/main/install.sh -o install.sh
less install.sh
bash install.sh
```

Install AiPlus into your project:

```bash
cd MyProject
aiplus install codex          # or: claude-code, opencode, all
```

Verify:

```bash
aiplus status
aiplus doctor
```

## Runtime support

| Runtime     | Install command              | Where adapters land                      |
|-------------|------------------------------|------------------------------------------|
| Codex       | `aiplus install codex`       | Managed block in `AGENTS.md`             |
| Claude Code | `aiplus install claude-code` | `.claude/` commands                      |
| OpenCode    | `aiplus install opencode`    | `.opencode/` prompts                     |
| All three   | `aiplus install all`         | All adapters                             |

Each adapter is project-local. We do not touch your global config.

## Daily commands

```bash
aiplus status                        # status across all modules
aiplus doctor                        # health checks across all modules

# Memory
aiplus memory status
aiplus memory context --runtime codex --budget 2000

# Compact
aiplus compact prepare               # build handoff + capsule
aiplus compact resume                # restore after compact
aiplus compact savings               # token + cost savings

# Velocity
aiplus velocity estimate --task-type feature --human-estimate 5h
aiplus velocity report

# Agent Team
aiplus agent status              # Show team status
aiplus agent route engineer-a    # Assign task to engineer-a
aiplus agent integrate engineer-a # Merge work back
aiplus agent audit run           # Run acceptance audit
aiplus agent talk <role>
aiplus agent invite <role>
aiplus agent dismiss <role>
aiplus agent transcript
aiplus agent prune-worktrees

# Updates
aiplus update all
```

## Architecture

```
MyProject/
├── .aiplus/
│   ├── memory/                  # JSONL memory records
│   ├── identities/              # Role identity definitions
│   ├── agents/                  # Agent team role definitions and state
│   ├── agent-memory/            # Agent continuity and context records
│   ├── consultant-team.toml     # Team routing config
│   └── velocity/                # Estimate and run records
├── .codex/compact/              # Compact handoffs and capsules
├── .claude/                     # Claude Code adapters (if installed)
├── .opencode/                   # OpenCode adapters (if installed)
└── AGENTS.md                    # Codex managed block (if installed)
```

## The five standalone modules

Each module also ships as its own GitHub repo if you want to inspect or
adopt one piece at a time:

- [AiPlus-Agent-Memory](https://github.com/izhiwen/AiPlus-Agent-Memory)
- [AiPlus-Compact-Reminder](https://github.com/izhiwen/AiPlus-Compact-Reminder)
- [AiPlus-Auto-Team-Consultant](https://github.com/izhiwen/AiPlus-Auto-Team-Consultant)
- [AiPlus-Agent-Velocity](https://github.com/izhiwen/AiPlus-Agent-Velocity)
- [AiPlus-Agent-Team](https://github.com/izhiwen/AiPlus-Agent-Team)

## Safety boundaries

AiPlus stays inside your project. It does not:

- upload project data, prompts, or transcripts
- emit telemetry, sync to cloud, or call external services
- edit global agent configuration
- store secrets in memory, compact files, or ledgers
- approve Owner-gated actions on its own
- publish packages, create tags, or push releases

Validation is structural and heuristic. It is not a safety or compliance
certification.

## Private profiles

AiPlus supports optional user-level private profiles for personal preferences
and secret aliases under `~/.config/aiplus/profiles/`. Private profiles are
never bundled into public repositories. See `aiplus profile install` and
`aiplus secret-broker` documentation for details.

## Status

Current cut: **v0.5.1**. See
[v0.5.2 known gaps](docs/roadmap/v0.5.2-known-gaps.md) for tracked work
before the next release.

## License

[Apache-2.0](LICENSE)
