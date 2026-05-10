# Evidence Ledger

Synthetic template. Replace placeholders before use.

## Protocol Version

0.1.0

## Last Updated

<ISO8601_TIMESTAMP>

## Current Goal

Track verification evidence for <REPO_ROOT>.

## Current Phase

IN_PROGRESS

## Completed Work

- Seeded evidence ledger.

## Open Blockers

- None.

## Owner Gates

- UNKNOWN_PENDING: Owner review of evidence quality before first real compact.

## Evidence

| ID | Confidence | Source | Finding | Artifact |
| --- | --- | --- | --- | --- |
| EVD-001 | measured | local validation | `validate` should pass after initialization. | <ARTIFACT_ID> |

Allowed confidence values: official, vendor_blog, github, measured, inferred, unknown.

## Next 3 Actions

1. Replace synthetic evidence with project evidence.
2. Mark uncertain findings as `unknown` or `inferred`.
3. Re-run `checkpoint`.

## Do Not Do

- Do not store raw logs, screenshots containing secrets, transcripts, provider payloads, or exact private paths.

## Recovery Order

1. Prefer measured and official evidence.
2. Recheck inferred and unknown evidence before risky actions.
3. Link evidence back to decisions.

## Resume Sanity Check

- Every evidence row has a confidence value and artifact placeholder.
