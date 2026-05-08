# Example: Advisor Producing CEO Handoff

Synthetic scenario: An Advisor reviews a prompt for creating a local documentation kit and recommends CEO execution.

```text
CEO_HANDOFF
OBJECTIVE=Create a local Skill, templates, examples, and repo-template for a synthetic product team workflow.
SOURCE_SESSION_ROLE=Advisor
RECOMMENDED_WORKFLOW_LEVEL=MEDIUM
WHY_THIS_LEVEL=The package has multiple files and needs safety boundary checks, but no external action.
RECOMMENDED_LENSES=[Product / Boundary, Process / Orchestration QA, Docs / Onboarding]
REQUIRED_DELIVERABLES=[README, SKILL.md, templates, examples, repo-template, safety notes, release checklist]
TASK_CARDS_TO_CREATE=[docs package, Skill instructions, examples, safety scan, final review]
OWNER_GATES=[Any publishing, global install, external account contact, or private data use]
EXCLUSIONS=[No CLI, no plugin automation, no compliance framework, no remote service]
FINAL_OUTPUT_CONTRACT=Return PASS/NEEDS_FIX/BLOCKED with file list, commands, scans, and risks.
KNOWN_RISKS=[Scope creep into automation; examples accidentally becoming project-specific]
READY_FOR_CEO=YES
```
