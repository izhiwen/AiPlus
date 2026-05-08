# Consultant Packet Template

```text
CONSULTANT_PACKET
VERDICT=ACCEPT | REVISE | BLOCKED
SESSION_ROLE=Advisor | CEO | Reviewer | Builder | Unknown
WORKFLOW_LEVEL=LIGHT | MEDIUM | HEAVY
INVOKED_LENSES=[...]
SKIPPED_LENSES_WITH_REASONS=[...]
PRESSURE_TEST_STATUS=USED | SKIPPED_WITH_REASON | REQUIRED | N/A
OWNER_GATES_TRIGGERED=YES | NO
KEY_FINDINGS=[...]
RISKS=[...]
RECOMMENDED_NEXT_ACTION=[...]
CEO_HANDOFF_REQUIRED=YES | NO
RATIONALE=[...]
```

Use concise findings. Do not treat this packet as final approval for Owner-gated actions.

Verdict rule: Consultant/advice verdicts use `ACCEPT | REVISE | BLOCKED`. Review/Gate verdicts use `PASS | REVISE | BLOCKED`.
