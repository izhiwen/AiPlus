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
