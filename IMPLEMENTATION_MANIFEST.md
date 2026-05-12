# AiPlus Agent Team v0.1 — Schema-to-Implementation Manifest

**Frozen Schema**: `.aiplus/agent-team/acceptance/v0.1.4/schema.yaml` (SHA256: `06ee2b35466f6bd2019dbed3bf70384f98428f5eacd6cc117ba2e74fcaf5b526`)  
**Target Workspace**: `/Users/steve/Dropbox/Project/AiPlus/aiplus-public/`  
**Status**: Planning document — no code changes yet  
**Created**: 2026-05-11  

---

## 1. Deliverable-to-Code Mapping

The schema defines **7 deliverables** (§deliverables). Each maps to:
- An **audit script** (generated, `.sh`)
- A **self-test script** (`.test.sh` with ≥1 pass + ≥4 fail fixtures)
- **Runtime checks** inside the Auditor
- **CLI commands** to invoke the audit

| # | Deliverable ID | Acceptance Mode | Audit Script | Self-Test | Runtime Verdict Fn | CLI Invocation |
|---|----------------|-----------------|--------------|-----------|-------------------|----------------|
| 1 | `v0.1-worktree-provisioning` | deterministic | `audit-scripts/v0.1-worktree-provisioning.sh` | `.test.sh` | `audit::run::check_worktree_provisioning()` | `aiplus agent audit run --deliverable v0.1-worktree-provisioning` |
| 2 | `v0.1-warm-bench-cache` | deterministic | `audit-scripts/v0.1-warm-bench-cache.sh` | `.test.sh` | `audit::run::check_warm_bench_cache()` | `aiplus agent audit run --deliverable v0.1-warm-bench-cache` |
| 3 | `v0.1-readme-pain-distinctness` | deterministic | `audit-scripts/v0.1-readme-pain-distinctness.sh` | `.test.sh` | `audit::run::check_readme_pain_distinctness()` | `aiplus agent audit run --deliverable v0.1-readme-pain-distinctness` |
| 4 | `v0.1-design-clarity-decisions` | deterministic | `audit-scripts/v0.1-design-clarity-decisions.sh` | `.test.sh` | `audit::run::check_design_clarity_decisions()` | `aiplus agent audit run --deliverable v0.1-design-clarity-decisions` |
| 5 | `v0.1-parity-tests-pass` | deterministic | `audit-scripts/v0.1-parity-tests-pass.sh` | `.test.sh` | `audit::run::check_parity_tests_pass()` | `aiplus agent audit run --deliverable v0.1-parity-tests-pass` |
| 6 | `v0.1-owner-memory-untouched` | deterministic | `audit-scripts/v0.1-owner-memory-untouched.sh` | `.test.sh` | `audit::run::check_owner_memory_untouched()` | `aiplus agent audit run --deliverable v0.1-owner-memory-untouched` |
| 7 | `v0.1-stub-not-invitable-error-format` | deterministic | `audit-scripts/v0.1-stub-not-invitable-error-format.sh` | `.test.sh` | `audit::run::check_stub_not_invitable_error_format()` | `aiplus agent audit run --deliverable v0.1-stub-not-invitable-error-format` |

### 1.1 Check Execution Flow

```
aiplus agent audit run
  └─> audit::commands::AuditArgs::Run { deliverable, mode }
        └─> audit::handlers::handle_audit_run(deliverable, mode)
              ├─> Phase 1: Pre-audit gate (manifest + GPG + sentinel)
              │     └─> audit::gate::pre_audit_check()
              ├─> Phase 2: Load schema + resolve variables
              │     └─> audit::schema::load_schema(project_root)
              ├─> Phase 3: Execute checks per deliverable
              │     └─> audit::run::execute_checks(deliverable)
              │           ├─> check_kind::exit_code(cmd, expected_exit, timeout)
              │           ├─> check_kind::file_exists(path)
              │           └─> check_kind::shell_output_match(cmd, expected_regex, timeout)
              ├─> Phase 4: Combine verdicts (all_must_pass / any_pass)
              │     └─> audit::verdict::combine(checks, combiner)
              ├─> Phase 5: Build AuditReport
              │     └─> audit::report::AuditReport::new(...)
              └─> Phase 6: Persist + canary bookkeeping
                    └─> audit::persistence::write_audit_trail(report)
```

### 1.2 CheckKind Implementations

| Check Kind | Rust Function | Schema § | Notes |
|------------|--------------|----------|-------|
| `exit_code` | `audit::checks::check_exit_code(cmd, expected, timeout)` | checks[].kind | Uses `std::process::Command` with timeout via `timeout` bin_alias |
| `file_exists` | `audit::checks::check_file_exists(path)` | checks[].kind | Resolves `{variables}` then `Path::exists()` |
| `shell_output_match` | `audit::checks::check_shell_output_match(cmd, regex, timeout)` | checks[].kind | Runs cmd, applies `regex` crate to stdout |

**Forbidden (schema guarantee, enforced at parse time):**
- `ask_user` → parse error in `audit::schema::validate_checks()`
- Raw LLM judgment → blocked by deterministic-only mode enforcement
- Arbitrary shell primitives not in `bin_aliases` → flagged in `audit::schema::validate_bin_aliases()`

---

## 2. Auditor Infrastructure Map (Phases 3–13)

