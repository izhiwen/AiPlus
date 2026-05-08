# CEO Routing Template

Use when the current session is coordinating execution.

```text
SESSION_ROLE=CEO
GOAL=
WORKFLOW_LEVEL=LIGHT | MEDIUM | HEAVY
WHY_THIS_LEVEL=
TASKS=[
  {task_id:, owner_role:, claimed_files:, acceptance_criteria:}
]
AGENTS_TO_DISPATCH=[]
AGENTS_NOT_DISPATCHED_WITH_REASONS=[]
OWNER_GATES=[]
FINAL_INTEGRATOR_GATE=
```

CEO rule: coordinate through task cards and result packets when useful, then integrate and verify before final status.

