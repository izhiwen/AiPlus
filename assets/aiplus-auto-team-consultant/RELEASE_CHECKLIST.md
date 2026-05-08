# Release Checklist: AiPlus Subproduct README Alignment

Local README alignment review must pass before the Owner-approved GitHub `main` push.

## Required Files

- [x] Root README.md explains the Rust `aiplus install` path and Codex, Claude Code, and OpenCode support in 5 minutes.
- [x] Root README.md explains AiPlus as the ecosystem / CLI distribution entry and AiPlus Auto Team Consultant as an independent subproduct/module.
- [x] Root README.md documents AiPlus installer path when available, existing `aiplus` command path, and advanced module-only adoption.
- [x] README.zh-CN.md exists.
- [x] SECURITY.md exists.
- [x] LICENSE exists.
- [x] CHANGELOG.md exists.
- [x] MODULES.md exists.
- [x] `core/docs/` and `core/templates/` exist.
- [x] `core/templates/TEMPLATE_INDEX.md` exists and maps templates to role, tier, and output.
- [x] Codex adapter exists.
- [x] Claude Code adapter exists.
- [x] OpenCode adapter exists.
- [x] Synthetic examples exist for shared, Codex, Claude Code, and OpenCode.

## Local QA

- [x] Required-file check passes.
- [x] JSON manifests parse.
- [x] Private-data scan passes.
- [x] Forbidden-claim scan passes with only negative-boundary or scanner false-positive hits.
- [x] No scripts, package manifests, CLI automation, telemetry, MCP server, App connector, or registry publish flow exists.
- [x] Adapter template paths are documented and resolvable from this repo layout.
- [x] README.md and README.zh-CN.md are beginner-friendly and explain the AiPlus ecosystem path plus the independent module-only path.
- [x] Adapter READMEs are reference/source docs, not the primary user install path.

## Publication

- [x] Owner approval for GitHub `main` push is recorded in the task context.
- [x] No npm publish, registry publish, GitHub Release, git tag, marketplace submission, global install, or global config edit occurred.
- [x] GitHub repo remote is `izhiwen/aiplus-auto-team-consultant`.
- Post-push verification must confirm the latest pushed commit matches local PASS state and be reported in the final handoff.

## Out Of Scope For This Release

- [x] No release tag, GitHub Release, registry publish, marketplace submission, global install, or external integration is included in this release.
- [x] Rust-first README alignment does not add scripts, package automation, telemetry, or runtime automation scope.
