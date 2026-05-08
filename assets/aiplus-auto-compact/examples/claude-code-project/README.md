# Synthetic Claude Code Project Example

This example shows how a Claude Code user can apply AiPlus Auto Compact to a synthetic project named `<EXAMPLE_PROJECT>`.

## Load Adapter

```bash
cd <TARGET_PROJECT>
aiplus install claude-code
```

Then type this in the already-open Claude Code session:

```text
刷新
```

English also works: `refresh`.

The adapter command documents may also be used in Claude Code:

```text
/aiplus-auto-compact:compact-validate
/aiplus-auto-compact:compact-checkpoint
/aiplus-auto-compact:compact-resume
```

## Compact Commands

```bash
cd <TARGET_PROJECT>
aiplus compact validate
aiplus compact checkpoint
```

After the user manually completes the Claude Code compact or session reset step:

```bash
aiplus compact resume
```

If the agent continues automatically, you do not need to do anything. If it does
not reply, send a natural continuation such as `continue`, `resume`, `go on`,
`继续`, `刷新`, or `接着`.

## Synthetic Handoff Content

Use placeholder-only entries:

```text
Current goal: Prepare Claude Code handoff for <EXAMPLE_PROJECT>.
Current phase: Review generated state files.
Owner gate: UNKNOWN_PENDING: <OWNER> has not reviewed the synthetic checkpoint yet.
Next safe action: Ask <OWNER> to review the checkpoint before compacting.
Evidence: Adapter command documents were loaded from <REPO_ROOT>.
```

Optional hooks in the Claude Code adapter are examples for reviewed local use. They should be inspected and adapted before enabling.
