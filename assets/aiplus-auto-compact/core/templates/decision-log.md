# Decision Log

Synthetic template. Replace placeholders before use.

## Protocol Version

0.1.0

## Last Updated

<ISO8601_TIMESTAMP>

## Current Goal

Track decisions needed to resume <REPO_ROOT>.

## Current Phase

IN_PROGRESS

## Completed Work

- Seeded decision log.

## Open Blockers

- None.

## Owner Gates

- UNKNOWN_PENDING: Owner review of first real decision entries.

## Decisions

| ID | Status | Decision | Rationale | Evidence |
| --- | --- | --- | --- | --- |
| DEC-001 | PROVISIONAL | Use local compact files only. | Local compact workflow avoids cloud, database, and automatic UI compact trigger. | EVD-001 |

Allowed status values: DECIDED, PROVISIONAL, REVERSED, NEEDS_VERIFICATION.

## Next 3 Actions

1. Replace synthetic decision with real project decision.
2. Link each decision to evidence.
3. Re-run `validate`.

## Do Not Do

- Do not store private project state, secrets, or provider payloads.

## Recovery Order

1. Read active `DECIDED` and `PROVISIONAL` rows.
2. Verify `NEEDS_VERIFICATION` rows before acting.
3. Treat `REVERSED` rows as historical context only.

## Resume Sanity Check

- Every active decision has a status and evidence ID.
