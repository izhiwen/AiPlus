# Release Checklist

Use this checklist before `<OWNER>` approves any public release, tag, package publish, adapter distribution, or repository announcement.

## Status

- Current repository: public GitHub module source.
- Current `package.json` version: `0.1.0`.
- Documentation target: AiPlus CLI (`aiplus`) first compact module docs.
- Owner gate required for tags, GitHub Releases, package publication, binary
  uploads, marketplace submission, or installer publication: yes.

## Pre-Release Checks

- Confirm `README.md` uses only placeholders for paths, owners, and examples.
- Confirm `README.md` and `README.zh-CN.md` use only the AiPlus CLI (`aiplus`)
  for active compact execution and do not present `compactctl.mjs` as fallback.
- Confirm `README.md` and `README.zh-CN.md` explain both identities:
  independent AiPlus subproduct and bundled AiPlus CLI module.
- Confirm installer docs mark the `install.sh` flow as future/planned until a
  reviewed AiPlus release installer is live.
- Confirm compact docs describe best-effort automatic resume without claiming
  Compact Reminder can wake a host runtime.
- Confirm `SECURITY.md` describes local storage, reporting, redaction, and validation limits.
- Confirm `CHANGELOG.md` includes the release entry.
- Confirm `MODULES.md` reflects the current shared core and adapters.
- Confirm `docs/migration-from-codex-compact-protocol.md` is accurate for existing users.
- Confirm examples under `examples/` are synthetic only.
- Confirm docs do not claim automatic compact control for Codex, Claude Code, or OpenCode.
- Confirm no public docs contain secrets, private paths, raw transcripts, provider payloads, account identifiers, or personal data.

## Validation Commands

```bash
cd <REPO_ROOT>/aiplus-compact-reminder
npm test
```

Optional JSON checks for Claude Code adapter files:

```bash
cd <REPO_ROOT>/aiplus-compact-reminder/adapters/claude-code
jq empty .claude-plugin/plugin.json
jq empty hooks/hooks.example.json
```

Optional package metadata check:

```bash
cd <REPO_ROOT>/aiplus-compact-reminder
npm pack --dry-run
```

Do not publish from the dry run. Use it only to inspect package contents.

## Documentation Review

- The README can be followed in about five minutes by a user with `aiplus`
  available on `PATH`.
- The README supports users who want only AiPlus Compact Reminder without implying
  this repository is the full AiPlus CLI repo.
- The runtime choice table has exactly these columns: Runtime, Install command,
  Auto compact support, Recommended use.
- Migration notes explain what changed from `codex-compact-protocol`.
- Legacy Node helper references are clearly marked advanced, compatibility, or
  migration-only.
- Security docs explain what validation can and cannot prove.
- Release notes state whether the package is private, pre-release, or approved for publication.

## Owner Gate

Before publication, `<OWNER>` must approve:

- Version number.
- Release notes.
- Package contents.
- Adapter distribution scope.
- Whether checkpoint examples should remain docs-only or be included in packaged artifacts.

Record the approval in the private release record. Do not place private approval transcripts in public docs.
