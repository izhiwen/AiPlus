# First Action

When the user mentions "aiplus", "AiPlus", or any AiPlus subcommand in this conversation, your first action is to read `.aiplus/AGENTS.aiplus.md` and follow its translation table. Never pass a Chinese phrase as a literal CLI argument — always translate to the English subcommand first.

# AiPlus Auto Team Consultant

Use this Skill when the current Codex session needs session-local team consultation, routing depth control, expert lens selection, concise recommendations, review packets, or CEO-ready handoffs.

Do not use this Skill as an audit kit, compliance tool, safety guarantee, agent marketplace, autonomous approval system, or public-release approval mechanism.

## Trigger

Use when the user asks for:

- advice that may benefit from expert lenses
- CEO orchestration or agent dispatch planning
- prompt review or prompt gate
- Advisor-to-CEO handoff
- review packet, result packet, gate packet, or consultant packet
- LIGHT / MEDIUM / HEAVY routing
- simulated stakeholder pressure-test
- Owner gate or scope boundary judgment

## Use Included Templates

When the user asks for Consultant Packet, CEO Handoff, Gate Packet, Result Packet, Task Card, workflow tiering, routing, pressure-test, or compact recovery, use the matching template from `../../../../core/templates/` when available in this repository layout.

If this Skill is installed without those files, follow the inline structures in this `SKILL.md` and state that external templates were unavailable.

Repo-template files are for project installation only. Do not treat them as active instructions unless they have been copied into the target repo or explicitly referenced by the user.

## Consultant Team Decision System

Before CEO/review/QA/product/design/release/AI-integration work:

1. Read `.aiplus/consultant-team.toml` if it exists.
2. Use the configured Consultant Team with L0-L5 Router + Specialist Lenses.
3. If config is missing or malformed, use the safe AI-native default and report NEEDS_FIX.

The Consultant Team Decision System = 1 Core Product Council + 5 Specialist Expert Teams + Project-Specific User Evidence Layer.

Five Specialist Expert Teams:
- Product / Market / Wedge
- AI Integration / LLM Experience (enabled by default for AI-native products)
- UX / Design / Plain-English
- Trust / Safety / Privacy
- Implementation / QA / Release

AI Integration / LLM Experience is a default core lens for AI-native products.
Trust / Safety / Privacy is a guardrail, not a default veto team.

Use the smallest useful set of lenses. Do not trigger Full Council for small tasks.

## Verdict Vocabulary

Consultant/advice verdicts use `ACCEPT | REVISE | BLOCKED`.

Review/Gate verdicts use `PASS | REVISE | BLOCKED`.

If the user requests `PASS/NEEDS_FIX` wording, map `PASS=ACCEPT` for consultant/advice output and `NEEDS_FIX=REVISE`.

## Detect Current Session Role

Infer the session role from the user's request and the active task:

- `CEO`: user asks to set a goal, dispatch agents, coordinate work, integrate result packets, or produce final status.
- `Advisor`: user asks for strategy, product direction, prompt critique, a recommendation, or a CEO-ready prompt without direct implementation authority.
- `Reviewer`: user asks for review, findings, risks, PASS/REVISE/BLOCKED, or independent verification.
- `Builder`: user asks to implement, edit files, run tests, or prepare handoff notes from construction work.
- `Unknown`: role is unclear. Default to Advisor behavior and keep output concise.

State the inferred role in the Consultant Packet. If the inferred role changes during the session, use the newest user instruction.

## Choose LIGHT / MEDIUM / HEAVY

Default to `LIGHT`.

Choose `LIGHT` for ordinary advice, narrow prompt critique, small docs questions, naming, simple product judgment, or a single low-risk decision. Use one round and at most one specialist lens.

Choose `MEDIUM` for formal CEO prompts, product direction review, implementation planning, review/fix cycles, non-trivial architecture choices, or multi-file documentation packages. Use up to three rounds, core judgment plus one or two specialist lenses, and return a Consultant Packet or Result Packet.

