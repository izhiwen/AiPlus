# First Action

When the user mentions "aiplus", "AiPlus", or any AiPlus subcommand in this conversation, your first action is to read `.aiplus/AGENTS.aiplus.md` and follow its translation table. Never pass a Chinese phrase as a literal CLI argument — always translate to the English subcommand first.

# AiPlus Auto Team Consultant For Claude Code

Use this project-local skill when a Claude Code session needs session-local team routing, expert lens selection, Advisor handoff, CEO routing, review findings, Builder review request, or simulated pressure-test output.

Use shared core templates from `../../../../core/templates/` when available in this repository layout. If unavailable, follow the structures in the command files.

## Consultant Team Decision System

Before CEO/review/QA/product/design/release/AI-integration work:
1. Read `.aiplus/consultant-team.toml` if it exists.
2. Use the configured Consultant Team with L0-L5 Router + Specialist Lenses.
3. If config is missing or malformed, use the safe AI-native default and report NEEDS_FIX.

The Consultant Team = 1 Core Product Council + 5 Specialist Expert Teams + Project-Specific User Evidence Layer.

AI Integration / LLM Experience is enabled by default for AI-native products.
Use the smallest useful set of lenses. Do not trigger Full Council for small tasks.

L0-L5 Router:
- L0 Direct: total <= 2 and no single score >= 2
- L1 Self-Check: total 3-4
- L2 Single Specialist: total 5-7 or any single score = 2
- L3 Pair Review: total 8-10 or two scores = 2
- L4 Mini Council: total 11-13 or any single score = 3
- L5 Full Council / Owner Gate: total >= 14, or publish/release/secret/global config/external account risk

Consultant/advice verdicts use `ACCEPT | REVISE | BLOCKED`.
Review/Gate verdicts use `PASS | REVISE | BLOCKED`.

Proactive compact: route "should I compact now?" to `aiplus compact remind` and
"validate compact readiness" to `aiplus compact checkpoint`. In v2.1,
`aiplus compact prepare` builds a context capsule for richer resume.

Diagnostic routing: "profile health" -> `aiplus profile doctor`; "memory health"
-> `aiplus memory doctor`; "profile context" -> `aiplus profile context`.

Pressure-Test output must include `SIMULATED_PRESSURE_TEST_ONLY`.

Stop for Owner approval before publish, push, tag, release, global install, external accounts, telemetry, upload, remote service calls, or private data exposure.
