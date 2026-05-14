# AiEconLab — OpenCode Adapter

## Current state in v0.1.x

This directory is **intentionally minimal in v0.1**. AEL's role
dispatch and persona embodiment work today via the AiPlus CLI's
generic `agent` subcommand, which is itself runtime-aware:

```bash
aiplus install opencode       # installs .opencode/ prompts and configs
aiplus agent route pi <task>  # creates worktree, logs dispatch
aiplus agent talk pi          # spawns OpenCode with pi.md pre-loaded
```

The reason there are no OpenCode-specific assets shipped under
`adapters/opencode/` in v0.1 is that all the OpenCode integration sits
one layer up in the AiPlus CLI. The persona definitions
(`core/templates/personas/*.md`) are runtime-agnostic Markdown.

## What v0.2 will add here

Once AiPlus CLI's `agent talk` flow ships richer runtime hooks
(Phase D), this directory will hold:

- OpenCode-specific `aiplus` key in `opencode.json` (migration from
  the legacy aiplus block to the new agent-team-aware structure)
- OpenCode prompts that pre-load AEL persona context for long-running
  research sessions
- OpenCode-specific MCP server bindings so AEL roles appear in the
  MCP tool surface (per AiPlus Phase E)

## Why this isn't blocked on v0.1 usage

Everything an AEL user needs today works through `aiplus agent *`
regardless of which adapter directory has files in it. The runtime-
specific assets here are ergonomic improvements for v0.2, not
capability gaps.

See [AiPlus Phase D / Phase E roadmap](https://github.com/izhiwen/AiPlus/blob/main/docs/roadmap/)
for the runtime adapter work that gates v0.2 here.