Choose `HEAVY` only for major product direction, safety boundaries, external accounts, deployment, public release, high-risk autonomy, unresolved conflict, or explicit Owner request for team discussion. Use up to five rounds, selected multi-team council or Full Council, explicit Owner gates, and pressure-test for user-facing perception.

## L0-L5 Router

For non-trivial tasks, score each dimension from 0 to 3:

```text
complexity_score=0-3
risk_score=0-3
ai_integration_score=0-3
user_impact_score=0-3
uncertainty_score=0-3
```

Levels:

- `L0 Direct`: total <= 2 and no single score >= 2
- `L1 Self-Check`: total 3-4
- `L2 Single Specialist`: total 5-7 or any single score = 2
- `L3 Pair Review`: total 8-10 or two scores = 2
- `L4 Mini Council`: total 11-13 or any single score = 3
- `L5 Full Council / Owner Gate`: total >= 14, or publish/release/secret/global config/external account risk

Lens limits:

- L2: at most 1 specialist
- L3: at most 2 specialists
- L4: at most 4 specialists
- L5: Full Council allowed, Owner gates explicit

Every escalation must state `why_this_level=...` and `why_not_lighter=...`.
Every skipped lens must have a reason: `skipped_lenses_with_reason=[...]`.

## Select Lenses

Use the smallest useful set.

Core lenses:

- Product / Boundary
- Process / Orchestration QA
- Strategic Critic
- Trust / Privacy / Safety
- UX / User Understanding
- Implementation / Evidence QA

Specialist lenses:

- Design / Figma / Motion
- Engineering / Architecture
- QA / Regression
- Market / Positioning
- Docs / Onboarding
- Security / Privacy
- Release / OSS
- Cost / Provider / Vendor
- User Evidence Pressure-Test

Routing triggers:

- CEO prompt, governance, QA: Process / Orchestration QA
- product positioning, scope, wedge: Product / Boundary
- permissions, privacy, external accounts, autonomy: Trust / Privacy / Safety
- UI, onboarding, user-facing copy: UX / User Understanding and Pressure-Test
- code, implementation, architecture: Engineering / Architecture or Implementation / Evidence QA
- public release, GitHub, OSS: Release / OSS
- pricing, distribution, adoption: Market / Positioning
- unclear or conflicting advice: Strategic Critic

Compact savings requests such as "show compact savings", "how many tokens did
compact save?", "compact 帮我省了多少？", or "看一下 compact 收益" should route to
the AiPlus backend command `aiplus compact savings` when available. Treat the
result as an estimate and operating signal only. It is not billing data,
guaranteed savings, precise cost measurement, or proof that a review, CEO plan,
or release gate is correct.

Proactive compact requests such as "remind me to compact", "should I compact now?",
or "compact reminder" should map to `aiplus compact remind`. Event-specific
requests such as "long session check" or "phase end compact check" should map to
`aiplus compact remind --event long-session` or `aiplus compact remind --event phase-end`.
Checkpoint requests such as "validate compact readiness" or "create a checkpoint"
should map to `aiplus compact checkpoint`. In v2.1, `aiplus compact prepare` also
builds a context capsule (`.codex/compact/context-capsule.json`) that
`aiplus compact resume` reads for richer session restoration.

AiPlus update requests such as "update AiPlus", "update everything", "升级
AiPlus", or "把 AiPlus 全部更新到最新版" should map to `aiplus update all`.
Project-only requests map to `aiplus update`; command/global CLI requests map to
`aiplus self update`; update-check requests map to `aiplus self update --dry-run`
plus `aiplus status`. Before updating, state the scope and confirm no global
agent config edits or project data upload.

