# Example: MEDIUM Triggered Cross-Check

Synthetic scenario: A project wants to add an onboarding prompt that asks users to connect a calendar.

```text
CONSULTANT_PACKET
VERDICT=REVISE
SESSION_ROLE=Advisor
WORKFLOW_LEVEL=MEDIUM
INVOKED_LENSES=[Product / Boundary, Trust / Privacy / Safety]
SKIPPED_LENSES_WITH_REASONS=[Full Council: two lenses cover the current risk; Engineering: no implementation requested]
PRESSURE_TEST_STATUS=REQUIRED
OWNER_GATES_TRIGGERED=YES
KEY_FINDINGS=[The prompt asks for a sensitive integration before value is clear.; The copy needs a skip path and plain explanation of what is read.]
RISKS=[Users may think access is mandatory.; Owner approval is needed before connecting external accounts.]
RECOMMENDED_NEXT_ACTION=[Rewrite prompt with value-first copy, clear skip action, and Owner-approved integration scope.]
CEO_HANDOFF_REQUIRED=YES
RATIONALE=[MEDIUM is justified by privacy and external account implications.]
```
