# Migration From Codex Compact Protocol

This guide is for users moving from `codex-compact-protocol` to AiPlus Auto Compact.

## What Changed

AiPlus Auto Compact keeps the compact/resume state model, but supports two
current audiences:

- AiPlus users who install the bundled Auto Compact module through `aiplus`.
- Module-only users who visit this repository directly to inspect, copy, or
  adapt the compact workflow.
- `aiplus-auto-compact` is the independent compact/checkpoint/resume workflow
  module.
- `core/scripts/compactctl.mjs` is retained only as archived legacy history and
  compatibility-test reference, not as an active compact execution path.
- Runtime adapters live under `<REPO_ROOT>/aiplus-auto-compact/adapters/`.

The core protocol still uses the compatibility state path:

```text
<TARGET_PROJECT>/.codex/compact/
```

This keeps existing handoff files and fixtures easier to review during migration.

## What Did Not Change

The manual compact boundary remains the same:

1. Update local handoff files.
2. Run validation.
3. Create a checkpoint.
4. Review Owner gates and checkpoint output.
5. Manually trigger the runtime's compact action when appropriate.
6. Resume from local files.

The tool still cannot trigger Codex UI compact, call `/compact`, or control Claude Code or OpenCode compaction.

## Path Mapping

| Previous concept | AiPlus Auto Compact location |
| --- | --- |
| Protocol templates | `<REPO_ROOT>/aiplus-auto-compact/core/templates/` |
| Protocol schemas | `<REPO_ROOT>/aiplus-auto-compact/core/schemas/` |
| Current user CLI | `aiplus compact validate`, `aiplus compact checkpoint`, `aiplus compact resume` |
| Archived legacy helper | `<REPO_ROOT>/aiplus-auto-compact/core/scripts/compactctl.mjs` retained only for history and compatibility tests |
| Target project state | `<TARGET_PROJECT>/.codex/compact/` |
| Codex-specific docs and assets | `<REPO_ROOT>/aiplus-auto-compact/adapters/codex/` |
| Claude Code command assets | `<REPO_ROOT>/aiplus-auto-compact/adapters/claude-code/` |

## Migration Steps

From the target project:

```bash
cd <TARGET_PROJECT>
aiplus compact validate
```

If validation passes, create a new checkpoint:

```bash
aiplus compact checkpoint
```

If validation reports unsupported or missing versions, review the files under `.codex/compact/` before editing. Unknown versions should be treated as review items, not automatic upgrades.

For a fresh target project:

```bash
cd <TARGET_PROJECT>
aiplus install codex
```

Use `--force` only when `<OWNER>` intends to replace existing compact files with templates.

## Codex Users

Use AiPlus CLI (`aiplus`) as the default workflow:

```bash
cd <TARGET_PROJECT>
aiplus compact validate
aiplus compact checkpoint
```

Keep Codex compact manual. After compact:

```bash
aiplus compact resume
```

If the agent continues automatically, you do not need to do anything. If it does
not reply, natural continuation messages such as `continue`, `resume`, `go on`,
`继续`, `刷新`, or `接着` should restart the resume flow.

## Claude Code Users

Load the Claude Code adapter when command documents are useful:

```bash
claude --plugin-dir <REPO_ROOT>/aiplus-auto-compact/adapters/claude-code
```

Then use the namespaced commands to guide validation, checkpoint, and resume.
When shell commands are needed, prefer `aiplus compact validate`,
`aiplus compact checkpoint`, and `aiplus compact resume`.

## OpenCode Users

Use the OpenCode adapter when project-local config, commands, agents, and prompts are useful:

```bash
cp <REPO_ROOT>/aiplus-auto-compact/adapters/opencode/opencode.json.example <TARGET_PROJECT>/.opencode/opencode.json
```

Merge rather than overwrite if the target project already has
`.opencode/opencode.json`. Use `aiplus compact ...` for structural validation,
and keep OpenCode compact/session controls manual.

## Review Checklist

- Existing `.codex/compact/` files are still relevant.
- Owner gates are explicit as `APPROVED`, `DENIED`, or `UNKNOWN_PENDING`.
- No compact file contains secrets, tokens, private paths, raw transcripts, provider payloads, account identifiers, or personal data.
- `compact-policy.json` has known `templateVersion` and `schemaVersion` values.
- `validate` has been rerun after edits.
- `checkpoint` output has been reviewed before manual compaction.
