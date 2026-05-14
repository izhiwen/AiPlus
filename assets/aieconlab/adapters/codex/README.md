# AiEconLab — Codex Adapter

## Current state in v0.1.x

This directory is **intentionally minimal in v0.1**. AEL's role
dispatch and persona embodiment work today via the AiPlus CLI's
generic `agent` subcommand, which is itself runtime-aware and adapts
its behavior per host runtime (Codex / Claude Code / OpenCode):

```bash
aiplus install codex          # installs Codex-specific managed block in AGENTS.md
aiplus agent route pi <task>  # creates worktree, logs dispatch
aiplus agent talk pi          # spawns the Codex CLI with pi.md pre-loaded
```

The reason there are no Codex-specific assets shipped under
`adapters/codex/` in v0.1 is that all the Codex integration sits one
layer up, in the AiPlus CLI itself. The persona definitions
(`core/templates/personas/*.md`) are runtime-agnostic Markdown that
Codex / Claude Code / OpenCode all load the same way.

## What v0.2 will add here

Once the AiPlus CLI's `agent talk` flow ships richer runtime hooks
(per the Phase D work tracked in [AiPlus](https://github.com/izhiwen/AiPlus)),
this directory will hold:

- A Codex-specific `SKILL.md` for the AEL team (so Codex's skill picker
  surfaces "AEL: route to Theorist" / "AEL: fire LLM-Measurement
  validity protocol" as one-click affordances)
- Codex hooks for the consultant team's 5 seats (so plan-time consults
  fire automatically before Codex writes a plan, not only when the
  user types `aiplus agent route`)
- Codex-specific managed block templates for `AGENTS.md` so the
  AEL roles appear as a discoverable virtual team in long sessions

## Why this isn't blocked on v0.1 usage

Everything an AEL user needs today works through `aiplus agent
*` regardless of which adapter directory has files in it. The
runtime-specific assets here will improve ergonomics in v0.2 (fewer
explicit `aiplus agent talk` invocations because Codex picks up the
team automatically), but the underlying behavior — persona embodiment,
worktree creation, dispatch logging, STOP-gate escalation — is already
live.

See [AiPlus Phase D
roadmap](https://github.com/izhiwen/AiPlus/blob/main/docs/roadmap/)
for the runtime adapter work that gates v0.2 here.