The Auditor is **not** a single function. It is a **13-phase pipeline** with explicit gate logic between each phase.

### 2.1 Phase Breakdown

| Phase | Name | Rust Module | Function | Gate Logic | Failure → |
|-------|------|-------------|----------|------------|-----------|
| 1 | **Sentinel Verification** | `audit::gate` | `gate::verify_sentinel()` | `owner_setup_sentinel_path` exists + valid YAML + non-empty name/email | `BLOCKED::ownership_unverified` |
| 2 | **Gitignore Check** | `audit::gate` | `gate::verify_sentinel_gitignored()` | `sentinel_path` in `.gitignore` | `BLOCKED::sentinel_in_git` |
| 3 | **Schema Load** | `audit::schema` | `schema::load_schema(path)` | Parse `schema.yaml` with `serde_yaml_ng` | `BLOCKED::schema_tampered` |
| 4 | **Variable Resolution** | `audit::schema` | `schema::resolve_variables(ctx)` | Expand `{project_name}`, `{agent_team_root}`, etc. | `BLOCKED::schema_tampered` |
| 5 | **Bin Alias Resolution** | `audit::schema` | `schema::resolve_bin_aliases(os)` | Pick linux/macOS variant per `std::env::consts::OS` | `BLOCKED::schema_tampered` |
| 6 | **Manifest Pre-check** | `audit::gate` | `gate::verify_manifest_clean()` | `git status --porcelain {release_manifest_path}` empty | `BLOCKED::manifest_dirty` |
| 7 | **GPG Signature Check** | `audit::gate` | `gate::verify_manifest_signature()` | `git log -1 --format='%G? %GF'` → `'G'` + fingerprint match | `BLOCKED::manifest_unsigned_or_wrong_key` |
| 8 | **Hash Chain Verification** | `audit::gate` | `gate::verify_hash_chain()` | Re-hash bin_aliases + each file list entry, compare to manifest | `BLOCKED::schema_tampered` |
| 9 | **Deliverable Execution** | `audit::run` | `run::execute_deliverable(d)` | Run checks per deliverable's `check_combiner` | Verdict per deliverable |
| 10 | **Pass Agreement** | `audit::verdict` | `verdict::require_pass_agreement(runs)` | `audit_passes=2`, `audit_passes_must_agree=true` | `BLOCKED::auditor_unavailable` if disagree |
| 11 | **Canary Trigger** | `audit::canary` | `canary::maybe_trigger(state)` | Every 7th run + risk-weighted sample | Enqueued for replay |
| 12 | **Report Generation** | `audit::report` | `report::build_output()` | Structured YAML per `auditor_output` contract | N/A |
| 13 | **Trail Persistence** | `audit::persistence` | `persistence::write_trail()` | Append to `{audit_trail_dir}/<run_id>.yaml` | N/A |

### 2.2 Gate Failure → BLOCKED Reason Mapping

| BLOCKED Reason | Trigger Phase | Schema § | CLI Exit Code |
|----------------|--------------|----------|---------------|
| `schema_tampered` | 3, 4, 5, 8 | auditor_output.blocked_deliverables | 2 |
| `auditor_unavailable` | 10 | auditor_output.blocked_deliverables | 2 |
| `ceo_override_attempted` | (enforced by GPG) | auditor_output.blocked_deliverables | 2 |
| `manifest_unsigned_or_wrong_key` | 7 | auditor_output.blocked_deliverables | 2 |
| `manifest_dirty` | 6 | auditor_output.blocked_deliverables | 2 |
| `hash_via_llm` | (never — deterministic only) | auditor_output.blocked_deliverables | 2 |
| `ownership_unverified` | 1 | failure_modes.ownership_unverified | **3** (user-actionable) |
| `sentinel_in_git` | 2 | failure_modes.sentinel_in_git_history | 2 |

### 2.3 Canary Replay Subsystem

```
aiplus agent audit canary
  └─> audit::commands::AuditArgs::Canary
        └─> audit::handlers::handle_canary()
              ├─> canary::check_freshness(last_audit_max_age_days=14)
              │     └─> If stale → surface message, skip canary
              ├─> canary::select_samples(strategy=risk_weighted)
              │     ├─> Priority: stop_gate_touched > HEAVY > MEDIUM > LIGHT
              │     ├─> HEAVY: 3 samples, MEDIUM: 2, LIGHT: round_robin
              │     └─> Cap: 8, Floor: 3
              ├─> canary::replay_each(sample)
              │     └─> `aiplus agent audit replay <audit_run_id>`
              ├─> canary::detect_drift(original, replay)
              │     └─> Different verdict on same state → overclaim detected
              └─> canary::handle_overclaim()
                    ├─> Enqueue to owner_spot_check.queue (PRIORITY=HIGH)
                    └─> Await `aiplus agent audit owner-feedback ...`
```

### 2.4 Owner Spot-Check Subsystem

```
aiplus agent audit owner-feedback <run_id> --actual-verdict PASS|FAIL --note "..."
  └─> audit::commands::AuditArgs::OwnerFeedback { run_id, actual_verdict, note }
        └─> audit::handlers::handle_owner_feedback()
              ├─> Load audit trail for run_id
              ├─> Compare actual_verdict vs auditor_verdict
              ├─> Append to {owner_spot_check_queue} (jsonl)
              └─> Recalculate 30-day metrics window

aiplus agent audit owner-feedback-retract <run_id>
  └─> audit::commands::AuditArgs::OwnerFeedbackRetract { run_id }
        └─> audit::handlers::handle_owner_feedback_retract()
              └─> Mark entry retracted in queue; recalculate metrics
```

