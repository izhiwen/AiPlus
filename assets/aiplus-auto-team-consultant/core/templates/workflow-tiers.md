# Workflow Tiers Template

```text
DEFAULT=LIGHT
SELECTED=LIGHT | MEDIUM | HEAVY
ESCALATION_TRIGGER=
MAX_ROUNDS=1 | 3 | 5
REQUIRED_OUTPUT=direct advice | Consultant Packet | Result Packet | CEO Handoff | Gate Packet
```

## LIGHT

Use for ordinary advice, narrow prompt critique, naming, small docs questions, and simple product judgment.

Limits: one round, core judgment or one specialist lens, no Full Council.

## MEDIUM

Use for formal CEO prompts, product direction review, implementation plans, review/fix cycles, non-trivial architecture choices, or multi-file documentation packages.

Limits: up to three rounds, core plus one or two specialist lenses, Consultant Packet or Result Packet required.

## HEAVY

Use for major product direction, safety boundaries, external accounts, deployment, public release, high-risk autonomy, unresolved conflict, or Owner-requested team discussion.

Limits: up to five rounds, explicit Owner gates, Full Council only when justified, pressure-test required for user-facing perception.
