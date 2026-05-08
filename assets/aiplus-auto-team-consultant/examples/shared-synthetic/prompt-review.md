# Example: Prompt Review

Synthetic scenario: A user asks whether a CEO prompt is ready for execution.

```text
GATE_PACKET
VERDICT=REVISE
TEMPLATE_USED=YES
PROMPT_GATE_RESULT=FAIL
WORKFLOW_LEVEL=MEDIUM
INVOKED_LENSES=[Product / Boundary, Process / Orchestration QA]
SKIPPED_LENSES_WITH_REASONS=[Full Council: prompt has scope issues but no public release or external account action]
PRESSURE_TEST_REQUIRED=NO
BOUNDARY_FLAGS=NONE_FOUND
OWNER_GATES_TRIGGERED=NO
PRIVATE_DATA_FLAGS=NONE_FOUND
COMPACT_RECOVERY_INCLUDED=NO
BLOCKERS=[]
REQUIRED_FIXES=[Add exact target directory; exclude CLI automation; specify final output contract]
RATIONALE=[The CEO can execute after these prompt gaps are fixed.]
```
