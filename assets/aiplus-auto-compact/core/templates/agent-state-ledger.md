# Agent State Ledger

Synthetic template. Replace placeholders before use.

## Protocol Version

0.1.0

## Last Updated

<ISO8601_TIMESTAMP>

## Current Goal

Track delegated agent work for <REPO_ROOT>.

## Current Phase

IN_PROGRESS

## Completed Work

- Seeded agent state ledger.

## Open Blockers

- None.

## Owner Gates

- UNKNOWN_PENDING: Owner review required before relying on delegated results.

## Agents

| Agent | Role | Status | Owned Scope | Last Evidence | Next Action |
| --- | --- | --- | --- | --- | --- |
| synthetic-cli-builder | CLI Builder | done | Rust-native compact commands | EVD-001 | Review generated output. |

Allowed status values: pending, running, blocked, done, abandoned.

## Next 3 Actions

1. Confirm no agents are running before manual compact.
2. Record blockers for any blocked agent.
3. Reassign abandoned work before resume.

## Do Not Do

- Do not include agent transcripts, raw logs, or private paths.

## Recovery Order

1. Resume done work from evidence only.
2. Resolve blocked agents before continuing dependent work.
3. Avoid trusting abandoned work without verification.

## Resume Sanity Check

- Every non-done agent has a next action or blocker.
