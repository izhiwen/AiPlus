# AiPlus Auto Compact Core Protocol

The core protocol defines local compact/resume state that any host agent can read, validate, checkpoint, and resume from. It is intentionally platform-neutral: adapters may expose the workflow inside a specific agent runtime, but the core files do not depend on a hosted API, cloud service, remote repository, global install, or automatic compact trigger.

## State Location

The compatibility state path is `.codex/compact/` in the target repository. The path is retained to preserve proven `compactctl` behavior and existing fixtures. Host adapters may map this path into their own workspace conventions, but the core CLI validates and writes only inside this local state directory.

Required files:

- `current-handoff.md`
- `decision-log.md`
- `agent-state-ledger.md`
- `evidence-ledger.md`
- `compact-policy.json`
- `checkpoints/`

## Version Contract

The initial protocol, schema, and template versions are `0.1.0`. Unknown or missing versions are review items, not silent upgrades. A resume should be blocked or marked unknown until an owner or host adapter verifies compatibility.

## Validation Contract

Validation is structural only. A validation pass means the state files have the required sections, allowed enum values, expected policy flags, and no detected sensitive patterns. It does not mean compacting is safe.

The core CLI treats:

- denied owner gates, validation errors, and sensitive warnings as `BLOCKED_DO_NOT_COMPACT`;
- pending owner gates or unknown versions as `UNKNOWN_NEEDS_REVIEW`;
- clean validation with no denied or pending gates as `SAFE_TO_COMPACT`.

## Redaction Contract

Core state must not store secrets, private paths, raw transcripts, provider payloads, HAR/WebRTC dumps, or PII. Use placeholders such as `<REPO_ROOT>`, `<HOME>`, `<REDACTED_SECRET>`, `<REDACTED_TOKEN>`, `<REDACTED_PII>`, and `<ARTIFACT_ID>`.
