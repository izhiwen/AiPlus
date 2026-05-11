# AiPlus
[中文 README](README.zh-CN.md)

## Why This Exists

Your agent drifts. Monday morning, you open a new session and start explaining project conventions for the third time this week. By Wednesday, the agent has forgotten the naming rules. Halfway through a long Codex task, the context window fills up and the agent loses half the thread. After compact, it comes back asking questions you already answered. When multiple agents collaborate, they step on each other because nobody defined who leads, who reviews, and who builds. And when the agent estimates five hours for a task that typically takes twenty minutes, nobody writes that down, so the next estimate is equally wrong.

AiPlus fixes these problems with four integrated modules that run entirely on your machine.

## What It Does

**Agent Memory** stores project conventions as JSONL under `.aiplus/memory/`. Before any record is written, twelve redaction patterns strip sensitive strings like passwords, JWTs, and raw transcripts. The agent remembers your naming rules, coding standards, and architectural decisions across sessions. Rejected or forgotten records stay in the store but remain hidden from context.

**Compact Reminder** prepares structured handoffs before the context window fills up. It captures the decision log, agent state, and evidence into a checksum-verified capsule. After compact, `aiplus compact resume` reads the capsule and restores context automatically. The agent continues from where it left off, not from zero.

**Auto Team Consultant** installs a routing system into your project. It defines clear roles: Advisor for direct advice, CEO for task breakdown, Reviewer for findings, and Builder for implementation. Tasks route through L0 direct advice up to L5 full governance. AI Integration is a default specialist team member, not an afterthought.

**Agent Velocity** records every estimate and actual completion time as local JSONL under `.aiplus/velocity/`. It detects human-time bias when estimates anchor on engineer hours instead of agent minutes. After a few records, it produces p50 and p90 AI-native estimates and adjusts the next guess.

Everything stays in `.aiplus/` inside your project. Nothing uploads. Nothing leaves your machine.

## Installation

Install the `aiplus` command:

```bash
curl -fsSL https://raw.githubusercontent.com/izhiwen/aiplus/main/install.sh \
  | bash
```

Install AiPlus into your project:

```bash
cd MyProject
aiplus install codex        # or: claude-code, opencode, all
```

Verify the installation:

```bash
aiplus status
aiplus doctor
```

## Runtime Support

AiPlus supports three AI coding agents with project-local adapter files:

| Runtime | Install Command | Adapter Files |
|---------|----------------|---------------|
| Codex | `aiplus install codex` | Managed block in `AGENTS.md` |
| Claude Code | `aiplus install claude-code` | Commands under `.claude/` |
| OpenCode | `aiplus install opencode` | Prompts under `.opencode/` |
| All three | `aiplus install all` | All adapters |

Install for one runtime or all. Each adapter is project-local and does not touch global configuration.

## Daily Commands

```bash
# Status and health
aiplus status                      # Show all module status
aiplus doctor                      # Run health checks across modules

# Memory
aiplus memory status              # Show memory records and identities
aiplus memory context --runtime codex --budget 2000

# Compact
aiplus compact prepare            # Build handoff and context capsule
aiplus compact resume             # Resume after compact
aiplus compact savings            # Show token and cost savings

# Velocity
aiplus velocity estimate --task-type feature --human-estimate 5h
aiplus velocity report            # Show bias and adjustment report

# Team
aiplus skill-candidate status     # Show proposed skills

# Updates
aiplus update all                 # Update CLI and all project modules
```

## Architecture

```
MyProject/
├── .aiplus/
│   ├── memory/              # JSONL memory records
│   ├── identities/          # Role identity definitions
│   ├── skills/              # Skill candidates
│   ├── consultant-team.toml # Team routing config
│   └── velocity/            # Estimate and run records
├── .codex/compact/          # Compact handoffs and capsules
├── .claude/                 # Claude Code adapters (if installed)
├── .opencode/               # OpenCode adapters (if installed)
└── AGENTS.md                # Codex managed block (if installed)
```

## Safety Boundaries

AiPlus operates entirely within your project directory:

- No uploads of project data, prompts, or transcripts
- No telemetry, cloud sync, or external services
- No edits to global Codex, Claude Code, or OpenCode configuration
- No storage of secrets in compact files, memory, or ledgers
- No automatic approval of Owner-gated actions
- No package publishing, tag creation, or releases

Validation is structural and heuristic, not a safety or compliance certification.

## Private Profiles

AiPlus supports optional user-level private profiles for personal preferences and secret aliases. These live under `~/.config/aiplus/profiles/` and are never bundled into public repositories. See the full documentation for `aiplus profile install` and `aiplus secret-broker` usage.

## Project Status

Current version: v0.5.1 with v2.1 hardening for all modules.

See [v0.5.2 known gaps](docs/roadmap/v0.5.2-known-gaps.md) for technical debt and planned work.

## License

[Apache-2.0](LICENSE)