### 2.5 GPG / Sentinel / Setup-GPG Subsystem

```
aiplus agent audit setup-gpg
  └─> audit::commands::AuditArgs::SetupGpg
        └─> audit::handlers::handle_setup_gpg()
              ├─> Phase A: Sentinel verification
              │     ├─> Read {owner_setup_sentinel_path}
              │     ├─> Validate YAML (name + email required, non-empty)
              │     └─> If fail → print refusal_message, exit 3
              ├─> Phase B: GPG batch config generation
              │     ├─> Try EDDSA + Curve25519 (gpg --version >= 2.1)
              │     ├─> Fallback to RSA-4096 if unsupported (notify user)
              │     └─> NEVER use %no-protection
              ├─> Phase C: Interactive passphrase wizard
              │     ├─> Prompt twice (confirmation)
              │     ├─> Refuse empty passphrase
              │     ├─> Warn if < 8 chars (but allow)
              │     └─> NEVER write passphrase to disk
              ├─> Phase D: gpg-agent cache TTL
              │     ├─> Check ~/.gnupg/gpg-agent.conf
              │     ├─> If default-cache-ttl not set → propose 600s
              │     └─> Apply after Owner confirmation
              ├─> Phase E: Key generation + commit
              │     ├─> Run gpg --batch --gen-key
              │     ├─> Extract fingerprint
              │     ├─> Write to {owner_key_fingerprint_path}
              │     ├─> Git-commit with GPG signature
              │     └─> Delete sentinel file (one-shot)
              └─> Phase F: Trust anchor complete

aiplus agent audit re-sign-manifest
  └─> audit::commands::AuditArgs::ResignManifest
        └─> audit::handlers::handle_resign_manifest()
              ├─> Require explicit TUI prompt (no --yes)
              ├─> GPG-sign resulting commit with Owner key
              ├─> Verify fingerprint matches recorded
              └─> Update release_manifest
```

---

## 3. Type System Design

New types live in `crates/aiplus-core/src/agent_team/types.rs` (new file). These are shared between `aiplus-core` and `aiplus-cli`.

### 3.1 Core Enum Types

```rust
// crates/aiplus-core/src/agent_team/types.rs

use serde::{Deserialize, Serialize};

/// Acceptance mode per deliverable.
/// Schema §deliverables[].acceptance_mode
/// Hierarchy: deterministic > llm_judge > owner_review
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AcceptanceMode {
    /// Authoritative. Checks reduce to deterministic verdict.
    /// v0.1 ONLY supports this mode.
    Deterministic,
    /// Always routes to Owner; Auditor skips.
    #[serde(alias = "llm-judge")]
    LlmJudge,
    /// Owner review replaces Auditor entirely.
    OwnerReview,
}

/// Tier classification for trigger policy and canary sampling.
/// Schema §defaults.trigger_policy, §canary_replay.sample_distribution
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Tier {
    #[serde(rename = "LIGHT")]
    Light,
    #[serde(rename = "MEDIUM")]
    Medium,
    #[serde(rename = "HEAVY")]
    Heavy,
}

/// Individual check kind inside a deliverable.
/// Schema §deliverables[].checks[].kind
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CheckKind {
    /// Command exit code must match expected_exit.
    ExitCode,
    /// File or directory must exist.
    FileExists,
    /// Command stdout must match expected_regex.
    ShellOutputMatch,
}

/// Final verdict emitted by the Auditor for a single deliverable.
/// Schema §auditor_output
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "UPPERCASE")]
pub enum AuditorVerdict {
    /// All checks passed (or combiner satisfied).
    Pass,
    /// One or more checks failed.
    Fail,
    /// Pre-audit gate blocked execution.
    Blocked,
    /// Owner review substituted for Auditor judgment.
    OwnerReview,
}

/// Reasons a deliverable may be BLOCKED.
/// Schema §auditor_output.blocked_deliverables[].reason
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BlockedReason {
    /// Schema file hash mismatch or parse failure.
    SchemaTampered,
    /// Auditor internal error or pass disagreement.
    AuditorUnavailable,
    /// CEO (or any agent) attempted to bypass a gate.
    CeoOverrideAttempted,
    /// Release manifest not GPG-signed or wrong key fingerprint.
    ManifestUnsignedOrWrongKey,
    /// Release manifest has uncommitted changes.
    ManifestDirty,
    /// Attempted to use LLM for hash comparison (forbidden).
    HashViaLlm,
    /// Sentinel file absent or malformed during first-run setup.
    OwnershipUnverified,
    /// Sentinel path found in git history (should be gitignored).
    SentinelInGit,
}
```

### 3.2 Supporting Struct Types

