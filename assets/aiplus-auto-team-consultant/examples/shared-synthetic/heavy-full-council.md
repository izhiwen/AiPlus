# Example: HEAVY Rare Escalation

Synthetic scenario: A team is considering public release of an autonomous workflow that can email external collaborators.

```text
CONSULTANT_PACKET
VERDICT=BLOCKED
SESSION_ROLE=CEO
WORKFLOW_LEVEL=HEAVY
INVOKED_LENSES=[Product / Boundary, Process / Orchestration QA, Trust / Privacy / Safety, UX / User Understanding, Implementation / Evidence QA, Release / OSS, User Evidence Pressure-Test]
SKIPPED_LENSES_WITH_REASONS=[Cost / Provider / Vendor: not relevant until the release scope is approved]
PRESSURE_TEST_STATUS=USED
OWNER_GATES_TRIGGERED=YES
KEY_FINDINGS=[External email action requires explicit Owner approval.; Public release cannot proceed from simulated pressure-test alone.; Implementation evidence and rollback plan are missing.]
RISKS=[Unintended external contact.; Misleading release-readiness claims.; Users may misunderstand autonomy boundaries.]
RECOMMENDED_NEXT_ACTION=[Stop before public release, produce an Owner decision packet, and require verified implementation tests.]
CEO_HANDOFF_REQUIRED=NO
RATIONALE=[HEAVY is required because the scenario combines external accounts, autonomy, and public release.]
```
