# First Action

When the user mentions "aiplus", "AiPlus", or any AiPlus subcommand in this conversation, your first action is to read `.aiplus/AGENTS.aiplus.md` and follow its translation table. Never pass a Chinese phrase as a literal CLI argument — always translate to the English subcommand first.

# Auto Team Consultant Prompt

Use AiPlus Auto Team Consultant as session-local decision-support. Detect role, choose LIGHT/MEDIUM/HEAVY, select lenses, avoid unnecessary Full Council, label pressure-tests, and stop for Owner-gated actions.

## Consultant Team Decision System

Before CEO/review/QA/product/design/release/AI-integration work:
1. Read `.aiplus/consultant-team.toml` if it exists.
2. Use the configured Consultant Team with L0-L5 Router + Specialist Lenses.
3. If config is missing or malformed, use the safe AI-native default and report NEEDS_FIX.

The Consultant Team = 1 Core Product Council + 5 Specialist Expert Teams + Project-Specific User Evidence Layer.

AI Integration / LLM Experience is enabled by default for AI-native products.
Use the smallest useful set of lenses. Do not trigger Full Council for small tasks.

## L0-L5 Router

Score each dimension 0-3: complexity, risk, ai_integration, user_impact, uncertainty.

- L0 Direct: total <= 2 and no single score >= 2
- L1 Self-Check: total 3-4
- L2 Single Specialist: total 5-7 or any single score = 2
- L3 Pair Review: total 8-10 or two scores = 2
- L4 Mini Council: total 11-13 or any single score = 3
- L5 Full Council / Owner Gate: total >= 14, or publish/release/secret/global config/external account risk

Lens limits: L2 max 1 specialist, L3 max 2, L4 max 4, L5 Full Council allowed with Owner gates.

Consultant/advice verdicts use `ACCEPT | REVISE | BLOCKED`. Review/gate verdicts use `PASS | REVISE | BLOCKED`.

v2.1 command routing: compact readiness -> `aiplus compact remind` or `aiplus compact checkpoint`; profile health -> `aiplus profile doctor`; memory health -> `aiplus memory doctor`; profile context -> `aiplus profile context` or `aiplus user context --profile <name>`.
