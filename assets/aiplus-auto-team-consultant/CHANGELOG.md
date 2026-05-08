# Changelog

## Unreleased

- Synced AiPlus v0.4.3 secret-broker guidance with the expanded alias inventory
  for common AI, search, image, and developer providers.
- Clarified that `aiplus secret-broker list` owns the current alias table and
  that real Bitwarden smoke checks require the `bws` CLI plus a read-only
  machine account token.
- Added natural-language routing for private profile status and
  `aiplus secret-broker` metadata-only secret status checks.
- Clarified that task cards, result packets, review findings, pressure-tests,
  and final answers must never print, paste, log, summarize, compact, or persist
  secret values.

## v0.3.1 trust and update alignment

- Added natural-language update routing for `aiplus update all`,
  `aiplus update`, and `aiplus self update`.
- Clarified that update flows must not edit global agent config or upload
  project data.

## v0.3 compact savings alignment

- Documented compact savings requests for team/advisor workflows.
- Clarified that savings reports are estimates only, not billing data and not
  proof of review, CEO plan, or release-gate quality.

## v0.2 compact readiness alignment

- Added v0.2.1 dogfood-fix notes for legacy compact handoff migration and
  blocked-checkpoint no-write behavior.
- Ignored project-local dogfood install artifacts in this public repo.
- Added natural-language compact readiness guidance so ordinary users can say
  "prepare compact", "save progress", "continue after compact", "帮我准备
  compact", "保存进度", or "继续" instead of memorizing compact CLI commands.
- Clarified that agents should use `aiplus compact prepare` before compact and
  `aiplus compact resume` after compact as AiPlus backend tools.
- Documented role-aware compact handoff preservation for Advisor, CEO,
  Reviewer, and Builder contexts.

## AiPlus ecosystem and subproduct alignment

- Clarified AiPlus as the ecosystem / CLI distribution entry and AiPlus Auto Team Consultant as an independent subproduct/module.
- Added three user paths: AiPlus release installer when available, existing local `aiplus` command, and advanced module-only adoption.
- Explained what changes after `刷新` / `refresh` in the already-open agent session.

## Rust-first README alignment

- Rewrote README.md and README.zh-CN.md around the Rust `aiplus install` path.
- Made `aiplus install codex`, `aiplus install claude-code`, `aiplus install opencode`, and `aiplus install all` the primary user flow.
- Clarified `刷新` / `refresh` for already-open agent sessions.
- Demoted adapter internals to reference/source documentation.
- Preserved Advisor, CEO, Reviewer, Builder, LIGHT/MEDIUM/HEAVY, pressure-test, and project-local boundaries.

## v0.1.2

- Added `core/templates/TEMPLATE_INDEX.md` as a template chooser for roles, workflow tiers, and expected outputs.
- Strengthened Claude Code and OpenCode adapter README quick starts, maps, and boundaries.
- Clarified that adapter docs do not add automation, global config changes, publishing, telemetry, or external account actions.

## v0.1.1

- Repackaged AiPlus Auto Team Consultant as a cross-agent public project.
- Added shared `core/` templates and docs.
- Added Codex, Claude Code, and OpenCode adapters.
- Added runtime-specific synthetic examples.
- Preserved session-local consultant behavior and Owner-gated boundaries.
