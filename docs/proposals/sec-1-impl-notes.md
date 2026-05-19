# SEC-1 Layer-7 Security Implementation Notes

Status: Phase 3 evidence complete
Date: 2026-05-18
Authoritative briefing: `aiplus-agent-team/docs/decisions/sec-1-impl-briefing.md`
Goal: `aiplus-agent-team/docs/proposals/goal-G-AT-SEC-1.md`
Design source: `aiplus-agent-team/DESIGN.md` sections 15.1 and 22

## 1. Hash Chain Format

Dispatch log path: `.aiplus/agents/dispatch-log.jsonl`.

SEC-1 chains every new dispatch-log event emitted by the CLI after the first
SEC-1 event. This includes `coordinator_decision`, role dispatch rows, and
`auditor_verdict`. Pre-existing rows without SEC-1 chain fields are legacy and
outside verification scope.

Fields added to new log rows:

- `genesis: true` on the first chained row when the existing log has no prior
  chained row.
- `prev_hash: "<sha256>"` on every later chained row.
- `entry_hash: "<sha256>"` on every chained row.

Hash input:

- Canonical JSON UTF-8 bytes.
- Lexicographic object-key order at every object level.
- No whitespace.
- Standard JSON string escaping.
- `entry_hash` is excluded from the self-hash input.
- `prev_hash` and `genesis` are included when present.

Append algorithm:

```text
event = event JSON without chain fields
last = last dispatch-log row, if any
if last has entry_hash:
  event.prev_hash = last.entry_hash
else:
  event.genesis = true
event.entry_hash = sha256(canonical(event without entry_hash))
append canonical(event with entry_hash) as JSONL
```

Verification algorithm:

```text
for each line:
  parse JSON
  if no entry_hash and no genesis and no prev_hash:
    if chain_started: FAIL missing_chain_fields
    else: legacy, continue
  chain_started = true
  recompute entry_hash from line with entry_hash removed
  if mismatch: FAIL line=N reason=entry_hash_mismatch
  if genesis:
    if prior chained line exists: FAIL line=N reason=unexpected_genesis
  else:
    if prev_hash missing: FAIL line=N reason=missing_prev_hash
    if prev_hash != prior.entry_hash: FAIL line=N reason=hash_mismatch
```

`entry_hash` is an intentional extension beyond the briefing's minimum
`prev_hash` so verification catches tail-line tampering, not only tampering
that is followed by another chained row.

## 2. Auditor Workflow

CLI flag: `aiplus agent route --auditor-provider <provider> "<task>"`.

The auditor runs sequentially after the primary route path completes. It is
opt-in and never changes the primary route outcome if the primary succeeded.
If auditor execution fails, the CLI reports a non-zero auditor error after the
primary dispatch artifacts remain written.

Primary provider detection:

1. `AIPLUS_PRIMARY_PROVIDER` env override for tests and advanced users.
2. First runtime adapter in `.aiplus/install-manifest.json`, when present.
3. Fallback `local-cli`.

The auditor provider must normalize to one of `codex`, `claude-code`, or
`opencode`, and must differ from the detected primary provider. This enforces
provider-family separation at the available CLI layer. A future adapter-native
implementation can replace the detection with `AdapterResult.provider` when
runtime adapter execution returns structured results to this route path.

Auditor prompt template:

```text
SEC-1 cross-provider audit.
Original task:
<task excerpt>

Primary provider: <primary_provider>
Primary output summary:
<dispatch summary>

Return verdict agree, disagree, or flag with concise reasoning.
```

Deterministic local verdict classifier for this round:

- `flag` for ambiguity, uncertainty, security, payment, auth, secret, or token
  risk terms.
- `agree` otherwise.
- `disagree` remains allowed in schema for future live adapter-backed runs.

Event schema:

```json
{
  "schemaVersion": "0.6.4",
  "event": "auditor_verdict",
  "decisionId": "<coordinator decision id or manual route id>",
  "timestamp": "<iso>",
  "auditor_provider": "codex",
  "primary_provider": "claude-code",
  "verdict": "agree|disagree|flag",
  "reasoning_summary": "<=200 chars",
  "secretValues": "none",
  "prev_hash": "<sha256>",
  "entry_hash": "<sha256>"
}
```

`aiplus doctor` reports `auditor_provider_configured=<provider>` when
`AIPLUS_AUDITOR_PROVIDER` is set, otherwise `auditor_provider_configured=disabled`.

## 3. Secure Enclave Setup

Command: `aiplus identity setup-signing [--dry-run]`.

macOS setup steps:

```text
ssh-keygen -t ecdsa-sk -O resident -O verify-required \
  -C aiplus-secure-enclave \
  -f ~/.ssh/id_ecdsa_sk_aiplus
git config --global gpg.format ssh
git config --global user.signingkey ~/.ssh/id_ecdsa_sk_aiplus.pub
git config --global commit.gpgsign true
git config --global gpg.ssh.allowedSignersFile ~/.ssh/aiplus_allowed_signers
```

`~/.ssh/aiplus_allowed_signers` gets one line:

```text
<git user.email or aiplus-owner@example.invalid> <public-key>
```

Idempotency and safety:

- `--dry-run` prints planned key path and git config changes without writing.
- Existing `gpg.format`, `user.signingkey`, `commit.gpgsign`, or
  `gpg.ssh.allowedSignersFile` values are printed before changes.
