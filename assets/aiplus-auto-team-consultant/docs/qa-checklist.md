# QA Checklist

This is a reusable maintainer checklist template. Current review status is recorded in `docs/qa-report.md` and `RELEASE_CHECKLIST.md`.

Before a publication push:

- [ ] README explains AiPlus as the ecosystem / CLI distribution entry and AiPlus Auto Team Consultant as an independent subproduct/module.
- [ ] README documents AiPlus installer path when available, existing local `aiplus` command path, and advanced module-only adoption.
- [ ] Required files exist for core and all adapters.
- [ ] JSON manifests parse.
- [ ] Forbidden claim scan has no public overclaims.
- [ ] Private data scan has no real private data.
- [ ] No scripts, package manifests, or CLI automation were added.
- [ ] README covers Codex, Claude Code, and OpenCode.
- [ ] `core/templates/TEMPLATE_INDEX.md` is discoverable from README.md and README.zh-CN.md.
- [ ] Claude Code adapter README has a quick start, command map, agent map, Owner gates, and no-global-config boundary.
- [ ] OpenCode adapter README has a quick start, project-local setup pattern, command map, agent map, prompt map, Owner gates, and no-global-config boundary.
- [ ] Docs polish did not add scripts, package manifests, CLI automation, telemetry, or runtime automation scope.
- [ ] GitHub remote points to `izhiwen/aiplus-auto-team-consultant`.
- [ ] Latest pushed commit matches local PASS state.
