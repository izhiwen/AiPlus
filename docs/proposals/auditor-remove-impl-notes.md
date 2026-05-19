# Auditor Remove Implementation Notes

Goal: `G-AT-AUDITOR-REMOVE-1`
Worktree: `aiplus-public.auditor-remove`
Branch: `feat/auditor-remove`

## Phase 1 Design

### Grep Targets

- `auditor_provider`: finds the CLI flag, route handler parameter plumbing, provider normalization/execution helpers, dispatch-log fields, test assertions, and doctor env INFO line.
- `auditor_verdict`: finds the removed dispatch-log event name and smoke-test assertions.
- `AuditorVerdict`: expected to remain in the audit subsystem and core type re-exports; it is unrelated to the cross-provider route auditor. The final scoped grep excludes allowed audit modules if needed.
- `AIPLUS_AUDITOR_PROVIDER`: finds the doctor-only env status line that should be removed.

### Deletion List

- Delete `crates/aiplus-cli/tests/sec_1_auditor_smoke.rs` entirely.
- In `crates/aiplus-cli/src/agent/commands.rs`, delete the `--auditor-provider <PROVIDER>` field and help text from `AgentSub::Route`.
- In `crates/aiplus-cli/src/agent/mod.rs`, delete `auditor_provider` destructuring and the argument passed into `route::handle_route`.
- In `crates/aiplus-cli/src/agent/route.rs`:
  - Remove the `auditor_provider` argument from `handle_route` and `run_adaptive_route`.
  - Remove score-only and author-critic-fixer conflict checks that only exist for `--auditor-provider`.
  - Remove manual and adaptive post-dispatch `record_auditor_verdict(...)` calls.
  - Delete the cross-provider helper region: `record_auditor_verdict`, `AuditorReview`, `normalize_auditor_provider`, `detect_primary_provider`, `run_auditor_provider_review`, `auditor_prompt`, `auditor_command`, `parse_auditor_verdict`, `auditor_reasoning_summary`, and `classify_auditor_verdict`.
  - Remove any imports that become auditor-only.
- In `crates/aiplus-cli/src/agent/doctor.rs`, delete the `INFO auditor_provider_configured=...` line.

STAY files are not touched: `audit.rs`, `audit/verify_log.rs`, `identity/setup_signing.rs`, `sec_1_tamper_evident_smoke.rs`, and `sec_1_setup_signing_smoke.rs`.

### Verification Plan

- After each deletion step, run `cargo build --bin aiplus`.
- Run `cargo test -p aiplus-cli`.
- Run `cargo test --workspace`.
- Confirm `rg "auditor_provider|auditor_verdict|AIPLUS_AUDITOR_PROVIDER" crates/aiplus-cli` returns no hits.
- Confirm any remaining `AuditorVerdict` hits are only the pre-existing audit subsystem/core type, not route auditor.
- Confirm `./target/debug/aiplus agent route --help` does not show `--auditor-provider`.
- Confirm `./target/debug/aiplus doctor` does not show `auditor_provider_configured`.
- Confirm forbidden STAY files were not modified with `git diff --name-only`.

### CHANGELOG 0.6.5 Draft

- Removed the experimental cross-provider route auditor surface from v0.6.4: `aiplus agent route --auditor-provider`, the `auditor_verdict` dispatch-log event, and the doctor `auditor_provider_configured` line are gone. Tamper-evident dispatch logs and hardware-backed signing remain.

## Phase 3 Evidence

### Files Changed

- Deleted `crates/aiplus-cli/tests/sec_1_auditor_smoke.rs`.
- Removed route auditor plumbing/helpers from `crates/aiplus-cli/src/agent/route.rs`.
- Removed `--auditor-provider` CLI field from `crates/aiplus-cli/src/agent/commands.rs`.
- Removed route dispatch argument plumbing from `crates/aiplus-cli/src/agent/mod.rs`.
- Removed doctor `INFO auditor_provider_configured=...` line from `crates/aiplus-cli/src/agent/doctor.rs`.

Forbidden STAY files were not modified:

- `crates/aiplus-cli/src/agent/audit.rs`
- `crates/aiplus-cli/src/agent/audit/verify_log.rs`
- `crates/aiplus-cli/src/identity/setup_signing.rs`
- `crates/aiplus-cli/tests/sec_1_tamper_evident_smoke.rs`
- `crates/aiplus-cli/tests/sec_1_setup_signing_smoke.rs`

### Commands Run

- `cargo build --bin aiplus` after test deletion: PASS.
- `cargo build --bin aiplus` after route/mod cleanup: first build caught the expected leftover CLI enum field; fixed in the next planned step.
- `cargo build --bin aiplus` after commands cleanup: PASS.
- `cargo build --bin aiplus` after doctor cleanup: PASS.
- `cargo fmt`: PASS.
- `cargo test -p aiplus-cli`: PASS, `360 passed, 1 ignored (38 suites, 41.75s)`.
- `cargo test --workspace`: PASS, `555 passed, 1 ignored (40 suites, 44.26s)`.
- `cargo build --bin aiplus`: PASS.
- `cargo clippy -p aiplus-cli --all-targets -- -D warnings`: retry-once FAIL both runs on 10 pre-existing `aiplus-core` lint errors outside this deletion scope; no warnings/errors pointed at changed files.

### Surface Verification

- `crates/aiplus-cli/tests/sec_1_auditor_smoke.rs` no longer exists.
- `./target/debug/aiplus agent route --help | rg "auditor-provider"` returned no hits.
- `./target/debug/aiplus agent doctor | rg "auditor_provider_configured"` returned no hits.
- Scoped route-auditor grep returned no hits:
  - `rg "auditor_provider|AIPLUS_AUDITOR_PROVIDER|--auditor-provider|AIPLUS_AUDITOR_PROMPT_SCHEMA|Auditor verdict recorded" crates/aiplus-cli`
- Raw `rg "auditor_verdict" crates/aiplus-cli` still returns the pre-existing audit subsystem field in `crates/aiplus-cli/src/agent/audit/weekly_spot_check.rs`. This is not the removed cross-provider route auditor dispatch-log event and was left untouched per scope fence.
- Remaining `AuditorVerdict` hits are pre-existing audit subsystem/core type uses, not route auditor code.

### Test Count Delta

- `aiplus-cli`: `363 -> 360`, exactly minus the 3 deleted auditor smoke tests.
- Workspace: `558 -> 555`, exactly minus the same 3 deleted tests.

### Verdict

`IMPL_VERDICT=PASS_WITH_DEVIATIONS`

Deviation: the briefing's broad raw grep expectation for `auditor_verdict` conflicts with pre-existing audit subsystem terminology in `weekly_spot_check.rs`. The removed cross-provider route auditor surface is gone; the audit subsystem was intentionally preserved.
