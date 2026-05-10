# AiPlus

[中文 README](README.zh-CN.md)

## The Pain

You open a new agent session and start explaining the same project conventions for the third time this week. Halfway through a long Codex task, the context fills up and the agent loses half the thread. After compact, it comes back amnesiac, asking questions you already answered. When you ask three different agents to collaborate, they step on each other because no one agreed on who is CEO, who is reviewer, and who is builder. And when the agent says "this will take 5 hours," you know from experience it usually finishes in twenty minutes, but no one writes that down.

## The Solution

AiPlus turns agent workflows into trustworthy engineering practices. It keeps project-local memory under `.aiplus/` so the agent remembers conventions across sessions. Auto Compact prepares structured handoffs before context runs out, then resumes from a checksum-verified capsule after. Auto Team Consultant installs a decision system that routes tasks through the right role at the right depth. Agent Velocity quietly records estimates against actuals, detects human-time bias, and adjusts the next guess. Everything stays local. Nothing uploads. Nothing phones home.

## Quick Start

Install the `aiplus` command:

```bash
curl -fsSL https://raw.githubusercontent.com/izhiwen/aiplus/main/install.sh | bash
```

Install AiPlus into your project:

```bash
cd MyProject
aiplus install codex
```

Verify everything is healthy:

```bash
aiplus status
aiplus doctor
```

## Runtime Choices

AiPlus supports three AI coding agents. Install for one or all:

```bash
aiplus install codex        # Codex CLI
aiplus install claude-code  # Claude Code
aiplus install opencode     # OpenCode
aiplus install all          # All three
```

Each runtime gets project-local adapter files:
- **Codex** — managed block in `AGENTS.md`
- **Claude Code** — files under `.claude/`
- **OpenCode** — files under `.opencode/`

## What's Inside

- **Agent Memory** (`agent-memory`) — project-local JSONL memory, role identity, and skill candidate governance. Twelve redaction patterns strip sensitive strings before writing.
- **Auto Compact** (`auto-compact`) — proactive compact reminder, checkpoint, handoff, and resume workflow. Creates checksum-verified context capsules.
- **Auto Team Consultant** (`auto-team-consultant`) — L0-L5 routing with Advisor, CEO, Reviewer, and Builder lenses. Installs `.aiplus/consultant-team.toml` with sensible defaults.
- **Agent Velocity** (`agent-velocity`) — AI-native time calibration with bias detection and retention. Stores estimates and actuals as local JSONL.

## Common Commands

```bash
aiplus status                    # Show all module status
aiplus doctor                    # Run health checks
aiplus update all               # Update CLI and project modules
aiplus memory status            # Show memory records and identities
aiplus compact savings          # Show compact savings estimate
aiplus velocity report          # Show velocity bias report
aiplus skill-candidate status   # Show proposed skills
aiplus profile status           # Show private profile (if installed)
```

## Safety Boundaries

AiPlus does not:
- Upload project data, prompts, or transcripts
- Implement telemetry or cloud sync
- Edit global Codex, Claude Code, or OpenCode configuration
- Store secrets in compact files, memory, or ledgers
- Automatically approve Owner-gated actions
- Publish packages, create tags, or make releases

Validation is structural and heuristic, not a safety or compliance certification.

## Private Profiles

AiPlus supports optional user-level private profiles for personal preferences and secret aliases. See the full docs for `aiplus profile install` and `aiplus secret-broker` usage.

## Roadmap

See [v0.5.2 known gaps](docs/roadmap/v0.5.2-known-gaps.md) for current technical debt and deferred work.

## License

[Apache-2.0](LICENSE)
