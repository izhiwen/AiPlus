# AiPlus Compact Reminder Core Protocol

The core protocol defines local compact/reminder/resume state that any host agent
can read, validate, checkpoint, and resume from. It is intentionally
platform-neutral: adapters may expose the workflow inside a specific agent
runtime, but the core files do not depend on a hosted API, cloud service, remote
repository, global install, or automatic host compact trigger.

AiPlus Compact Reminder does not replace Codex, Claude Code, or OpenCode compact.
Its core value is proactive reminder timing: at stable high-value moments it
runs `aiplus compact remind`, prepares a checkpoint/handoff first, estimates
token/USD savings, and then tells the agent whether to wait, prepare only, or
recommend manual host compact.

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

## Reminder Contract

`aiplus compact remind` is the proactive decision surface. It never clicks or
calls the host compact control. It reports:

- `REMINDER_DECISION=remind_now | prepare_only | wait | blocked`
- `REMINDER_LEVEL=soft | ready | safety_block`
- `HANDOFF_STATE=missing | template_only | stale | current`
- `RECOVERY_CONFIDENCE=high | medium | low`
- estimated token/USD savings when local pricing is available

Adapters should call it on HEAVY tasks every 30 minutes or major phase boundary,
on MEDIUM tasks at phase boundaries or before review/QA, and before subagent
bursts, release prep, or Owner handoff. If the decision is `remind_now`, the
agent should run or confirm `aiplus compact prepare`, then suggest manual host
compact. If the decision is `prepare_only`, `wait`, or `blocked`, the agent must
not suggest host compact yet.

## Redaction Contract

Core state must not store secrets, private paths, raw transcripts, provider payloads, HAR/WebRTC dumps, or PII. Use placeholders such as `<REPO_ROOT>`, `<HOME>`, `<REDACTED_SECRET>`, `<REDACTED_TOKEN>`, `<REDACTED_PII>`, and `<ARTIFACT_ID>`.
