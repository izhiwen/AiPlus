# Evidence Ledger

Fixture evidence ledger for compactctl acceptance tests.

## Protocol Version

0.1.0

## Last Updated

2026-01-01T00:00:00.000Z

## Current Goal

Track fixture evidence.

## Current Phase

IN_PROGRESS

## Completed Work

- Seeded fixture evidence.

## Open Blockers

- None.

## Owner Gates

- APPROVED: Fixture evidence gate is approved for local tests.

## Evidence

| ID | Confidence | Source | Finding | Artifact |
| --- | --- | --- | --- | --- |
| EVD-001 | measured | local test fixture | compactctl can validate this fixture. | ARTIFACT-001 |

## Next 3 Actions

1. Validate fixture evidence ledger.
2. Keep confidence values allowed.
3. Continue from handoff.

## Do Not Do

- Do not store transcripts or provider payloads.

## Recovery Order

1. Prefer measured evidence.
2. Recheck inferred evidence.
3. Link evidence to decisions.
