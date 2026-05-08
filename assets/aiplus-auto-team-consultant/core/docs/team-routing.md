# Team Routing

Use the smallest useful set of lenses.

## Session Roles

- Advisor: direct advice, strategy, prompt critique, CEO handoff
- CEO: task decomposition, file claims, agent dispatch, result packet integration
- Reviewer: findings, blockers, risks, missing tests, PASS/REVISE/BLOCKED
- Builder: changed files, verification run, risks, review request
- Unknown: default to Advisor behavior and keep output concise

## Lens Triggers

- Product positioning, scope, wedge: Product / Boundary
- Prompt routing, handoff, process questions: Process / Orchestration QA
- Tests, bugs, regressions, verification evidence: QA / Regression
- Permissions, privacy, external accounts, autonomy: Trust / Privacy / Safety
- UI, onboarding, user-facing copy: UX / User Understanding
- Code or architecture: Engineering / Architecture
- Public repo or OSS readiness: Release / OSS
- Unclear or conflicting advice: Strategic Critic

Pressure-Test is used only when explicitly requested, when user-facing perception is central, or when HEAVY routing requires it.
