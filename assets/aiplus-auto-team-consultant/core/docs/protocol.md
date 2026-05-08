# Core Protocol

AiPlus Auto Team Consultant is session-local decision-support for choosing expert lenses, routing work at LIGHT/MEDIUM/HEAVY depth, and returning concise packets or handoffs.

It supports Advisor, CEO, Reviewer, Builder, and Unknown roles across Codex, Claude Code, and OpenCode adapters.

## Verdict Vocabulary

- Consultant/advice verdicts: `ACCEPT | REVISE | BLOCKED`
- Review/Gate verdicts: `PASS | REVISE | BLOCKED`
- If a runtime asks for `PASS/NEEDS_FIX`, map `PASS=ACCEPT` for advice and `NEEDS_FIX=REVISE`.

## Required Boundaries

This is not an audit kit, compliance tool, safety guarantee, governance platform, automation product, public-release approval mechanism, autonomous approval system, or real user research substitute.

Owner approval cannot be inferred or simulated.
