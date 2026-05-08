# Compact Handoff

Synthetic template. Replace placeholders before use.

## Protocol Version

0.1.0

## Last Updated

<ISO8601_TIMESTAMP>

## Current Goal

Initialize compact/resume handoff state for <REPO_ROOT>.

## Current Phase

IN_PROGRESS

## Completed Work

- Created compact protocol files.

## Open Blockers

- None.

## Owner Gates

- UNKNOWN_PENDING: Owner review of compact handoff before first real use.

## Next 3 Actions

1. Review all compact files for project-specific placeholders.
2. Run `aiplus compact validate`.
3. Run `aiplus compact checkpoint` before manual compact.

## Do Not Do

- Do not include secrets, PII, raw audio, transcripts, provider payloads, or exact private paths.
- Do not claim this protocol can trigger any host agent compact control.
- Do not claim this protocol can wake a host runtime that is waiting for a user message.

## Recovery Order

1. Read this handoff.
2. Read `compact-policy.json`.
3. Review Owner Gates and Open Blockers.
4. Read `decision-log.md`.
5. Read `agent-state-ledger.md`.
6. Read `evidence-ledger.md`.
7. Run `resume`.

## Resume Sanity Check

- Current goal is clear.
- Current phase is one allowed task/result status.
- Next safe action is actionable.
- No sensitive material is present.
- If the host returns control after compact, run `aiplus compact resume`.
