# Migration From auto-team-consultant

Source package: `auto-team-consultant`

Target package: `aiplus-auto-team-consultant`

## Decision

The draft target directory already existed but was only partially migrated: it had copied templates and a Codex Skill, but was missing complete core docs, adapter manifests, Claude Code files, OpenCode files, runtime examples, migration notes, and GitHub readiness artifacts.

Decision: rebuild the target directory from the existing source module and keep the useful source content by copying templates into `core/templates/` and the Codex Skill into `adapters/codex/skills/auto-team-consultant/SKILL.md`.

The source directory was not deleted or modified.

## Mapping

- `auto-team-consultant/templates/*` -> `core/templates/*`
- `auto-team-consultant/skills/auto-team-consultant/SKILL.md` -> `adapters/codex/skills/auto-team-consultant/SKILL.md`
- Platform-neutral docs were rewritten into `core/docs/*`
- Runtime-specific setup moved into `adapters/codex`, `adapters/claude-code`, and `adapters/opencode`
- Synthetic examples were reorganized under runtime directories and `examples/shared-synthetic`

## Scope Boundary

This migration does not create CLI automation, package registry publishing, marketplace submission, global install, telemetry, MCP server, App connector, or real user research.
