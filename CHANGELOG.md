# Changelog

## Unreleased

## 0.4.8

- Rejected empty, whitespace-only, and `PENDING_OWNER_INPUT_DO_NOT_USE`
  Bitwarden secret values as not configured.
- Preserved metadata-only output while returning
  `reason=secret_placeholder_or_empty` for placeholder or empty requested
  aliases.
- Kept unrequested placeholder aliases from blocking best-effort
  `secret-broker run -- <command...>` and selective runs for valid aliases.

## 0.4.7

- Added selective `secret-broker run` injection with `--aliases a,b` and
  repeated `--alias a`, so requested provider keys can be injected without
  unrelated placeholder providers blocking the command.
- Changed bare `secret-broker run -- <command...>` to best-effort compatibility
  behavior: inject aliases that resolve, report skipped aliases as metadata, and
  avoid printing secret values.
- Added first-class Kimi metadata that treats `kimi` as Kimi Code membership
  (`https://api.kimi.com/coding/v1`, model `kimi-for-coding`) while documenting
  Kimi Open Platform / Moonshot as a separate key system.

## 0.4.6

- Fixed real Bitwarden `secret-broker resolve` by resolving an alias key/name to
  a Bitwarden secret ID in memory before calling `bws secret get`.
- Added safe resolver metadata output (`secret_key`, `secret_id_found`) without
  printing secret IDs or secret values.
- Kept secret values out of logs, docs, tests, and default command output while
  preserving `secret-broker run -- <command...>` as the explicit env-injection
  path.

## 0.4.5

- Added `aiplus profile migrate` and `aiplus profile cleanup` so legacy
  `work-with-zhiwen` user-level profile registrations can be backed up and
  removed after the canonical `aiplus-work-with-zhiwen` profile is installed.
- Updated `aiplus profile status` to report only active canonical profiles in
  `profiles=[...]` while listing legacy registrations separately with the cleanup
  next step.
- Clarified `aiplus secret-broker doctor` output when `bws` is installed but the
  Bitwarden token is not configured.

## 0.4.4

- Changed private profile installation to a generic source-based flow so public
  AiPlus no longer embeds private profile content or private Bitwarden alias
  namespaces.
- Moved private secret alias inventory to user-installed profile packages.
- Added `aiplus profile uninstall` for reversible user-level profile removal.

## 0.4.3

- Added private-profile installed alias support for `aiplus secret-broker`.
- Added test coverage that installed aliases appear in `aiplus secret-broker
  list`, resolve without printing secret values by default, and unknown aliases
  remain blocked.
- Clarified that real Bitwarden smoke checks require the Bitwarden Secrets
  Manager `bws` CLI plus a private read-only machine account token.
- Kept secret values out of normal `list`, `status`, and default `resolve`
  output. `run -- <command...>` remains the explicit runtime-only injection path.

## 0.4.2

- Added user-level private profile commands for collaboration preferences under
  `~/.config/aiplus/`.
- Added `aiplus secret-broker` with mock and Bitwarden `bws` provider paths,
  approved alias mapping, metadata-only status/list/resolve output, and
  child-process environment injection via `run -- <command...>`.
- Added explicit warnings that `secret-broker run` only keeps AiPlus from
  printing or persisting secrets; the invoked child command can still print, log,
  transmit, or store environment variables.
- Updated installed agent guidance for natural-language profile and secret
  status triggers while keeping secret values out of chat, logs, compact files,
  repos, and release artifacts.
- Preserved v0.3.1 compact savings and update semantics.

## 0.3.1

- Fixed Compact Savings all-time totals so projected `prepare` and candidate
  `checkpoint` events do not count as completed savings.
- Defined compact savings event semantics: `prepare=projected`,
  `checkpoint=candidate`, and successful `resume=completed`.
- Deduplicated completed compact cycles by `checkpointId`, so repeated resume
  does not double-count the same compact cycle.
- Added `aiplus self update` for checksum-verified user-level CLI updates with
  dry-run, backup, staged replacement, and smoke-check output.
- Added `aiplus update all` to update the CLI and current project guidance in
  one command when safe.
- Clarified pricing update/status output with `pricing_fetch_mode`,
  `pricing_source`, cache age, `billing_data=no`, and `uploads=none`.
- Added natural-language update guidance for "update AiPlus", "升级 AiPlus",
  "update the aiplus command", and project-only update requests.

## 0.3.0

- Added Compact Savings Estimate with project-local
  `.codex/compact/savings-ledger.jsonl` aggregate events.
- Added `aiplus compact savings` and `aiplus compact savings --json`.
- Added `aiplus pricing status` and `aiplus pricing update`; savings reports
  read cached pricing by default, while explicit pricing update fetches public
  pricing data.
- Added conservative local token savings, weighted reduction percentage, and
  estimated USD savings reporting. Reports are estimates only, not billing data.
