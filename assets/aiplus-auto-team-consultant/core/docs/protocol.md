# Core Protocol

AiPlus Auto Team Consultant is session-local decision-support for choosing expert lenses, routing work at LIGHT/MEDIUM/HEAVY depth, and returning concise packets or handoffs.

The v2.1 reference design is
[`consultant-team-decision-system.md`](consultant-team-decision-system.md). It
defines the project-specific Consultant Team Decision System: 1 Core Product
Council, 5 specialist expert teams, a Project-Specific User Evidence Layer,
Router scoring, L0-L5 routing levels, and trigger accountability.

It supports Advisor, CEO, Reviewer, Builder, and Unknown roles across Codex, Claude Code, and OpenCode adapters.

Already-open sessions should recognize explicit AiPlus refresh triggers:
`AiPlus 刷新`, `刷新 AiPlus`, `aiplus refresh`, `aiplus status`,
`AiPlus status`, `继续 AiPlus`, and `resume AiPlus`. Generic `刷新` /
`refresh` should still try AiPlus first after installation, but explicit
triggers are safer when a project has its own refresh meaning.

## Verdict Vocabulary

- Consultant/advice verdicts: `ACCEPT | REVISE | BLOCKED`
- Review/Gate verdicts: `PASS | REVISE | BLOCKED`
- If a runtime asks for `PASS/NEEDS_FIX`, map `PASS=ACCEPT` for advice and `NEEDS_FIX=REVISE`.

## Required Boundaries

This is not an audit kit, compliance tool, safety guarantee, governance platform, automation product, public-release approval mechanism, autonomous approval system, or real user research substitute.

Owner approval cannot be inferred or simulated.

Consultant routing never grants permission for Owner-gated actions. It only
decides which local planning, review, QA, or documentation lenses should be
used before work continues.
