# Synthetic Codex Project Example

This example shows how a Codex user can apply AiPlus Auto Compact to a synthetic project named `<EXAMPLE_PROJECT>`.

## Layout

```text
<TARGET_PROJECT>/
  src/
  tests/
  .codex/compact/
```

## Commands

```bash
cd <TARGET_PROJECT>
aiplus install codex
aiplus compact validate
aiplus compact checkpoint
```

If checkpoint output is acceptable after review, the user manually runs Codex compact through the Codex UI or `/compact`.

The agent can say:

```text
建议现在 compact。AiPlus checkpoint 已准备好。compact 后如果宿主继续把控制权交给我，我会自动恢复；如果工具等待你发消息，随便说“继续”“刷新”“continue”“resume”或类似意思即可。
```

After compact, if Codex returns control automatically:

```bash
cd <TARGET_PROJECT>
aiplus compact resume
```

If Codex waits for the user, send a natural continuation such as `继续`,
`刷新`, `refresh`, `continue`, `resume`, `go on`, or `接着`.

## Synthetic Handoff Content

Use placeholder-only entries:

```text
Current goal: Finish synthetic feature validation for <EXAMPLE_PROJECT>.
Current phase: Verify docs and tests.
Owner gate: APPROVED: <OWNER> approved manual compact review for synthetic example state.
Next safe action: Run validation in <TARGET_PROJECT> and inspect checkpoint output.
Evidence: npm test passed in <REPO_ROOT>/aiplus-auto-compact.
```

Do not include real private paths, account data, transcripts, screenshots, provider payloads, or secrets.
