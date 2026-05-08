# Decision Log

Fixture decision log for compactctl acceptance tests.

## Protocol Version

0.1.0

## Last Updated

2026-01-01T00:00:00.000Z

## Current Goal

Track fixture decisions.

## Current Phase

IN_PROGRESS

## Completed Work

- Seeded fixture decision.

## Open Blockers

- None.

## Owner Gates

- APPROVED: Fixture decision gate is approved for local tests.

## Decisions

| ID | Status | Decision | Rationale | Evidence |
| --- | --- | --- | --- | --- |
| DEC-001 | DECIDED | Use local fixture files. | Acceptance tests need deterministic state. | EVD-001 |

## Next 3 Actions

1. Validate fixture decision log.
2. Keep evidence linked.
3. Keep statuses allowed.

## Do Not Do

- Do not store private project state.

## Recovery Order

1. Read decided rows.
2. Verify evidence links.
3. Continue from current handoff.
