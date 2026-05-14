# AiEconLab — Claude Code Adapter

## Current state in v0.1.x

This directory is **intentionally minimal in v0.1**. AEL's role
dispatch and persona embodiment work today via the AiPlus CLI's
generic `agent` subcommand, which is itself runtime-aware:

```bash
aiplus install claude-code    # installs .claude/ commands
aiplus agent route pi <task>  # creates worktree, logs dispatch
aiplus agent talk pi          # spawns Claude Code with pi.md pre-loaded
```

The reason there are no Claude-Code-specific assets shipped under
`adapters/claude-code/` in v0.1 is that all the Claude Code integration
sits one layer up in the AiPlus CLI. The persona definitions
(`core/templates/personas/*.md`) are runtime-agnostic Markdown.

## What v0.2 will add here

Once AiPlus CLI's `agent talk` flow ships richer runtime hooks (Phase D),
this directory will hold:

- A Claude Code slash-command bundle (`/aiel-route`, `/aiel-talk-pi`,
  `/aiel-fire-consultant`) for one-click access from inside a session
- A Claude Code `subagents/` registration (AEL roles as named subagents
  Claude Code can dispatch to)
- Claude-Code-specific session-start prompts that load AEL persona
  context without explicit invocation

## Why this isn't blocked on v0.1 usage

Everything an AEL user needs today works through `aiplus agent *`
regardless of which adapter directory has files in it. The
runtime-specific assets here are ergonomic improvements for v0.2,
not capability gaps.

See [AiPlus Phase D
roadmap](https://github.com/izhiwen/AiPlus/blob/main/docs/roadmap/)
for the runtime adapter work that gates v0.2 here.
