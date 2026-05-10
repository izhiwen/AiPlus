# AiPlus Auto Compact OpenCode Adapter

This directory contains project-local OpenCode assets for the AiPlus compact checkpoint and resume protocol.

## Files

- `opencode.json.example` shows optional project-local OpenCode config for the compact agents and commands.
- `agents/compact-advisor.md` defines a subagent for checkpoint and resume preparation.
- `agents/compact-reviewer.md` defines a subagent for validation and review.
- `commands/compact-checkpoint.md` documents `/compact-checkpoint`.
- `commands/compact-resume.md` documents `/compact-resume`.
- `commands/compact-validate.md` documents `/compact-validate`.
- `prompts/compact-protocol.md` contains shared protocol guidance.

## Project-Local Setup

Ordinary users should install the project-local adapter through the AiPlus CLI
(`aiplus`):

```bash
cd <TARGET_PROJECT>
aiplus install opencode
```

Then type this in the already-open OpenCode session:

```text
刷新
```

English also works:

```text
refresh
```

Use the files in this adapter for direct inspection or manual project-local
setup. This adapter does not modify global OpenCode configuration by default.

Manual example from a project root:

```bash
mkdir -p .opencode/agents .opencode/commands .opencode/prompts
cp aiplus-auto-compact/adapters/opencode/opencode.json.example .opencode/opencode.json
cp aiplus-auto-compact/adapters/opencode/agents/*.md .opencode/agents/
cp aiplus-auto-compact/adapters/opencode/commands/*.md .opencode/commands/
cp aiplus-auto-compact/adapters/opencode/prompts/*.md .opencode/prompts/
```

If your project already has `.opencode/opencode.json`, merge the `agent` and `command` entries manually instead of overwriting it.

Do not copy these files into `~/.config/opencode/` unless the project owner explicitly wants global behavior.

## Commands

After project-local setup, run the commands in the OpenCode TUI:

- `/compact-checkpoint`
- `/compact-resume`
- `/compact-validate`

The command files are prompt documents. They do not run shell scripts, publish packages, push commits, create tags, or change global settings.

## Agents

The example config registers:

- `compact-advisor` for checkpoint and resume planning.
- `compact-reviewer` for validation and readiness review.

They are designed for a Plan/Build style workflow:

- Plan style: inspect scope, prepare checkpoint packets, validate resume readiness, and report blockers before edits.
- Build style: continue implementation only after the active task scope and conflict rules still permit the next action.

## Core CLI Reference

When shell validation is needed, use the AiPlus CLI (`aiplus`):

```bash
aiplus compact remind
aiplus compact remind --event phase-end
aiplus compact prepare
aiplus compact validate
aiplus compact checkpoint
aiplus compact resume
```

Auto Compact's primary behavior is proactive reminder timing. For HEAVY tasks,
run `aiplus compact remind --event long-session` at least every 30 minutes and
at major phase boundaries, before review/QA, before many subagents, before
release prep, and before Owner handoff. For MEDIUM tasks, run it at phase
boundaries and before review/QA. For LIGHT tasks, run it only on user request or
an obvious handoff point.

### Watch Mode

For daemon-lite monitoring during long sessions:

```bash
aiplus compact watch --once
aiplus compact watch --interval 10m
aiplus compact watch --once --json
```

Watch evaluates the same conservative decision logic as `remind` and records
the result to `.codex/compact/reminder-state.json`. It is safe to interrupt
with Ctrl+C. Watch never triggers host compact automatically.

### Context Capsule

`aiplus compact prepare` creates `.codex/compact/context-capsule.json` with
hierarchical hot/warm/cold tiers, importance scoring, Owner gates, decisions,
and recovery metadata. The capsule is redacted (no secrets, no raw transcript)
and includes checksums. Use it to resume objective, current state, and next safe
action after compact.

### Safety Boundaries

- AiPlus never triggers host compact automatically.
- AiPlus never captures raw transcript or secret values.
- All state files are project-local (`.codex/compact/`).
- No global agent config edits, no telemetry, no network calls from watch/remind.

Before compact, run validate and checkpoint. If `REMINDER_DECISION=prepare_only`,
update handoff/checkpoint first. If `REMINDER_DECISION=wait` or `blocked`,
explain the safety reason and keep working. If checkpoint state is ready, the
agent can recommend manual compact and explain best-effort resume: after compact,
run `aiplus compact resume`; if the agent does not reply, explicit AiPlus
messages such as `AiPlus 刷新`, `刷新 AiPlus`, `aiplus refresh`,
`aiplus status`, `AiPlus status`, `继续 AiPlus`, or `resume AiPlus` should
restart the resume flow. Generic messages such as `继续`, `刷新`, `refresh`,
`continue`, `resume`, `go on`, or `接着` should try AiPlus first when possible;
when project-specific refresh rules conflict, report AiPlus
status before project status.

`../../core/scripts/compactctl.mjs` remains only as archived history and a
compatibility test fixture. It is not the ordinary-user path and must not be
used as a fallback if `aiplus` is missing.

## Limits

This adapter is guidance, not a safety system. It does not prove correct
recovery, prevent scope mistakes, wake OpenCode if it is waiting for a user
message, or make compaction safe without agent or human review.

## Validation

Recommended local checks:

```bash
jq empty opencode.json.example
find agents commands prompts -type f -maxdepth 2 | sort
```

Run validation from this adapter directory.
