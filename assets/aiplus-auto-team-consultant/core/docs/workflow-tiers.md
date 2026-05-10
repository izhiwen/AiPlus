# Workflow Tiers

Default to `LIGHT`.

For the full project-specific consultant architecture, see
[`consultant-team-decision-system.md`](consultant-team-decision-system.md).
This file keeps the short tier model and maps it to the v2.1 L0-L5 router.

## LIGHT

Use for ordinary advice, narrow prompt critique, naming, small docs questions, copy questions, and simple product judgment.

Limits: one round, core judgment or one specialist lens, no Full Council.

Router mapping:

- `L0 Direct`: direct answer, no consultant workflow.
- `L1 Self-Check`: current agent does a quick self-check.
- `L2 Single Specialist`: one specialist lens.

## MEDIUM

Use for formal prompts, product direction review, implementation plans, review/fix cycles, non-trivial architecture choices, or multi-file documentation packages.

Limits: up to three rounds, core plus one or two specialist lenses, Consultant Packet or Result Packet required.

Router mapping:

- `L3 Pair Review`: two specialist lenses, usually one primary and one
  counterweight.

## HEAVY

Use for major product direction, Owner-gated actions, public publication, high-risk autonomy, unresolved conflict, or explicit Owner request for team discussion.

Limits: up to five rounds, selected council only when justified, explicit Owner gates, and pressure-test for central user-facing perception work.

Router mapping:

- `L4 Mini Council`: three or four specialist lenses with explicit
  integration owner.
- `L5 Full Council / Owner Gate`: full council or Owner-gated decision. This
  level does not authorize dangerous actions; it only creates the gate packet
  or final recommendation.
