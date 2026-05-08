# AiPlus Auto Compact Claude Code Adapter

This directory contains a Claude Code plugin-shaped adapter for the AiPlus compact checkpoint and resume protocol. It is project-local documentation and assets only.

## Files

- `.claude-plugin/plugin.json` defines the local plugin identity.
- `skills/compact-protocol/SKILL.md` describes the compaction operating rules.
- `commands/compact-checkpoint.md` documents `/aiplus-auto-compact:compact-checkpoint`.
- `commands/compact-resume.md` documents `/aiplus-auto-compact:compact-resume`.
- `commands/compact-validate.md` documents `/aiplus-auto-compact:compact-validate`.
- `hooks/hooks.example.json` is an optional reviewed-before-use hook example.

Claude Code plugin layout keeps `plugin.json` inside `.claude-plugin/`; `commands/`, `skills/`, and `hooks/` stay at the plugin root.

## Custom Slash Commands

When this directory is loaded as a Claude Code plugin, command files under `commands/` are exposed as namespaced slash commands:

- `/aiplus-auto-compact:compact-checkpoint`
- `/aiplus-auto-compact:compact-resume`
- `/aiplus-auto-compact:compact-validate`

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
claude --plugin-dir ./aiplus-auto-compact/adapters/claude-code
```

You can then invoke the namespaced slash commands in Claude Code.

When shell validation is needed, use:

```bash
aiplus compact validate
aiplus compact checkpoint
aiplus compact resume
```

Before compact, the agent should run `aiplus compact validate` and
`aiplus compact checkpoint`. If checkpoint state is ready, it can recommend
manual compact and explain that resume is best-effort: when Claude Code returns
control automatically, the agent should run `aiplus compact resume`; if Claude
Code waits for the user, messages such as `继续`, `刷新`, `refresh`, `continue`,
`resume`, `go on`, or `接着` should restart the resume flow.

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