```rust
/// A single check inside a deliverable.
/// Schema §deliverables[].checks[]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Check {
    pub id: String,
    pub kind: CheckKind,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cmd: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub expected_exit: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub path: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub expected_regex: Option<String>,
    #[serde(default = "default_check_timeout")]
    pub timeout_seconds: u64,
}

fn default_check_timeout() -> u64 {
    30
}

/// A deliverable as defined in the schema.
/// Schema §deliverables[]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Deliverable {
    pub deliverable_id: String,
    pub description: String,
    pub acceptance_mode: AcceptanceMode,
    pub check_combiner: CheckCombiner,
    pub checks: Vec<Check>,
    pub persisted_audit_script: String,
    pub self_test_script: String,
    #[serde(default)]
    pub related_stop_gates: Vec<String>,
    #[serde(default)]
    pub owner_review_required: bool,
}

/// How to combine multiple checks into a deliverable verdict.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CheckCombiner {
    AllMustPass,
    AnyPass,
}

/// Bin alias entry for cross-platform commands.
/// Schema §bin_aliases
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BinAlias {
    pub linux: String,
    pub macos: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fallback_macos: Option<String>,
}

/// Trigger policy per tier.
/// Schema §defaults.trigger_policy
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TriggerPolicy {
    #[serde(rename = "LIGHT")]
    pub light: TriggerAction,
    #[serde(rename = "MEDIUM")]
    pub medium: TriggerAction,
    #[serde(rename = "HEAVY")]
    pub heavy: TriggerAction,
    pub stop_gate_touched: TriggerAction,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TriggerAction {
    Skip,
    Sample,
    Full,
    ForceFull,
}

/// Audit script self-test requirements.
/// Schema §audit_script_self_test
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditScriptSelfTest {
    pub required: bool,
    pub fixture_diversity: FixtureDiversity,
    pub reviewer_constraint: ReviewerConstraint,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FixtureDiversity {
    pub pass_fixtures_min: u32,
    pub fail_fixtures_min: u32,
    pub fail_modes_covered: Vec<FailMode>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FailMode {
    DeliverableAbsent,
    DeliverablePartial,
    DeliverableCorrupted,
    DeliverableBehavioral,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReviewerConstraint {
    pub rule: String,
}

/// Owner spot-check queue entry.
/// Schema §owner_spot_check.queue
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpotCheckEntry {
    pub audit_run_id: String,
    pub deliverable_id: String,
    pub auditor_verdict: AuditorVerdict,
    pub owner_verdict: Option<AuditorVerdict>,
    pub note: Option<String>,
    pub retracted: bool,
    pub timestamp: String, // ISO8601
}

/// Canary replay state entry.
/// Schema §canary_replay
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CanaryState {
    pub audit_run_count: u64,
    pub last_canary_trigger: Option<String>, // ISO8601
    pub canary_dropped_this_run: u32,
    pub consecutive_drop_runs: u32,
}

/// Structured auditor output (YAML).
/// Schema §auditor_output
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditReport {
    pub schema_version: String,
    pub audit_run_id: String,
    pub started_at: String,
    pub completed_at: String,
    pub overall_verdict: AuditorVerdict,
    pub deliverables: Vec<DeliverableReport>,
    pub blocked_deliverables: Vec<BlockedDeliverable>,
    pub metrics: AuditMetrics,
    pub owner_feedback_prompt: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeliverableReport {
    pub deliverable_id: String,
    pub verdict: AuditorVerdict,
    pub checks: Vec<CheckReport>,
    pub execution_time_ms: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CheckReport {
    pub check_id: String,
    pub passed: bool,
    pub actual_exit_code: Option<i32>,
    pub actual_stdout: Option<String>,
    pub error: Option<String>,
    pub execution_time_ms: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BlockedDeliverable {
    pub deliverable_id: String,
    pub reason: BlockedReason,
    pub detail: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditMetrics {
    pub total_checks: u32,
    pub passed_checks: u32,
    pub failed_checks: u32,
    pub blocked_checks: u32,
    pub total_execution_time_ms: u64,
    pub canary_dropped_this_run: u32,
}

/// Sentinel file contents (YAML).
/// Schema §release_manifest.first_run_setup.ownership_verification
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OwnerSentinel {
    pub name: String,
    pub email: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub note: Option<String>,
}

/// Release manifest schema.
/// Schema §release_manifest.schema
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReleaseManifest {
    pub schema_version: String,
    pub released_at: String,
    pub released_by: String,
    pub auditor_min_version: String,
    pub acceptance_files: Vec<String>,
    pub audit_scripts: Vec<String>,
    pub audit_script_self_tests: Vec<String>,
    pub synthetic_fixtures: Vec<String>,
    pub bin_aliases_hash: String,
}
```

---

## 4. Module Hierarchy

### 4.1 New Files in `crates/aiplus-cli/src/agent/audit/`

```
crates/aiplus-cli/src/agent/
├── mod.rs                          (existing — add `pub mod audit;`)
├── commands.rs                     (existing — add Audit subcommand enum)
├── core.rs                         (existing)
├── handlers.rs                     (existing)
├── worktree.rs                     (existing)
├── cache.rs                        (existing)
└── audit/                          (NEW DIRECTORY)
    ├── mod.rs                      (re-exports, init)
    ├── commands.rs                 (AuditArgs, AuditSub CLI definitions)
    ├── handlers.rs                 (handle_audit_run, handle_setup_gpg, etc.)
    ├── schema.rs                   (load_schema, validate_schema, resolve_variables)
    ├── gate.rs                     (pre-audit gate: sentinel, GPG, manifest, hash)
    ├── checks.rs                   (check_exit_code, check_file_exists, check_shell_output_match)
    ├── run.rs                      (execute_deliverable, execute_checks, pass agreement)
    ├── verdict.rs                  (combine checks → deliverable verdict)
    ├── report.rs                   (AuditReport builder + YAML serialization)
    ├── persistence.rs              (write audit trail, read historical runs)
    ├── canary.rs                   (canary trigger, sample selection, replay, drift detection)
    ├── owner_feedback.rs           (owner-feedback command, queue management)
    ├── gpg_wizard.rs               (setup-gpg interactive wizard, batch config gen)
    └── bin_aliases.rs              (cross-platform bin alias resolution)
```

