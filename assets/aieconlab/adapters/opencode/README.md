# AiEconLab — OpenCode Adapter

## Current state in v0.3.x

This adapter ships OpenCode-native AEL assets. After a project installs
OpenCode support and opts into AiEconLab, AiPlus writes:

- 22 AEL subagents to `.opencode/agents/aieconlab-*.md`
- 4 slash commands to `.opencode/commands/aiel-*.md`
- project-local AEL module assets under `.aiplus/modules/aieconlab/`

The generic `agent` subcommand remains runtime-aware and still works:

```bash
aiplus install opencode       # installs .opencode/ prompts and configs
aiplus agent route pi <task>  # creates worktree, logs dispatch
aiplus agent talk pi          # spawns OpenCode with pi.md pre-loaded
```

The persona definitions (`core/templates/personas/*.md`) remain
runtime-agnostic Markdown. `subagents.toml` maps each AEL role to the
matching persona file and supplies routing descriptions for OpenCode's
agent surface.

## Included files

| File | Purpose |
| --- | --- |
| `subagents.toml` | Manifest of 22 OpenCode subagents: name, routing description, and source persona. |
| `commands/aiel-route.md` | Explicit PI-style routing command. |
| `commands/aiel-talk.md` | Role-switch command for opening a specific AEL persona. |
| `commands/aiel-fire-consultant.md` | Research consultant-table command for non-trivial plans. |
| `commands/aiel-status.md` | Team status snapshot command. |

## Install check

```bash
aiplus install opencode
aiplus add aieconlab
aiplus doctor
```

Expected result: `.opencode/agents/` contains 22 prefixed AEL agent
files, `.opencode/commands/` contains the four `/aiel-*` commands, and
`aiplus doctor` reports `DOCTOR_STATUS=PASS`.

## Role switching from natural language

OpenCode's interactive TUI recognizes role switches like "you are PI"
or "switch to the Referee" without explicit slash-command invocation.
The `subagents.toml` description gives OpenCode's agent picker enough
signal to re-bind the active persona mid-session. Note: in
non-interactive `opencode run` mode, this is currently limited by
OpenCode itself — we're tracking the upstream fix.
