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

## What's Inside

- **Agent Memory** (`agent-memory`) — project-local JSONL memory, role identity, and skill candidate governance
- **Auto Compact** (`auto-compact`) — proactive compact reminder, checkpoint, handoff, and resume workflow
- **Auto Team Consultant** (`auto-team-consultant`) — L0-L5 routing with Advisor, CEO, Reviewer, and Builder lenses
- **Agent Velocity** (`agent-velocity`) — AI-native time calibration with bias detection and retention

## Roadmap

See [v0.5.2 known gaps](docs/roadmap/v0.5.2-known-gaps.md) for current technical debt and deferred work.
