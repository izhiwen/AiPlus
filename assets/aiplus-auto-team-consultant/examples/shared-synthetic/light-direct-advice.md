# Example: LIGHT Direct Advice

Synthetic scenario: A team asks whether a settings page label should be "Controls" or "Preferences" for a small prototype.

```text
CONSULTANT_PACKET
VERDICT=ACCEPT
SESSION_ROLE=Advisor
WORKFLOW_LEVEL=LIGHT
INVOKED_LENSES=[UX / User Understanding]
SKIPPED_LENSES_WITH_REASONS=[Full Council: low-risk naming decision; Trust / Safety: no permissions or privacy change]
PRESSURE_TEST_STATUS=SKIPPED_WITH_REASON: label choice is simple and reversible
OWNER_GATES_TRIGGERED=NO
KEY_FINDINGS=[Use "Preferences" because it better matches user expectation for configurable defaults.]
RISKS=[If the product uses "Controls" elsewhere for live operations, check consistency.]
RECOMMENDED_NEXT_ACTION=[Use "Preferences" in the prototype and revisit only if usability feedback conflicts.]
CEO_HANDOFF_REQUIRED=NO
RATIONALE=[LIGHT is enough because this is a narrow, low-risk wording decision.]
```
