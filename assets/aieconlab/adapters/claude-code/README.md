# AiEconLab — Claude Code Adapter

## Current state in v0.2.x

This directory ships the Claude Code adapter content that the AiPlus
installer reads when running `aiplus add aieconlab` against a project
that already has `aiplus install claude-code` applied. After install,
**Claude Code agents auto-use AEL features without you reminding them
each session.**

What lands in the user's project:

- `.claude/agents/aieconlab-*.md` — 20 subagents (8 core roles + 12
  expert specialists) with YAML frontmatter tuned for Claude Code's
  auto-routing. Persona bodies are loaded from
  `core/templates/personas/*.md`.
- `.claude/commands/aiel-*.md` — 4 slash commands (`/aiel-route`,
  `/aiel-talk`, `/aiel-fire-consultant`, `/aiel-status`) for explicit
  invocation when auto-routing isn't what you want.
- AEL managed block in `CLAUDE.md` (separate markers from AiPlus's
  block) — declares the research team, expert directory, natural-
  language routing map, and coordinator discipline.

The AEL block coexists with the AiPlus block. AiPlus's `SessionStart` /
`PreCompact` hooks (declared in `.claude/settings.local.json`) continue
to fire and inject project memory + prepare compact handoff. AEL adds
the research-team layer on top.

## File map (this directory)

| File | Purpose |
|---|---|
| `subagents.toml` | Manifest of 20 Claude Code subagents — name, description (drives auto-routing), and `persona_file` (system prompt source). |
| `claude-md-block.md` | Body of the `<!-- BEGIN AIECONLAB MANAGED BLOCK -->` … `<!-- END AIECONLAB MANAGED BLOCK -->` section that AiPlus inserts into the user's project `CLAUDE.md`. |
| `commands/aiel-route.md` | Slash command — explicit PI-style task routing. |
| `commands/aiel-talk.md` | Slash command — load a specific role's persona as active context. |
| `commands/aiel-fire-consultant.md` | Slash command — fire the research-tuned consultant team before non-trivial plans. |
| `commands/aiel-status.md` | Slash command — one-shot team status. |

## How it gets installed

1. User runs `aiplus install claude-code` in their project — writes the
   5 AiPlus subagents, `.claude/settings.local.json` hooks, and the
   AiPlus CLAUDE.md block.
2. User runs `aiplus add aieconlab` — AiPlus reads this directory's
   contents from its embedded asset table and:
   - For each entry in `subagents.toml`, reads the matching persona
     file, prepends YAML frontmatter (`name`, `description`), and
     writes `.claude/agents/aieconlab-<name>.md`.
   - Copies each `commands/*.md` to `.claude/commands/`.
   - Inserts the contents of `claude-md-block.md` between
     `<!-- BEGIN AIECONLAB MANAGED BLOCK -->` and
     `<!-- END AIECONLAB MANAGED BLOCK -->` in the project root
     `CLAUDE.md` (creates the file if missing; preserves user content
     outside the block).
3. Optionally: re-run `aiplus install claude-code` later to refresh
   AiPlus content; AEL content survives because markers differ.
4. `aiplus doctor` verifies all 20 subagents, slash commands, and the
   AEL managed block are present.

## Uninstall

`aiplus uninstall --yes` strips the AEL managed block from `CLAUDE.md`
and the AiPlus managed block. Subagent and slash-command files are
left in place (consistent with AiPlus's adapter-file retention) — they
become inert without the `aiplus` binary on PATH, and they can be
manually deleted if desired.

## Safety boundaries

- No global config edits (`~/.claude/` is never touched).
- No secrets written. Persona files, role configs, and consultant-team
  config never contain credentials or restricted data paths.
- Owner-gated actions (journal submission, posting, sending referee
  responses, data sharing, authorship-order changes) never auto-fire
  from subagents or slash commands. They surface as recommendations
  awaiting explicit Owner confirmation.