- Added safe unknown-model behavior: token savings and reduction still report,
  while USD savings become unavailable or partial when pricing is missing.
- Documented that AiPlus does not upload prompts, project files, checkpoints,
  savings ledgers, secrets, billing data, or usage history.

## 0.2.1

- Fixed dogfood upgrade behavior for legacy compact handoffs by adding missing
  v0.2 role-aware sections during install/update while preserving existing
  handoff content and backing up the original file.
- Changed blocked compact checkpoint behavior so `BLOCKED_BY_OWNER_GATE` does
  not create a normal checkpoint JSON by default.
- Added public repo hygiene ignores for project-local dogfood install artifacts
  such as `.aiplus/`, `.codex/`, `.claude/`, `.opencode/`, and generated
  `AGENTS.md`.
- Added v0.2 Compact Readiness & Recovery:
  `aiplus compact prepare`, readiness states, `aiplus compact score`,
  `checkpoint --level light|standard|full`, and role-aware resume guidance.
- Made natural language the primary compact interface for ordinary users:
  "prepare compact", "save progress", "continue", "帮我准备 compact", "保存进度",
  and "继续" map to agent use of AiPlus backend commands.
- Documented that compact CLI commands are agent backend tools, advanced manual
  fallbacks, and maintainer debugging commands, not beginner memorization
  requirements.
- Removed active Node `compactctl.mjs` guidance from installed and
  ordinary-user compact paths.
- Made Rust-native `aiplus compact prepare`, `score`, `checkpoint`, `validate`,
  and `resume` the only supported compact execution commands.
- Added missing-`aiplus` guidance: install AiPlus or fix PATH instead of falling
  back to Node.
- Updated bundled Auto Compact docs so legacy Node references are archived
  history or compatibility-test fixtures only.

## 0.1.2

- Added explicit AiPlus refresh triggers for already-open sessions:
  `AiPlus 刷新`, `刷新 AiPlus`, `aiplus refresh`, `aiplus status`,
  `AiPlus status`, `继续 AiPlus`, and `resume AiPlus`.
- Added `aiplus refresh` as a concise helper command for agents and users.
- Strengthened installed `.aiplus/AGENTS.aiplus.md` guidance so AiPlus status is
  reported before unrelated project refresh when the user asks for AiPlus.
- Documented project-specific refresh conflict handling while preserving generic
  `刷新` / `refresh` as AiPlus-first after installation.

## 0.1.1

- Fixed existing-project `aiplus install codex` upgrades so old AiPlus managed
  files are backed up and refreshed without requiring ordinary users to know
  `--force --backup --yes`.
- Preserved existing `.codex/compact/` state during install/upgrade.
- Updated generated refresh guidance so `刷新` and `refresh` are treated as
  AiPlus refresh first, with a concise installed-status response.
- Refined Auto Compact checkpoint/resume and Auto Team Consultant activation
  guidance in generated project instructions and bundled module docs.
- Kept the v0.1.1 installer on the verified macOS Apple Silicon release asset
  path with checksum verification and user-level `~/.local/bin/aiplus` install.

## 0.1.0

- Published v0.1.0 as the first practical binary-installed AiPlus CLI release.
- Added `install.sh` for checksum-verified user-level install to
  `~/.local/bin/aiplus`.
- Documented best-effort automatic compact resume behavior and natural
  continuation phrases.
- Rewrote public README and README.zh-CN beginner flow to use copy-pasteable
  installer commands instead of source-build placeholders.
- Standardized human-facing product naming to `AiPlus` while keeping the
  command, binary, repo, and crate identifiers as `aiplus`/`aiplus-cli`.
- Added an Owner-approved v0.1.0 GitHub Release path with a verified macOS Apple
  Silicon binary, `checksums.txt`, and checksum-verifying install script.
- Added Rust-first `aiplus` CLI workspace.
- Added local vendored AiPlus module asset snapshot.
- Added project-local install/update/add/status/doctor/uninstall workflows.
- Added Codex, Claude Code, OpenCode, and all-runtimes adapter support.
- Added Rust parity and safety tests.
- Documented Node CLI as archived historical reference.
- Replaced compact bridge limitation with Rust-native compact status
  `COMPACT_RUST_NATIVE_STATUS=PASS`.

## Public-ready candidate docs

- Documented recommended public repo name `aiplus`.
- Documented public repo structure with Rust workspace as root.
- Added v0.1.0 distribution plan.
- Added binary artifact matrix with macOS Apple Silicon verified first and other
  platforms planned.
- Added migration guide from archived Node CLI.
- Added QA release-readiness checklist.
- Kept installed manifest schema `0.2.1` for compatibility.
- Applied Owner-approved Apache-2.0 licensing to the Rust mainline/public-ready
  package metadata and docs.
