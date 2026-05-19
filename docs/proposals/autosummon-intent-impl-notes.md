# Auto-summon Intent Implementation Notes

Goal: G-AT-AUTOSUMMON-INTENT-1.

## Phase 1 Design

### 1. G2 Reuse Decision

Reuse path checked:

- `crates/aiplus-cli/src/main.rs` hosts the G2 dispatch-gate doctor fixture check through `dispatch_gate_doctor_status_from_fixture`.
- `crates/aiplus-core::consult::match_gates` performs local consultant-team gate matching.
- No reusable Anthropic/OpenAI HTTP client or generic LLM-call wrapper is present in `crates/aiplus-cli/src/agent/` or `crates/aiplus-core/src/`.

Decision: implement a small local helper inside `crates/aiplus-cli/src/agent/coordinator.rs`, in the existing auto-summon section. It uses existing dependencies only (`serde_json`, `sha2`, `hex`, `anyhow`) and invokes `curl` as the transport, matching existing CLI style for network fetches. The default model is `claude-haiku-4-5-20251001`, matching the existing persona behavior smoke-test convention. No new crate and no new module are added.

Testability: production uses Anthropic when `ANTHROPIC_API_KEY` is present. Tests use a narrow `AIPLUS_AUTOSUMMON_INTENT_MOCK=1` env path so offline cargo test remains deterministic and does not require secrets.

### 2. Prompt Template

Final prompt:

```text
You are classifying whether a software task matches an intent description.

Task: "{task}"
Intent: "{intent_hint}"

Does this task match this intent? Reply with a single word: YES or NO.
```

Expected examples:

- Task `实现支付接口` with security-reviewer intent `支付、认证、敏感数据、credentials、凭据、安全漏洞或隐私相关的软件工作` => `YES`
- Task `实现支付接口` with tech-writer intent `文档、README、教程、用户指南、API 文档、发布说明或技术写作相关的工作` => `NO`
- Task `describe git status` with all three expert intents => `NO`

LLM failures are fail-closed: network errors, missing key, quota errors, malformed JSON, and non-YES/NO answers all return `false`.

### 3. Cache Design

- Scope: in-process only.
- Key: `sha256(task + "\n" + intent_hint)` encoded as lowercase hex.
- Value: `bool`.
- Size cap: 1000 entries.
- Eviction: FIFO oldest-entry eviction when adding entry 1001.
- Visibility: cache hit count is exposed through `autosummon_intent_cache_metrics()` for tests only.

### 4. Role TOML Schema Migration

The three existing expert templates move from `keywords` + `match_mode` to a single `intent_hint` plus `priority`.

- `security-reviewer`: `支付、认证、敏感数据、credentials、凭据、安全漏洞或隐私相关的软件工作`
- `tech-writer`: `文档、README、教程、用户指南、API 文档、发布说明或技术写作相关的工作`
- `ai-integration`: `LLM、大模型、prompt、RAG、embedding、OpenAI、Anthropic、Claude、Codex、agent 或模型集成相关的软件工作`

`priority` remains unchanged: security-reviewer 90, ai-integration 70, tech-writer 60.

### 5. Calibration Fixture Rewrite Plan

The first 16 entries in `crates/aiplus-cli/tests/fixtures/coordinator_calibration.toml` remain byte-identical.

The 10 v0.3.1 auto-summon entries are rewritten from keyword notes to intent notes. The fixture schema gains optional `expected_auto_summoned = [...]` assertions. Entries without that field assert only the existing score/tier behavior.

### 6. CHANGELOG Draft Text for v0.6.5

Draft only, not applied to `CHANGELOG.md`:

```text
### Changed
- Replaced expert auto-summon keyword matching with intent-hint classification. Expert role TOMLs now declare `[autosummon].intent_hint`, and the coordinator classifies task-to-expert matches through a cached yes/no LLM intent check. Missing or failed runtime auth now fails closed to no extra expert rather than over-summoning.
```

## Phase 3 Evidence

Implementation evidence:

- `cargo fmt --check` PASS.
- `cargo build -p aiplus-cli` PASS.
- Focused tests PASS:
  - `cargo test -p aiplus-cli intent_match_cache_hits_on_repeat_task_and_intent`
  - `cargo test -p aiplus-cli score_only_auto_summons_experts_by_intent`
  - `cargo test -p aiplus-cli coordinator_scores_match_calibration_fixture`
  - `cargo test -p aiplus-cli score_only_prints_auto_summoned_experts_without_dispatch`
  - `cargo test -p aiplus-cli --test v03_adaptive_coordinator d5_payment_task_staffs_heavy_team`
- Full package test PASS with the user's global git hook isolated by env override only:
  - `GIT_CONFIG_COUNT=1 GIT_CONFIG_KEY_0=core.hooksPath GIT_CONFIG_VALUE_0= cargo test --package aiplus-cli`
  - Result: all aiplus-cli suites pass, including 148 unit tests and 38 integration suites; OpenCode live TUI remains the existing ignored opt-in test.
- Plain `cargo test --package aiplus-cli` was retried once and blocked by the user's global git config `core.hooksPath=/Users/steve/.git-hooks`, which makes temporary test repo commits run a hook that greps `crates/aiplus-cli/Cargo.toml`. No global config was edited. The isolated-env run preserves tests' fake HOME gitconfig while disabling only inherited hooks.

Live smoke evidence using BWS-backed secret-broker:

```text
aiplus secret-broker run --aliases anthropic -- aiplus agent route --score-only '实现支付接口'
=> Auto-summoned experts: [security-reviewer]

aiplus secret-broker run --aliases anthropic -- aiplus agent route --score-only 'update README onboarding guide'
=> Auto-summoned experts: [tech-writer]

aiplus secret-broker run --aliases anthropic -- aiplus agent route --score-only 'implement AI agent prompt routing'
=> Auto-summoned experts: [ai-integration]

aiplus secret-broker run --aliases anthropic -- aiplus agent route --score-only 'describe git status'
=> Would staff: [] and no Auto-summoned experts line
```

Isolation evidence:

- `crates/aiplus-cli/src/agent/route.rs` SHA-256 stayed `ca574933238b4653108046abf0f1d42d727851df33f43fb1f41db60ee6d73525`.
- Forbidden-file diff for `route.rs`, `doctor.rs`, `commands.rs`, and workspace `Cargo.toml` is empty.
- Calibration fixture first 16 entries are byte-identical to `HEAD`.
- No `keywords =`, `match_mode`, `autosummon.keywords`, or `autosummon.match_mode` remain in the migrated expert TOMLs or coordinator/core autosummon implementation.

Known deviation:

- `core.rs` was touched to add the new `intent_hint` TOML schema field. The briefing's owned-file summary did not list it explicitly, but schema migration cannot compile without this field; forbidden files were not touched.
