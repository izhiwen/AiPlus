# Synthetic Codex Project Example

This example shows how a Codex user can apply AiPlus Compact Reminder to a synthetic project named `<EXAMPLE_PROJECT>`.

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
aiplus compact prepare
aiplus compact validate
aiplus compact checkpoint
```

If checkpoint output is acceptable after review, the user manually runs Codex compact through the Codex UI or `/compact`.

The agent can say:

```text
Ready to compact.

After compact:
- If I continue automatically, you do not need to do anything.
- If I do not reply, send: continue

I will resume from here.
```

After compact:

```bash
cd <TARGET_PROJECT>
aiplus compact resume
```

If Codex does not reply, send a natural continuation such as `continue`, `resume`,
`go on`, `继续`, `刷新`, or `接着`.

## Synthetic Handoff Content

Use placeholder-only entries:

```text
Current goal: Finish synthetic feature validation for <EXAMPLE_PROJECT>.
Current phase: Verify docs and tests.
Owner gate: APPROVED: <OWNER> approved manual compact review for synthetic example state.
Next safe action: Run validation in <TARGET_PROJECT> and inspect checkpoint output.
Evidence: npm test passed in <REPO_ROOT>/aiplus-compact-reminder.
```

Do not include real private paths, account data, transcripts, screenshots, provider payloads, or secrets.
