# Changelog

## Unreleased

## 0.5.18

- **Uninstall hygiene (Track A.1)**: `aiplus uninstall --yes` now
  sweeps `.claude/agents/{aieconlab,agent-team,aiplus}-*.md`,
  `.claude/commands/{aiel,aiplus,at}-*.md`, and the matching
  `.opencode/{agents,commands,prompts}/aiplus*` mirrors. Empty
  parent dirs we created are pruned. User-authored files survive.
  / **卸载清理（A.1）**：`aiplus uninstall --yes` 现在清理三个 prefix 组
  的 `.claude/`、`.opencode/` 残留文件，并修剪空目录。用户自建文件不动。

- **Cross-team residue cleanup at install (Track A.2)**:
  `agent_team_init` / `aieconlab_init` now clear the OTHER team's
  exclusive files from `.aiplus/agents/` before writing their own.
  Snapshot mechanism captures clean per-team state; the bare-mirror
  orphans (architect.md, ceo.md, …) that A.1 documented as a known
  limit are now prevented at source.
  / **跨 team 残留清理（A.2）**：两个 init 在写自家文件前先清掉对家 exclusive
  文件。snapshot 现在只存自家干净状态。

- **AEL OpenCode adapter v0.3 (Track B.1)**: 20 prefixed subagents
  (`.opencode/agents/aieconlab-<role>.md`) + 4 slash commands
  (`.opencode/commands/aiel-*.md`). AEL module 0.2.0 → 0.3.0.
  / **AEL OpenCode 适配器 v0.3（B.1）**：20 个角色文件 + 4 个 slash 命令。

- **agent-team OpenCode adapter v0.2 (Track B.2)**: 14 prefixed
  subagents (`.opencode/agents/agent-team-<role>.md`) + 2 slash
  commands. agent-team module 0.2.0 → 0.3.0.
  / **agent-team OpenCode 适配器 v0.2（B.2）**：14 个角色文件 + 2 个 slash 命令。

- **Codex coexistence audit (Track B.3)**: regression tests lock the
  AGENTS.md / AGENTS.aiplus.md dual-team coexistence behavior so
  future changes to the section-append path can't silently break
  the codex view of either team.
  / **codex 共存审计（B.3）**：回归测试锁定 codex 视角下双 team 的可见性。

- **agent-team persona behavior suite (Track C.1)**: mirrors AEL's
  W8 suite — 8 personas × 3 cases (in_scope / boundary / stop_gate),
  Python runner using Anthropic API, dedicated workflow that skips
  on missing API key. 5 offline structural sanity tests run in
  regular CI without API credentials.
  / **agent-team persona 行为测试（C.1）**：8 角色 × 3 case 共 24 个测试。

- **Cross-runtime install matrix test (Track C.2)**: single
  end-to-end test that exercises `install all → add aieconlab →
  set-team → uninstall` across all 3 runtimes with assertions at
  every phase. Regression boundary for any change touching the
  three adapter install paths.
  / **跨 runtime 安装矩阵测试（C.2）**：4 阶段 e2e 测试覆盖 3 个 runtime 全流程。

## 0.5.17

- **agent-key OS keyring default**: agent-key now uses the OS keyring
  (macOS Keychain / Linux Secret Service / Windows Credential
  Manager) as the default backend — free, zero-config. Bitwarden
  remains an opt-in for users who prefer their existing vault.
- **Persona drift detection (P1.4, P1.6, N3)**: `aiplus doctor` now
  walks `.aiplus/agents/personas/` and compares each persona against
  same-named mirrors under `.claude/agents/` and `.opencode/agents/`.
  Name-mapping table handles the prefixed mirror filenames; trim +
  strip-frontmatter normalize the comparison so wrapped mirrors
  don't trigger false positives. New UPGRADE.md captures the
  human-facing remediation flow.
- **`is_supported_manifest_schema` accepts `0.5.*` pattern (P2.3)**:
  match-based extension replaced with a glob so future minor bumps
  don't require a per-release source edit. Coupled with the
  install.sh fallback invariant test, drift between Cargo.toml and
  supported-schema list is now impossible to merge silently.
- **Release notes from tag annotation (P2.1)**: `release.yml` now
  passes `--notes-from-tag` instead of `--generate-notes`, so the
  git tag's annotated message drives the GitHub Release body. Stops
  the "release notes are PR backlinks" antipattern.
- **Merge policy + branch protection docs (P2.4)**: CONTRIBUTING.md
  documents the squash-merge + delete-branch convention and the
  branch-protection rules that enforce CI-green-before-merge.

## 0.5.16

User-visible fixes for the agent-team + AiEconLab coexistence story that
landed in v0.5.14 / v0.5.15 but still had rough edges in real use.

