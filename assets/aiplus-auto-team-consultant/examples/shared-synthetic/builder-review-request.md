# Example: Builder Review Request

Synthetic scenario: A Builder has edited local docs for a fictional workflow and asks for targeted review.

```text
BUILDER_REVIEW_REQUEST
SESSION_ROLE=Builder
WORKFLOW_LEVEL=MEDIUM
CHANGED_FILES=[docs/synthetic-onboarding.md, templates/synthetic-permission-copy.md]
VERIFICATION_RUN=[Checked links manually.; Confirmed examples use placeholder data.]
KNOWN_RISKS=[Permission wording may still be too broad.; Reviewer should confirm no Owner-gated action is implied.]
REVIEW_REQUESTED_FROM=[Product / Boundary, Process / Orchestration QA]
HANDOFF_NOTES=[Please review only the changed synthetic docs and template copy.; Do not treat this request as self-approval of completion.]
OWNER_GATE_NEEDED=NO
```
