# Team Architecture Template

Define only the lenses needed for the session.

For the full v2.1 structure, see
[`../docs/consultant-team-decision-system.md`](../docs/consultant-team-decision-system.md).

## Core Council

- Product / Boundary:
- AI Integration / LLM Experience:
- Process / Orchestration QA:
- Strategic Critic:
- Trust / Privacy / Safety:
- UX / User Understanding:
- Implementation / Evidence QA:

## Specialist Lenses

- Product / Market / Wedge:
- AI Integration / LLM Experience:
- Design / Figma / Motion:
- Engineering / Architecture:
- QA / Regression:
- Market / Positioning:
- Docs / Onboarding:
- Security / Privacy:
- Release / OSS:
- Cost / Provider / Vendor:
- User Evidence Pressure-Test:

## Project-Specific User Evidence Layer

- Target user panel:
- Single persona needed:
- Small panel needed:
- Full panel needed:
- 5-second understanding test needed:
- Forced re-review trigger:

## Router Packet

```text
task_summary:
complexity_score:
risk_score:
ai_integration_score:
user_impact_score:
uncertainty_score:
total_score:
level: L0 | L1 | L2 | L3 | L4 | L5
invoked_lenses:
skipped_lenses_with_reason:
owner_gate: yes | no
next_action:
```

## Selection Rule

Use the smallest useful set. Full Council requires HEAVY tier, explicit justification, and a maximum of five rounds.
