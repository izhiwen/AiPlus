# Changelog

All notable public documentation changes for AiPlus Compact Reminder are recorded here.

## Unreleased

- Added `aiplus compact remind` guidance for proactive compact reminders,
  reminder levels, snooze, handoff freshness, recovery confidence, and local
  token/USD savings preview.
- Added `aiplus-module.json` for AiPlus Rust module-system manifest validation.
- Synced AiPlus v0.4.3 secret-broker guidance with the expanded alias inventory
  for common AI, search, image, and developer providers.
- Clarified that `aiplus secret-broker list` owns the current alias table and
  that real Bitwarden smoke checks require the `bws` CLI plus a read-only
  machine account token.
- Added guidance for user-level private profiles and `aiplus secret-broker`
  metadata-only checks.
- Clarified that compact handoffs, checkpoints, and savings ledgers must never
  store secret values, Bitwarden tokens, auth headers, provider response bodies,
  or raw profile-private material.
- Added v0.3.1 update guidance for `aiplus self update`, `aiplus update`, and
  `aiplus update all`.
- Clarified compact savings event semantics: prepare is projected, checkpoint is
  candidate, and resume completes one counted compact cycle per checkpoint.
- Added AiPlus v0.3 Compact Savings Estimate guidance for natural-language
  savings requests.
- Clarified cache-first pricing behavior, weighted all-time reduction, unknown
  model USD-unavailable behavior, and estimate-only/not-billing-data wording.
- Added security wording that savings ledgers must remain aggregate and must not
  include prompts, project files, raw checkpoint text, billing data, or usage
  history.
- Added v0.2.1 dogfood-fix guidance for legacy compact handoff migration:
  install/update should back up old handoffs, preserve user content, and add
  missing `Session Role`, `Workflow Level`, and `Output Contract` sections.
- Documented blocked checkpoint no-write behavior:
  `BLOCKED_BY_OWNER_GATE` must not create a normal checkpoint file by default.
- Ignored project-local dogfood install artifacts in this public repo.
- Added v0.2 Compact Readiness & Recovery guidance with `aiplus compact prepare`,
  readiness states, `aiplus compact score`, checkpoint levels, and role-aware
  resume context.
- Made natural language the primary interface for ordinary users: "prepare
  compact", "save progress", "continue", "帮我准备 compact", "保存进度", and
  "继续" map to agent use of AiPlus backend commands.
- Clarified that compact CLI commands are backend tools for agents, advanced
  manual fallbacks, and maintainer debugging commands, not beginner memorization
  requirements.
- Removed active Node `compactctl.mjs` command guidance from ordinary-user and
  module-only compact paths.
- Clarified that Rust-native `aiplus compact prepare`, `score`, `validate`,
  `checkpoint`, and `resume` are the only supported active compact execution
  commands.
- Added missing-`aiplus` guidance to install AiPlus or fix PATH instead of
  falling back to Node.
- Reworked `README.md` for AiPlus CLI (`aiplus`) first ordinary-user guidance.
- Added `README.zh-CN.md` with matching beginner flow.
- Clarified naming: `AiPlus` is the product/module brand, while `aiplus` is
  reserved for CLI commands, repo names, paths, and code identifiers.
- Clarified that AiPlus Compact Reminder is both an independent AiPlus subproduct
  and a bundled module installed by the AiPlus CLI.
- Added future AiPlus release installer guidance, existing `aiplus` command
  guidance, module-only adoption guidance, and best-effort auto-resume wording.
- Repositioned `compactctl.mjs` as a legacy standalone helper rather than the
  ordinary-user path.
- Updated adapter and synthetic example docs to use `aiplus install ...` and
  `aiplus compact ...` commands.
- Added root public README with Codex, Claude Code, and OpenCode quick starts.
- Added shared core versus adapter explanation.
- Added runtime comparison table for recommended v0.2 usage.
- Added safety, privacy, validation, contribution, and release-status guidance.
- Added migration notes from `codex-compact-protocol`.
- Added synthetic examples for Codex, Claude Code, and OpenCode projects.
- Added release checklist, security policy, and module map.

## 0.1.0

- Initial private workspace package with shared core protocol files, Codex adapter assets, Claude Code adapter assets, and acceptance tests.