- Existing signing configuration is not clobbered unless it already points at
  the AiPlus Secure Enclave key path or is absent.
- Existing key files are reused rather than overwritten.
- Non-macOS prints `SETUP_SIGNING=UNSUPPORTED platform=<platform>` and exits
  cleanly.

YubiKey: documented in command help/notes only; no YubiKey setup code in this
round.

`aiplus doctor` reports `commit_signing=secure_enclave|ssh|gpg|none` based on
global git config.

## 4. Test Plan

Sub-feature 1 tests: `sec_1_tamper_evident_smoke.rs`

- Fresh dispatch log verify: PASS.
- Tamper a chained line: `verify-log` FAIL with line number.
- Legacy pre-genesis row then new route: legacy row ignored and first new row
  is genesis.

Sub-feature 2 tests: `sec_1_auditor_smoke.rs`

- `--auditor-provider codex` writes `auditor_verdict`.
- Same provider is rejected.
- Ambiguous task produces `flag` with non-empty reasoning.

Sub-feature 3 tests: `sec_1_setup_signing_smoke.rs`

- `setup-signing --dry-run` prints planned Secure Enclave and git config steps.
- Existing signing config is detected and not clobbered.
- Non-macOS forced by env override degrades cleanly.
- Fake keygen env in temp HOME verifies isolated git config writes without
  touching Owner global config.

End-to-end demos in Phase 3:

- `aiplus agent audit verify-log` PASS on fresh chained log.
- Manual tamper causes FAIL with line number.
- `aiplus agent route --auditor-provider codex "<task>"` writes
  `auditor_verdict`.
- `aiplus identity setup-signing --dry-run` previews global git changes safely.

## 5. Draft CHANGELOG 0.6.4 Text

```text
## 0.6.4

- Added tamper-evident dispatch-log verification with `aiplus agent audit
  verify-log`.
- Added opt-in cross-provider auditor verdict logging via `aiplus agent route
  --auditor-provider <provider>`.
- Added `aiplus identity setup-signing` to configure Mac Secure Enclave-backed
  SSH commit signing, with dry-run and existing-config protection.
```

## Phase 3 Evidence

Implementation completed in worktree
`~/Projects/AiPlus/aiplus-public.sec-1` on branch `feat/sec-1`.

### Acceptance Status

- D1 tamper-evident dispatch log: _IMPL-OK_.
  - New chained rows include `genesis` or `prev_hash` plus `entry_hash`.
  - `aiplus agent audit verify-log` returns `VERIFY_LOG=PASS` or
    `VERIFY_LOG=FAIL line=N reason=<...>`.
  - `aiplus doctor` prints `INFO dispatch_log_chain=<status>`.
  - Parallel route append race found during auditor testing; fixed with a
    process-local append mutex around hash-chain read/append.
- D2 cross-provider auditor: _IMPL-OK_.
  - `aiplus agent route --auditor-provider <provider> "<task>"` records
    `auditor_verdict` after the primary route path.
  - Same-provider auditor is rejected.
  - `aiplus doctor` prints `INFO auditor_provider_configured=<provider|disabled>`.
- D3 hardware-backed signing setup: _IMPL-OK_.
  - `aiplus identity setup-signing [--dry-run]` added.
  - macOS path uses Secure Enclave/FIDO2 `ssh-keygen -t ecdsa-sk -O resident
    -O verify-required`.
  - Non-macOS degrades with `SETUP_SIGNING_STATUS=UNSUPPORTED`.
  - Existing signing config is refused instead of clobbered.
  - Tests isolate `HOME` and `GIT_CONFIG_GLOBAL`; no Owner global config was
    edited by the test suite.
  - `aiplus doctor` prints `INFO commit_signing=<secure_enclave|ssh|gpg|none>`.

### Verification Commands

```text
rtk cargo fmt
rtk cargo test -p aiplus-cli --test sec_1_tamper_evident_smoke -- --nocapture
  PASS: 4 passed
rtk cargo test -p aiplus-cli --test sec_1_auditor_smoke -- --nocapture
  PASS: 3 passed
rtk cargo test -p aiplus-cli --test sec_1_setup_signing_smoke -- --nocapture
  PASS: 4 passed
rtk cargo test -p aiplus-cli
  PASS: 363 passed, 1 ignored (39 suites, 41.73s)
rtk cargo test
  PASS: 558 passed, 1 ignored (41 suites, 72.05s)
```

### End-to-End Coverage

- Fresh chained dispatch log verifies successfully.
- Mutating a chained row causes `VERIFY_LOG=FAIL line=2`.
- Legacy pre-genesis rows are ignored until the first SEC-1 chained row.
- `--auditor-provider codex` writes a chained `auditor_verdict` event with a
  `flag` verdict for an intentionally ambiguous secure payment prompt.
- `identity setup-signing --dry-run` previews Secure Enclave key generation and
  global git config without writing.
- Fake-keygen signing setup writes isolated SSH signing config and doctor reports
  `commit_signing=secure_enclave`.

### Deviations / Notes

- The auditor implementation is deterministic and local in this round rather
  than adapter-live. It preserves the event schema and provider-separation
  policy while avoiding live provider cost/flakiness in regression tests.
- No CONTRACT, adapter, coordinator scoring, calibration fixture, version,
  CHANGELOG, or install.sh files were edited.