### 4.2 New Files in `crates/aiplus-core/src/agent_team/`

```
crates/aiplus-core/src/
├── lib.rs                          (existing — add `pub mod agent_team;`)
└── agent_team/
    ├── mod.rs                      (re-exports types)
    └── types.rs                    (ALL type definitions from Section 3)
```

### 4.3 Module Responsibilities

| Module | Purpose | Public API |
|--------|---------|-----------|
| `audit::commands` | CLI argument parsing for all `aiplus agent audit *` subcommands | `AuditArgs`, `AuditSub` |
| `audit::handlers` | Top-level dispatch for each audit subcommand | `handle_audit_run()`, `handle_setup_gpg()`, `handle_canary()`, `handle_owner_feedback()`, `handle_owner_feedback_retract()`, `handle_resign_manifest()` |
| `audit::schema` | Load, parse, and validate `schema.yaml`; resolve variables and bin aliases | `load_schema()`, `validate_schema()`, `resolve_variables()`, `resolve_bin_aliases()` |
| `audit::gate` | All pre-audit checks: sentinel, gitignore, manifest cleanliness, GPG signature, hash chain | `pre_audit_check()`, `verify_sentinel()`, `verify_manifest_signature()`, `verify_hash_chain()` |
| `audit::checks` | Execute individual check kinds | `check_exit_code()`, `check_file_exists()`, `check_shell_output_match()` |
| `audit::run` | Orchestrate deliverable execution across passes | `execute_deliverable()`, `execute_checks()`, `require_pass_agreement()` |
| `audit::verdict` | Combine check results into deliverable verdict | `combine_checks()`, `all_must_pass()`, `any_pass()` |
| `audit::report` | Build and serialize `AuditReport` | `AuditReport::builder()`, `to_yaml()` |
| `audit::persistence` | Write/read audit trail files | `write_audit_trail()`, `read_audit_run()`, `list_audit_runs()` |
| `audit::canary` | Canary replay subsystem | `maybe_trigger()`, `select_samples()`, `replay_audit_run()`, `detect_drift()` |
| `audit::owner_feedback` | Owner spot-check queue | `enqueue_feedback()`, `retract_feedback()`, `calculate_metrics()` |
| `audit::gpg_wizard` | Interactive GPG setup wizard | `run_setup_gpg_wizard()`, `generate_batch_config()`, `configure_gpg_agent()` |
| `audit::bin_aliases` | Cross-platform command resolution | `resolve(alias_name, os)`, `tokenize(cmd)` (uses `shell-words`) |

---

## 5. Dependency Decisions

### 5.1 New Dependencies

| Crate | Version | Where | Purpose | Rationale |
|-------|---------|-------|---------|-----------|
| `serde_yaml_ng` | `0.10` | `aiplus-core` + `aiplus-cli` | Parse `schema.yaml`, sentinel YAML, audit report YAML | **Explicit requirement: use `serde_yaml_ng`, NOT `serde_yaml`** (legacy crate). `serde_yaml_ng` is the maintained fork. |
| `shell-words` | `1.1` | `aiplus-cli` (`audit::bin_aliases`) | Tokenize `bin_aliases` command strings into `Vec<String>` for `std::process::Command` | **Explicit requirement**. Prevents shell-injection when expanding `{sha256}` → `"shasum -a 256"`. Correctly handles spaces and quotes. |
| `fs2` | `0.4` | `aiplus-cli` (`audit::persistence`) | `flock(2)` advisory locking on audit trail files | **Explicit requirement**. Prevents concurrent `aiplus agent audit run` processes from corrupting shared `audit-trail/` and `canary-replay-state.jsonl` files. |
| `regex` | `1.10` | `aiplus-cli` (`audit::checks`) | Match `shell_output_match` expected_regex | Already widely used; no additional transitive deps concern. |
| `chrono` | `0.4` | `aiplus-core` + `aiplus-cli` | ISO8601 timestamps, date arithmetic for canary freshness | Needed for `released_at`, `started_at`, `last_audit_max_age_days`. |

### 5.2 Dependency Placement

**Workspace `Cargo.toml` additions:**

```toml
[workspace.dependencies]
# ... existing deps ...
serde_yaml_ng = "0.10"
shell-words = "1.1"
fs2 = "0.4"
regex = "1.10"
chrono = { version = "0.4", features = ["serde"] }
```

**`crates/aiplus-core/Cargo.toml` additions:**

```toml
[dependencies]
# ... existing ...
serde_yaml_ng = { workspace = true }
chrono = { workspace = true }
```

**`crates/aiplus-cli/Cargo.toml` additions:**

```toml
[dependencies]
# ... existing ...
serde_yaml_ng = { workspace = true }
shell-words = { workspace = true }
fs2 = { workspace = true }
regex = { workspace = true }
chrono = { workspace = true }
```

