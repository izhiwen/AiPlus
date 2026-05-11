# Release Packet Template

Template for generating AiPlus release readiness packets.

## Packet Metadata

```
Release: v{VERSION}
Date: {YYYY-MM-DD}
Status: DRY_RUN | READY_FOR_REVIEW | BLOCKED
Author: Release Automation Lead
```

## Version Consistency

| Source | Version | Status |
|--------|---------|--------|
| crates/aiplus-cli/Cargo.toml | {VERSION} | PASS / FAIL |
| CLI --version | {VERSION} | PASS / FAIL |
| CHANGELOG.md | {VERSION} | PASS / FAIL |
| Release packet | {VERSION} | PASS / FAIL |

## Rust QA Results

| Check | Command | Status | Evidence |
|-------|---------|--------|----------|
| Format | cargo fmt --all --check | PASS / WARN / BLOCK | |
| Lint | cargo clippy --workspace --all-targets --all-features | PASS / WARN / BLOCK | |
| Test | cargo test --workspace | PASS / WARN / BLOCK | |
| Metadata | cargo metadata --format-version 1 | PASS / BLOCK | |
| Git diff | git diff --check | PASS / WARN | |

## CLI Smoke Tests

| Command | Status | Evidence |
|---------|--------|----------|
| aiplus --help | PASS / FAIL | |
| aiplus doctor | PASS / FAIL | |
| aiplus memory doctor | PASS / FAIL | |
| aiplus profile doctor aiplus-work-with-zhiwen | PASS / FAIL | |
| aiplus status | PASS / FAIL | |
| aiplus compact validate | PASS / FAIL | |

## Secret & Boundary Scan

| Check | Status | Evidence |
|-------|--------|----------|
| Secret values | PASS / BLOCK | |
| Raw transcripts | PASS / WARN | |
| Private profile leakage | PASS / WARN | |
| Telemetry | PASS / WARN | |
| Global config edits | PASS / WARN | |
| Node fallback | PASS / BLOCK | |
| Production paths | PASS / WARN | |
| Bitwarden tokens | PASS / BLOCK | |

## Artifact Dry-Run

| Check | Status | Evidence |
|-------|--------|----------|
| Binary builds | PASS / WARN | |
| Archive created | PASS / WARN | |
| LICENSE included | PASS / BLOCK | |
| Binary included | PASS / BLOCK | |
| Checksum generated | PASS / WARN | |
| No excluded items | PASS / WARN | |

## Subproduct Drift

| Subproduct | Asset Files | Source Files | Version Match | Private Content |
|------------|-------------|--------------|---------------|-----------------|
| aiplus-compact-reminder | {count} | {count} | PASS / WARN | PASS / BLOCK |
| aiplus-auto-team-consultant | {count} | {count} | PASS / WARN | PASS / BLOCK |
| aiplus-agent-memory | {count} | {count} | PASS / WARN | PASS / BLOCK |

## Safety Packet

```
publish_push_release_attempted: no
global_config_touched: no
private_profile_copied_to_public: no
raw_transcript_or_provider_payload_stored: no
real_memory_deleted_or_modified: no
external_accounts_touched: no
secret_values_read_or_printed: no
commands_blocked_for_owner_gate: [push, tag, release, upload, publish, deploy]
redactions_applied: [secret_values, provider_payloads, personal_paths]
remaining_owner_approvals_needed: []
```

## Known Limitations

- {Limitation 1}
- {Limitation 2}

## Blockers

- {Blocker 1} or "None"

## Recommendations

- {Recommendation 1}
- {Recommendation 2}

## Approval Status

- [ ] Release Automation Lead: PASS
- [ ] Platform CEO: {pending / approved / blocked}
- [ ] Owner: {pending / approved / blocked}