- **Agent-team is now visible to Claude Code's auto-routing.** Before
  this release, `aiplus install claude-code` (or `aiplus add agent-team`
  on a Claude Code project) wrote `.claude/agents/<role>.md` files
  without YAML frontmatter, so Claude Code's auto-routing never saw the
  team — `architect`, `ceo`, `engineer-a`, `engineer-b`, `qa`, and
  `reviewer` were effectively invisible. Now ships 14 prefixed
  subagents (`agent-team-<role>.md`, 8 core + 6 functional experts)
  with proper frontmatter, plus `/at-status` and `/at-route` slash
  commands and an `AIPLUS-AGENT-TEAM` managed block in CLAUDE.md that
  coexists cleanly with the existing AEL block (#31).
  / **Agent-team 现在能被 Claude Code 自动路由识别。** 之前 14 个 SWE 角色
  没有 YAML frontmatter，Claude Code 看不到。现在每个角色文件有 `name` /
  `description`，并加上 `/at-status` 和 `/at-route` 两个 slash 命令、
  CLAUDE.md 受管块。

- **`aiplus agent status` filters by active team.** With both
  `agent-team` and `aieconlab` installed, the status command used to
  report a confused 37-role roster regardless of which team was active.
  Now `aieconlab` active shows only the 20-role AEL roster, and
  `agent-team` active shows only the SWE roster — matching every other
  command (`route`, `set-team`, `talk`) that already respected the
  active marker (#32).
  / **`aiplus agent status` 按 active team 过滤。** 之前两个模块都装时
  统一显示 37 个混合角色，现在按当前 active team 只显示对应 roster。

- **Research-paper tasks now reach the AEL consultant.** PI tasks like
  "draft scoping note", "data acquisition plan", "referee response",
  and "rebuttal letter" used to score LIGHT and silently skip the
  consultant team (LIGHT tier is consult-skip by design). Tier scoring
  now recognizes 15 research-paper compounds (scoping-note, data
  acquisition, referee, weak-instrument, paper-revision, treaty-port,
  main-spec, …) so genuinely heavy research moves engage the right
  consultant seats. Trivial work (typo fix, version bump) is unchanged
  (#33).
  / **研究类任务现在会触发 AEL consultant。** 之前 "draft scoping note"
  / "data acquisition plan" / "referee response" 都被打成 LIGHT，绕过了
  consultant team。现在 tier scoring 增加了 15 个研究类关键词组合。

- **`aiplus compact prepare` is quiet on fresh installs.** A
  just-installed project has no Owner gate decisions yet, but the seed
  compact templates ship UNKNOWN_PENDING placeholders that historically
  made `compact prepare` (and the PreCompact hook) report
  UNKNOWN_NEEDS_REVIEW on every host compact attempt. Now distinguishes
  the seed-only state and returns the informational
  `FRESH_INSTALL_AWAITING_FIRST_USE` with exit 0; any custom edit to
  the handoff or Owner Gates section moves the project back into the
  normal review loop (#34).
  / **`aiplus compact prepare` 在 fresh install 上不再吵闹。** 之前每次
  host compact 都会因 seed Owner gate 报 UNKNOWN_NEEDS_REVIEW。现在能
  分辨 "seed 状态" 与 "真正需要 review"。

- **`install.sh` offline fallback bumped to current Latest.** The
  hard-coded `VERSION=v0.5.11` fallback (used only when both `gh api`
  and `curl` for the latest release fail) was four releases stale.
  Bumped to v0.5.16, and a new integration test asserts the fallback
  tracks `aiplus-cli` Cargo.toml — future Cargo.toml bumps now require
  the install.sh bump in the same commit, preventing this drift class
  (#35).

- **Fixed RED main from v0.5.15.** Two pre-existing test failures had
  been blocking PR CI test jobs since v0.5.15: (1) the
  `is_supported_manifest_schema` match list stopped at `"0.5.14"`, so
  every fresh v0.5.15 install reported `NEEDS_FIX manifest schemaVersion
  supported` and the integration test suite was red; (2) the
  `agent_route_blocks_dispatch_on_unapproved_owner_gate` parity test
  asserted no dispatch-log entry on refusal, but P1.3 (dispatch
  outcome) changed the behavior to always log with
  `outcome="canceled"`. Both fixed (PRs #37, #46).

## 0.5.1

- Wired Agent Continuity into `aiplus refresh`, `aiplus status`, and
  `aiplus doctor` so memory, identity, Skill Candidate, profile, secret safety,
  and global config state are visible from the normal refresh path.
- Added `aiplus memory list`, `aiplus memory recent`, safer forget output, and a
  more compact `aiplus memory context` packet for runtime agents.
- Improved identity and Skill Candidate UX with `identity list`, summarized
  advisor/CEO context, explicit permission-free identity output, and guidance
  that candidates are not approved skills.
- Updated Codex, Claude Code, and OpenCode project-local guidance for natural
  phrases such as `记住这个`, `忘掉这个`, `新开顾问`, `新开 CEO`, and
  `把这次经验沉淀成 skill`.

## 0.5.0

- Added the public `aiplus-agent-memory` Agent Continuity foundation for local
  Memory Context, Role Identity, and Skill Candidate governance.
- Added `aiplus memory`, `aiplus identity`, and `aiplus skill-candidate`
  foundation commands with project-local stores under `.aiplus/`.
- Added schemas, templates, adapters, synthetic examples, fake-HOME tests,
  project isolation tests, redaction guards, and public/private asset checks.

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