### 5.3 Dependency Rationale

- **`serde_yaml_ng` vs `serde_yaml`**: `serde_yaml` is deprecated/unmaintained. `serde_yaml_ng` is the community-maintained successor. The schema freeze explicitly requires `serde_yaml_ng`.
- **`shell-words` vs manual `split_whitespace()`**: `bin_aliases` values contain spaces (`"shasum -a 256"`). Manual splitting would break on multi-word arguments. `shell-words` implements POSIX shell word splitting rules correctly.
- **`fs2` vs `fd-lock`**: `fs2` provides a simple, cross-platform `flock` wrapper. `audit-trail/` and `canary-replay-state.jsonl` are append-only shared files; advisory locking is sufficient (no need for `fd-lock`'s more complex API).
- **`regex`**: The schema requires `shell_output_match` checks with `expected_regex`. We need the full regex engine (not just `String::contains`).
- **`chrono`**: ISO8601 parsing and date arithmetic are error-prone to implement manually. `chrono` is the standard Rust solution.

---

## 6. Chinese Alias Mapping

### 6.1 Agent Subcommands (Existing — 13 total)

These are **already implemented** in `crates/aiplus-cli/src/agent/commands.rs`. Listed here for completeness and to show the mapping contract.

| English Command | Chinese Aliases | Rust Enum Variant | Handler Function |
|-----------------|-----------------|-------------------|------------------|
| `agent status` | `团队`, `团` | `AgentSub::Status` | `handle_status()` |
| `agent doctor` | `健康`, `诊断` | `AgentSub::Doctor` | `handle_doctor()` |
| `agent list` | `列表`, `清单` | `AgentSub::List` | `handle_list()` |
| `agent talk <role>` | `跟`, `找` | `AgentSub::Talk` | `handle_talk()` |
| `agent route <task>` | `派单`, `分` | `AgentSub::Route` | `handle_route()` |
| `agent reset` | `重置`, `复位` | `AgentSub::Reset` | `handle_reset()` |
| `agent invite <role>` | `召唤`, `请` | `AgentSub::Invite` | `handle_invite()` |
| `agent dismiss <role>` | `让走`, `解散` | `AgentSub::Dismiss` | `handle_dismiss()` |
| `agent disable <role>` | `禁用`, `关闭` | `AgentSub::Disable` | `handle_disable()` |
| `agent enable <role>` | `启用`, `打开` | `AgentSub::Enable` | `handle_enable()` |
| `agent integrate <role>` | `合并`, `集成` | `AgentSub::Integrate` | `handle_integrate()` |
| `agent transcript` | `看活`, `记录` | `AgentSub::Transcript` | `handle_transcript()` |
| `agent prune-worktrees` | `清`, `清理` | `AgentSub::PruneWorktrees` | `handle_prune_worktrees()` |

### 6.2 Audit Subcommands (NEW — 9 total)

These map to the **new** `AuditSub` enum in `crates/aiplus-cli/src/agent/audit/commands.rs`.

| English Command | Chinese Aliases | Rust Enum Variant | Handler Function | Schema § |
|-----------------|-----------------|-------------------|------------------|----------|
| `agent audit run` | `审`, `审查` | `AuditSub::Run` | `handle_audit_run()` | deliverables, defaults |
| `agent audit canary` | `金丝雀`, `复查` | `AuditSub::Canary` | `handle_canary()` | canary_replay |
| `agent audit replay <id>` | `重放`, `回放` | `AuditSub::Replay { run_id }` | `handle_replay()` | canary_replay.replay_command |
| `agent audit setup-gpg` | `配置签名`, `初始化` | `AuditSub::SetupGpg` | `handle_setup_gpg()` | release_manifest.first_run_setup |
| `agent audit re-sign-manifest` | `重新签名`, `更新清单` | `AuditSub::ResignManifest` | `handle_resign_manifest()` | release_manifest.ownership |
| `agent audit owner-feedback` | `反馈`, `评价` | `AuditSub::OwnerFeedback` | `handle_owner_feedback()` | owner_spot_check |
| `agent audit owner-feedback-retract` | `撤回反馈` | `AuditSub::OwnerFeedbackRetract` | `handle_owner_feedback_retract()` | owner_spot_check.feedback_retraction |
| `agent audit status` | `审计状态`, `审态` | `AuditSub::Status` | `handle_audit_status()` | (diagnostic) |
| `agent audit history` | `审计历史`, `记录` | `AuditSub::History` | `handle_audit_history()` | (diagnostic) |

### 6.3 Audit Arguments (Detailed)

```rust
// crates/aiplus-cli/src/agent/audit/commands.rs

use clap::{Parser, Subcommand};

#[derive(Parser)]
pub struct AuditArgs {
    #[command(subcommand)]
    pub subcommand: AuditSub,
}

#[derive(Subcommand)]
pub enum AuditSub {
    /// Run audit against acceptance schema
    #[command(visible_aliases = ["审", "审查"])]
    Run {
        #[arg(long)]
        deliverable: Option<String>,
        #[arg(long, default_value = "deterministic")]
        mode: String,
    },

    /// Trigger canary replay
    #[command(visible_aliases = ["金丝雀", "复查"])]
    Canary,

    /// Replay a specific audit run
    #[command(visible_aliases = ["重放", "回放"])]
    Replay {
        run_id: String,
    },

    /// First-run GPG setup wizard
    #[command(visible_aliases = ["配置签名", "初始化"])]
    SetupGpg,

    /// Re-sign release manifest
    #[command(visible_aliases = ["重新签名", "更新清单"])]
    ResignManifest,

    /// Provide owner feedback on an audit run
    #[command(visible_aliases = ["反馈", "评价"])]
    OwnerFeedback {
        run_id: String,
        #[arg(long)]
        actual_verdict: String,
        #[arg(long)]
        note: Option<String>,
    },

    /// Retract previous owner feedback
    #[command(visible_aliases = ["撤回反馈"])]
    OwnerFeedbackRetract {
        run_id: String,
    },

    /// Show audit subsystem status
    #[command(visible_aliases = ["审计状态", "审态"])]
    Status,

    /// Show audit history
    #[command(visible_aliases = ["审计历史", "记录"])]
    History {
        #[arg(long)]
        limit: Option<usize>,
    },
}
```

### 6.4 Wiring into Main Agent Command

`crates/aiplus-cli/src/agent/commands.rs` adds:

```rust
#[derive(Subcommand)]
pub enum AgentSub {
    // ... existing variants ...

    /// Audit subsystem (acceptance criteria, canary, GPG)
    #[command(visible_aliases = ["审", "审查"])]
    Audit {
        #[command(subcommand)]
        subcommand: crate::agent::audit::commands::AuditSub,
    },
}
```

`crates/aiplus-cli/src/agent/mod.rs` updates dispatch:

```rust
pub fn dispatch(args: AgentArgs) -> Result<()> {
    match args.subcommand {
        // ... existing arms ...
        AgentSub::Audit { subcommand } => {
            crate::agent::audit::dispatch(subcommand)
        }
    }
}
```

---

## 7. File Creation Checklist

### 7.1 New Files (16 files)

| # | File Path | Purpose | Lines (est.) | Dependencies |
|---|-----------|---------|--------------|--------------|
| 1 | `crates/aiplus-core/src/agent_team/mod.rs` | Re-export types module | 5 | — |
| 2 | `crates/aiplus-core/src/agent_team/types.rs` | **All type definitions** (Section 3) | 250 | `serde`, `serde_yaml_ng` |
| 3 | `crates/aiplus-cli/src/agent/audit/mod.rs` | Audit module init, re-exports, `dispatch()` | 30 | — |
| 4 | `crates/aiplus-cli/src/agent/audit/commands.rs` | `AuditArgs`, `AuditSub` CLI enum | 80 | `clap` |
| 5 | `crates/aiplus-cli/src/agent/audit/handlers.rs` | Top-level handler dispatch | 150 | `anyhow` |
| 6 | `crates/aiplus-cli/src/agent/audit/schema.rs` | Load, validate, resolve schema.yaml | 200 | `serde_yaml_ng`, `anyhow` |
| 7 | `crates/aiplus-cli/src/agent/audit/gate.rs` | Pre-audit gate: sentinel, GPG, manifest, hash | 250 | `anyhow`, `fs2`, `regex` |
| 8 | `crates/aiplus-cli/src/agent/audit/checks.rs` | Check execution: exit_code, file_exists, shell_output_match | 150 | `anyhow`, `regex` |
| 9 | `crates/aiplus-cli/src/agent/audit/run.rs` | Deliverable execution orchestration | 200 | `anyhow` |
| 10 | `crates/aiplus-cli/src/agent/audit/verdict.rs` | Verdict combination logic | 80 | — |
| 11 | `crates/aiplus-cli/src/agent/audit/report.rs` | AuditReport builder + YAML output | 120 | `serde_yaml_ng`, `chrono` |
| 12 | `crates/aiplus-cli/src/agent/audit/persistence.rs` | Audit trail read/write with flock | 150 | `fs2`, `anyhow`, `serde_json` |
| 13 | `crates/aiplus-cli/src/agent/audit/canary.rs` | Canary trigger, sample, replay, drift | 200 | `chrono`, `anyhow` |
| 14 | `crates/aiplus-cli/src/agent/audit/owner_feedback.rs` | Owner feedback queue management | 150 | `serde_json`, `anyhow` |
| 15 | `crates/aiplus-cli/src/agent/audit/gpg_wizard.rs` | Interactive GPG setup wizard | 250 | `anyhow` |
| 16 | `crates/aiplus-cli/src/agent/audit/bin_aliases.rs` | Cross-platform bin alias resolution | 80 | `shell-words` |

**Total new estimated lines: ~2,145**

### 7.2 Modified Files (5 files)

| # | File Path | Change | Rationale |
|---|-----------|--------|-----------|
| 1 | `crates/aiplus-cli/src/agent/mod.rs` | Add `pub mod audit;`, update `dispatch()` | Wire audit module into agent command tree |
| 2 | `crates/aiplus-cli/src/agent/commands.rs` | Add `AgentSub::Audit { subcommand }` variant | Expose `aiplus agent audit *` CLI surface |
| 3 | `crates/aiplus-core/src/lib.rs` | Add `pub mod agent_team;` | Expose shared types to CLI crate |
| 4 | `crates/aiplus-cli/Cargo.toml` | Add: `serde_yaml_ng`, `shell-words`, `fs2`, `regex`, `chrono` | Audit subsystem dependencies |
| 5 | `crates/aiplus-core/Cargo.toml` | Add: `serde_yaml_ng`, `chrono` | Shared types dependencies |
| 6 | `Cargo.toml` (workspace root) | Add: `serde_yaml_ng`, `shell-words`, `fs2`, `regex`, `chrono` to `[workspace.dependencies]` | Centralize version management |

### 7.3 Generated Files (by the system, not hand-written)

| File Path | Generated By | Schema § |
|-----------|--------------|----------|
| `.aiplus/agent-team/audit-scripts/*.sh` | `audit::run` (on first run) | deliverables[].persisted_audit_script |
| `.aiplus/agent-team/audit-scripts/*.test.sh` | Human / Agent author | deliverables[].self_test_script |
| `.aiplus/agent-team/audit-trail/*.yaml` | `audit::persistence` | audit_trail_dir |
| `.aiplus/agent-team/audit-trail/owner-spot-check-queue.jsonl` | `audit::owner_feedback` | owner_spot_check.queue |
| `.aiplus/agent-team/audit-trail/canary-replay-state.jsonl` | `audit::canary` | canary_replay_state |
| `.aiplus/agent-team/release-manifest.yaml` | `aiplus agent audit re-sign-manifest` | release_manifest.path |
| `.aiplus/agent-team/owner-key-fingerprint` | `audit::gpg_wizard` | owner_key_fingerprint_path |
| `.aiplus/agent-team/.owner-setup-authorized` | Owner (manually, one-shot) | owner_setup_sentinel_path |

---

## 8. Implementation Order (Recommended)

1. **Phase 0**: Add dependencies to workspace + crate `Cargo.toml` files
2. **Phase 1**: Create `aiplus-core/src/agent_team/types.rs` + `mod.rs` (types unlock everything)
3. **Phase 2**: Create `audit/commands.rs` + `audit/mod.rs` + wire into `agent/mod.rs` and `agent/commands.rs`
4. **Phase 3**: Implement `audit/schema.rs` (load/parse/validate `schema.yaml`)
5. **Phase 4**: Implement `audit/bin_aliases.rs` + `audit/checks.rs` (foundations)
6. **Phase 5**: Implement `audit/gate.rs` (sentinel → GPG → manifest → hash chain)
7. **Phase 6**: Implement `audit/run.rs` + `audit/verdict.rs` (deliverable execution)
8. **Phase 7**: Implement `audit/report.rs` + `audit/persistence.rs` (output + trail)
9. **Phase 8**: Implement `audit/gpg_wizard.rs` (setup-gpg command)
10. **Phase 9**: Implement `audit/canary.rs` (canary replay subsystem)
11. **Phase 10**: Implement `audit/owner_feedback.rs` (owner-feedback commands)
12. **Phase 11**: Implement `audit/handlers.rs` (wire all handlers)
13. **Phase 12**: Generate 7 audit scripts + 7 self-test scripts (per deliverables)
14. **Phase 13**: Integration test: run `aiplus agent audit run` end-to-end

---

## 9. Constraints & Boundaries

- **DO NOT modify** the frozen schema file at `.aiplus/agent-team/acceptance/v0.1.4/schema.yaml`
- **DO NOT modify** existing agent code (status, doctor, list, talk, route, etc.) except for the 5 wiring changes listed in Section 7.2
- **Use `serde_yaml_ng`** for all YAML parsing — never `serde_yaml`
- **Use `shell-words`** for tokenizing `bin_aliases` command strings
- **Use `fs2`** for `flock(2)` advisory locking on shared audit trail files
- **Deterministic-only** for v0.1: `llm_judge` and `owner_review` modes are parsed but always route to `BLOCKED` or `OwnerReview` with a clear message
- **GPG strong_mode**: EDDSA + Curve25519 primary; RSA-4096 fallback; passphrase required; never `%no-protection`
- **Sentinel**: One-shot, manual Owner creation, auto-deleted after successful setup
- **Exit code 3** for `ownership_unverified` (user-actionable), exit code 2 for all other `BLOCKED` reasons

---

## 10. Schema Compliance Matrix

| Schema Section | Implementation Location | Test Location |
|----------------|------------------------|---------------|
| `defaults` | `audit::schema::Defaults`, `audit::run` | `tests/defaults.rs` |
| `variables` | `audit::schema::resolve_variables()` | `tests/variables.rs` |
| `bin_aliases` | `audit::bin_aliases::resolve()` | `tests/bin_aliases.rs` |
| `deliverables` | `audit::run::execute_deliverable()` | 7 `.test.sh` scripts |
| `audit_script_self_test` | `audit::schema::validate_self_tests()` | `tests/self_test.rs` |
| `release_manifest` | `audit::gate`, `audit::gpg_wizard` | `tests/manifest.rs` |
| `canary_replay` | `audit::canary` | `tests/canary.rs` |
| `owner_spot_check` | `audit::owner_feedback` | `tests/owner_feedback.rs` |
| `auditor_output` | `audit::report` | `tests/report.rs` |
| `failure_modes` | `audit::gate`, `audit::handlers` | `tests/failure_modes.rs` |
| `validation_rules` | `audit::schema::validate_schema()` | `tests/validation.rs` |
| `invalid_examples` | (documentation + negative tests) | `tests/invalid_examples.rs` |

---

*End of Implementation Manifest*
