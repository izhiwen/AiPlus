# Synthetic OpenCode Project Example

This example shows how an OpenCode user can apply AiPlus Auto Compact to a synthetic project named `<EXAMPLE_PROJECT>`.

Use the project-local OpenCode adapter when command, agent, prompt, and
`opencode.json` examples are useful. AiPlus CLI (`aiplus`) is the ordinary-user
compact command path.

## Commands

```bash
cd <TARGET_PROJECT>
aiplus install opencode
aiplus compact validate
aiplus compact checkpoint
```

Use OpenCode's own compact or session-management controls manually after reviewing the checkpoint output.

After compact, if OpenCode returns control automatically:

```bash
cd <TARGET_PROJECT>
aiplus compact resume
```

If OpenCode waits for the user, send a natural continuation such as `继续`,
`刷新`, `refresh`, `continue`, `resume`, `go on`, or `接着`.

## Synthetic Handoff Content

Use placeholder-only entries:

```text
Current goal: Preserve OpenCode context for <EXAMPLE_PROJECT>.
Current phase: Complete checkpoint review.
Owner gate: DENIED: <OWNER> wants one more validation pass before compacting.
Next safe action: Fix validation findings in <TARGET_PROJECT> and rerun checkpoint.
Evidence: Synthetic OpenCode usage documented with placeholders only.
```

Record any OpenCode-specific compact steps in `current-handoff.md` using placeholders rather than real local paths or private project details.