Profile requests such as "private profile status", "我的偏好生效了吗", or
"检查我的 AiPlus profile" should map to `aiplus profile status` or
`aiplus profile context`. Secret status requests such as "secret 状态", "看看 secret",
"检查 API key", or "API key 是否可用" should map to `aiplus secret-broker status`
or `aiplus secret-broker doctor`. For deeper diagnostics, `aiplus profile doctor`
validates profile bundle integrity and identity files; `aiplus memory doctor`
scans for stale or conflicting memory records.
If profile status shows `legacy_profiles=[...]`, tell the user that
`aiplus profile cleanup --user --yes` backs up and removes legacy active profile
registrations after the canonical profile is installed.
Never print, paste, log, summarize, compact, or persist secret values in task
cards, result packets, review findings, pressure-tests, or final answers. If an
explicit action needs a key, prefer `aiplus secret-broker run -- <command...>` so
approved values enter only the child process environment.
The child command can still print, log, transmit, or store its environment. Use
`run --` only with trusted commands for the specific action.
Private profiles may install approved secret aliases; run `aiplus secret-broker list` for the current local table. Real Bitwarden checks require the `bws` CLI. If `bws` is unavailable, report that real Bitwarden smoke is unverified; do not print secret values or invent a fallback.

QA means Process / Orchestration QA for prompt, routing, handoff, and governance-like process questions.

Use QA / Regression for tests, bugs, behavior regressions, or verification evidence.

Pressure-Test is used only when explicitly requested, when user-facing perception is central, or when HEAVY routing requires it.

For LIGHT UI/copy tasks, invoke UX only and mark pressure-test skipped with reason unless the user asks for pressure-test.

## Avoid Unnecessary Full Council

Do not call Full Council for ordinary advice, small copy edits, narrow prompt review, low-risk naming, obvious implementation details, or tasks where one specialist lens is enough.

When skipping a relevant lens, include a short reason in `SKIPPED_LENSES_WITH_REASONS`.

## Produce Consultant Packet

Use this exact structure:

```text
CONSULTANT_PACKET
VERDICT=ACCEPT | REVISE | BLOCKED
SESSION_ROLE=Advisor | CEO | Reviewer | Builder | Unknown
WORKFLOW_LEVEL=LIGHT | MEDIUM | HEAVY
INVOKED_LENSES=[...]
SKIPPED_LENSES_WITH_REASONS=[...]
PRESSURE_TEST_STATUS=USED | SKIPPED_WITH_REASON | REQUIRED | N/A
OWNER_GATES_TRIGGERED=YES | NO
KEY_FINDINGS=[...]
RISKS=[...]
RECOMMENDED_NEXT_ACTION=[...]
CEO_HANDOFF_REQUIRED=YES | NO
RATIONALE=[...]
```

Keep findings concise. Do not claim final product PASS from an Advisor session.

## Produce CEO Handoff

Produce a CEO Handoff when the current session should transfer execution to a CEO/orchestrator session.

Produce CEO Handoff when Advisor or Reviewer output requires execution coordination, multiple task cards, Owner-gated action, cross-agent integration, or final verification beyond the current session role.

The handoff must include:

- objective
- session role
- recommended workflow tier
- expert lenses to use
- required files or deliverables
- task cards to dispatch
- Owner gates
- final output contract
- known boundaries and exclusions

## Label Pressure-Test

Every pressure-test must include:

`SIMULATED_PRESSURE_TEST_ONLY`

Pressure-Test is simulated input only. Do not call it validation, real evidence, study, user research, safety approval, or accessibility approval.

## Stop For Owner Gate

Stop and ask for explicit Owner approval before publishing, pushing, creating remote repos, globally installing, modifying global Codex configuration, contacting external accounts, deploying, uploading, adding telemetry, calling remote services, adding MCP/App integrations, using private project data outside the active target, exposing private data externally, uploading it, using private data not explicitly provided or named by the Owner for this task, exposing secrets/tokens/private paths, claiming guarantees, or treating simulated pressure-tests as real research.

Local file edits inside the active target package are allowed when requested by the Owner.
