# AiPlus Compact Reminder Claude Code Adapter

This directory contains a Claude Code plugin-shaped adapter for the AiPlus compact checkpoint and resume protocol. It is project-local documentation and assets only.

## Files

- `.claude-plugin/plugin.json` defines the local plugin identity.
- `skills/compact-protocol/SKILL.md` describes the compaction operating rules.
- `commands/compact-checkpoint.md` documents `/aiplus-compact-reminder:compact-checkpoint`.
- `commands/compact-resume.md` documents `/aiplus-compact-reminder:compact-resume`.
- `commands/compact-validate.md` documents `/aiplus-compact-reminder:compact-validate`.
- `hooks/hooks.example.json` is an optional reviewed-before-use hook example.

Claude Code plugin layout keeps `plugin.json` inside `.claude-plugin/`; `commands/`, `skills/`, and `hooks/` stay at the plugin root.

## Custom Slash Commands

When this directory is loaded as a Claude Code plugin, command files under `commands/` are exposed as namespaced slash commands:

- `/aiplus-compact-reminder:compact-checkpoint`
- `/aiplus-compact-reminder:compact-resume`
- `/aiplus-compact-reminder:compact-validate`

The command files are prompt documents. They do not run shell scripts, change global settings, publish packages, push commits, or create tags.

## Loading Locally

Ordinary users should install the project-local adapter through the AiPlus CLI
(`aiplus`):

```bash
cd <TARGET_PROJECT>
aiplus install claude-code
```

Then type this in the already-open Claude Code session:

```text
刷新
```

English also works:

```text
refresh
```

For direct adapter inspection in a local checkout, load this adapter explicitly:

```bash
claude --plugin-dir ./aiplus-compact-reminder/adapters/claude-code
```

You can then invoke the namespaced slash commands in Claude Code.

When shell validation is needed, use:

```bash
aiplus compact remind
aiplus compact remind --event phase-end
aiplus compact prepare
aiplus compact validate
aiplus compact checkpoint
aiplus compact resume
```

Compact Reminder's primary behavior is proactive reminder timing. For HEAVY tasks,
run `aiplus compact remind --event long-session` at least every 30 minutes and
at major phase boundaries, before review/QA, before many subagents, before
release prep, and before Owner handoff. For MEDIUM tasks, run it at phase
boundaries and before review/QA. For LIGHT tasks, run it only on user request or
an obvious handoff point.

Before compact, the agent should treat natural language such as "prepare compact" or "save progress" as the primary interface and run `aiplus compact prepare`. If `REMINDER_DECISION=prepare_only`, update handoff/checkpoint first. If `REMINDER_DECISION=wait` or `blocked`, explain the safety reason and keep working. If checkpoint state is ready, it can recommend
manual compact and explain that resume is best-effort: after compact, run
`aiplus compact resume`; if the agent does not reply, explicit AiPlus messages
such as `AiPlus 刷新`, `刷新 AiPlus`, `aiplus refresh`, `aiplus status`,
`AiPlus status`, `继续 AiPlus`, or `resume AiPlus` should restart the resume
flow. Generic messages such as `继续`, `刷新`, `refresh`, `continue`, `resume`,
`go on`, or `接着` should try AiPlus first when possible; when project-specific refresh
rules conflict, report AiPlus status before project status.

## Watch Mode

For daemon-lite continuous monitoring:

```bash
aiplus compact watch --once
aiplus compact watch --interval 10m
```

`watch` uses the same conservative decision logic as `remind` and updates
`.codex/compact/reminder-state.json`. It is safe to interrupt (Ctrl+C).
Watch never triggers host compact automatically.

## Context Capsule

`aiplus compact prepare` creates `.codex/compact/context-capsule.json` with
hierarchical hot/warm/cold tiers, importance scoring, Owner gates, decisions,
and recovery metadata. The capsule is redacted: no secrets, no raw transcript.
It includes checksums for integrity verification. Use the capsule to resume
objective, current state, and next safe action after compact.

## Safety Boundaries

- AiPlus never triggers host compact automatically.
- AiPlus never captures raw transcript or secret values.
- All state files are project-local (`.codex/compact/`).
- No global agent config edits, no telemetry, no network calls from watch/remind.

## Optional Hook Concept

`hooks/hooks.example.json` shows optional `PreCompact` and `SessionStart` reminder hooks:

- `PreCompact` can remind the agent to prepare or review a compact checkpoint before compaction.
- `SessionStart` can remind the agent to read the latest checkpoint when a session starts, resumes, or returns from compaction.

These hooks are not default-on from this adapter. Review the example before use, adapt it to the project's policy, and validate it with Claude Code before installing or enabling it. Hooks can support the protocol, but they do not provide full automatic safety and do not replace agent review of task scope, workspace state, or checkpoint contents.

## Validation

Recommended local checks:

```bash
jq empty .claude-plugin/plugin.json
jq empty hooks/hooks.example.json
claude plugin validate
```

Run validation from this adapter directory. The `claude plugin validate` command depends on the local Claude Code installation and version.
