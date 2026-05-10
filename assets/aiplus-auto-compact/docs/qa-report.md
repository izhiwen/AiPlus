# QA Report

Scope: final local validation of `aiplus-auto-compact` before the approved
AiPlus subproduct/module, installer-path, and auto-resume documentation update
is pushed to GitHub `main`.

## Status

LOCAL PASS.

Required local validation passed. GitHub remote/latest-commit verification is a
post-push step recorded in the final result packet for each approved GitHub
`main` update. No npm publish, Cargo publish, tag, GitHub Release, marketplace
submission, global install, or `$CODEX_HOME` modification was performed.

## Commands and Results

Run from this repository root unless a row explicitly names the active
`../aiplus-public` checkout.

| Check | Command | Result |
| --- | --- | --- |
| Legacy compatibility fixture | `core/scripts/compactctl.mjs` review | PASS, retained only for archived history and package acceptance tests; not an active compact execution path |
| Acceptance tests | `rtk npm test` | PASS, 15/15 tests passed |
| AiPlus CLI command truth | `rtk cargo test` from the active `../aiplus-public` checkout | PASS, 6/6 tests passed |
| AiPlus CLI help | `rtk cargo run -p aiplus-cli -- --help` from the active `../aiplus-public` checkout | PASS, shows `install`, `status`, `doctor`, `compact` |
| JSON parse | Recursive Node parser for `package.json`, `plugin.json`, `opencode.json.example`, `hooks.example.json` | PASS |
| Required-file check | Node existence check for core, Codex, Claude Code, OpenCode, and examples | PASS |
| README relative link/path sanity | Node markdown link checker across `.md` files | PASS |
| README beginner UX scan | Node checker for installer path, existing `aiplus` path, module-only path, and refresh/resume wording | PASS |
| Auto-resume wording scan | targeted `rtk rg` scan for best-effort resume wording and wake/overclaim boundaries | PASS |
| Public-safety scan | targeted `rtk rg` scans for unsafe content and overclaims | PASS with reviewed expected matches in policy/detector text only |
| High-risk token scan | targeted `rtk rg` scan for key/token/JWT/private-key shapes | PASS |
| Private-data scan | targeted `rtk rg` scan for private paths, auth headers, account/customer identifiers, and PII-like patterns | PASS with reviewed expected matches in policy/detector text only |
| Media/log/archive scan | `rtk find` for image, log, HAR, archive, audio, and video files | PASS, no matches |
| Old Codex-only branding in root/core | targeted `rtk rg` scan | PASS with caveat: `.codex/compact` compatibility paths remain by design and are documented |
| Release action scan | targeted `rtk rg` scan for publish/tag/release commands | PASS |

## Test Evidence

`rtk npm test` reported:

```text
1..15
# 15/15 compactctl acceptance tests passed
```

The passing tests covered init behavior, idempotence, valid and invalid validation cases, owner-gate handling, version review paths, secret/PII leakage warnings, resume-blocked output, parseable checkpoint JSON, and unknown-command usage output.

`rtk cargo test` in the active AiPlus CLI workspace reported 6/6 tests passed,
including Rust-native compact coverage that does not invoke Node.

## Adapter Coverage

Required files were present for:

- Shared core.
- Codex adapter.
- Claude Code adapter.
- OpenCode adapter.
- Synthetic examples for all three runtimes.

## Blockers

No local QA blocker found.

## Not Done

- No npm publish.
- No Cargo publish.
- No package registry publish.
- No GitHub Release.
- No git tag.
- No marketplace submission.
- No global install.
- No `$CODEX_HOME` modification.
- No deletion or transfer of `codex-compact-protocol`.

## Handoff

Recommended next action: push the reviewed AiPlus Auto Compact documentation
update to GitHub `main`, verify remote HEAD, and keep any future npm/Cargo/package
registry publish, tags, GitHub Releases, marketplace submissions, binary
uploads, global installs, and `$CODEX_HOME` changes behind separate Owner
approval.
