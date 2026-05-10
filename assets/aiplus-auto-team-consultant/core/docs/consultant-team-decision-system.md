# Consultant Team Decision System

This document defines the target design for AiPlus Auto Team Consultant.

Auto Team Consultant is not just a set of role prompts. It is a local,
project-specific, AI-native decision system that helps agents decide when to
work alone, when to ask one specialist lens, and when to form a full consultant
team.

## Product Position

Auto Team Consultant should make agents:

- use AI and LLM capabilities more fully;
- make better product, engineering, and release decisions;
- avoid false PASS claims;
- avoid unnecessary Owner interruption;
- preserve Owner gates for dangerous actions;
- keep privacy and security as guardrails, not as the main brake.

The primary value is AI leverage and efficiency. Trust, safety, and privacy are
required guardrails that make higher AI efficiency usable.

## Architecture

The default system is:

```text
Consultant Team Decision System
= 1 Core Product Council
+ 5 Specialist Expert Teams
+ 1 Project-Specific User Evidence Layer
```

### Core Product Council

The Core Product Council is the permanent convergence body.

It runs first and last:

1. At the start, it defines goal, scope, user promise, non-goals, and Owner
   gates.
2. At the end, it reconciles specialist findings and decides
   `PASS`, `NEEDS_FIX`, or `BLOCKED`.

Recommended roles:

- Product CEO / Orchestrator
- Product Advisor
- Technical Architect
- Trust / Safety Lead
- UX / Plain-English Lead

The Core Product Council should not be used for every small task. It is for
direction, conflict resolution, and final convergence.

### Five Specialist Expert Teams

#### 1. Product / Market / Wedge Team

Focus:

- Who is the target user?
- What daily pain does this solve?
- Why would the user use it repeatedly?
- Why would the user pay or authorize more capability?
- What should stay out of scope?
- Is the wedge strong enough?

Typical outputs:

- user promise
- target segment
- value proposition
- daily use case
- willingness-to-pay hypothesis
- non-goals
- future Owner-gated features

#### 2. AI Integration / LLM Experience Team

This team is required by default because AiPlus projects are AI-native.

Focus:

- Where should AI be used?
- Where should AI not be used?
- Does AI create real product differentiation?
- Does the workflow save time or reduce cognitive load?
- Are prompt, context, memory, and tool-use contracts clear?
- Is model routing appropriate for quality, cost, and latency?
- Are hallucination, stale memory, and bad tool-call risks handled?
- Are user control, confirmation, undo, and fallback paths clear?
- Are evals or red-team cases defined for AI behavior?

Typical outputs:

- AI value assessment
- context strategy
- memory strategy
- prompt/tool contract
- model routing recommendation
- cost/latency tradeoff
- user-control boundary
- AI failure and fallback plan

#### 3. UX / Design / Plain-English Team

Focus:

- Can users understand the first screen or first answer?
- Is the main action obvious?
- Is the language simple?
- Are error states actionable?
- Are onboarding, settings, and opt-outs clear?
- Can accessibility users complete the core path?

Typical outputs:

- UX critique
- plain-English copy recommendations
- confusing states
- onboarding risks
- 5-second understanding test setup
- accessibility notes

#### 4. Trust / Safety / Privacy Team

Focus:

- Are permissions and data boundaries clear?
- Could users think the AI will send, delete, publish, or modify content
  unexpectedly?
- Are private data, secrets, raw transcripts, and provider payloads protected?
- Are dangerous actions behind Owner gates?
- Are public/private boundaries preserved?

This team is a guardrail, not a default veto team. It should enable safe AI
efficiency, not suppress useful AI behavior without a concrete boundary risk.

Typical outputs:

- trust boundary
- privacy boundary
- Owner gates
- forbidden actions
- risk register
- safe wording
- redaction expectations

#### 5. Implementation / QA / Release Team

Focus:

- Can this be built safely?
- What is the smallest implementation slice?
- What tests prove the claim?
- Was the source binary tested rather than a stale installed binary?
- What release, hotfix, and rollback checks are required?

Typical outputs:

- implementation plan
- QA matrix
- acceptance criteria
- source-vs-installed evidence
- release-readiness checklist
- hotfix triage
- rollback notes

## Project-Specific User Evidence Layer

The User Evidence Layer is not a decision team. It provides evidence about
whether target users understand, trust, want, and can use the product.

It answers only five categories:

1. Do I understand what this is?
2. Do I know what to do next?
3. Am I worried it will look at, send, modify, delete, publish, or monitor
   things unexpectedly?
4. Would I use it daily?
5. Would I pay for it or authorize stronger capabilities?

### Default Panel Size

Default panel size is 6 agents, but the personas must be project-specific.

For MailCue, the default user agents are:

1. Overwhelmed Professional
2. Apple Mail Loyalist
3. Privacy-Conscious Skeptic
4. Grandma / Low-Tech User
5. AI-Power Early Adopter
6. Accessibility / Assistive-Tech User

Do not hard-code this panel for every project.

Example AiPlus panel:

- Agent Operator
- Developer
- AI Power User
- Release Manager
- Low-Tech Project Owner
- Security Boundary Reviewer

Example AppModules panel:

- Product Founder
- Full-Stack Developer
- AI App Builder
- Skeptical Architect
- Integration Engineer
- Future Maintainer

Example companion-product panel:

- AI Companion User
- Emotionally Sensitive User
- Privacy-Conscious User
- AI Power User
- Accessibility User
- Long-Term Continuity User

### 5-Second Understanding Test

Show the first screen, first concept, or first response for 5 seconds, then ask:

1. What is this?
2. Where would you click next?
3. Will it automatically send or modify anything?
4. Is it monitoring in the background?
5. If you do not want the assistant, pet, widget, or automation, can you turn
   it off?

Pass standard:

- At least 5/6 know what the product is.
- At least 5/6 can identify the main action.
- 6/6 must not believe it will automatically send or modify content.
- 0 users should believe it is secretly monitoring in the background.
- The accessibility persona must be able to complete the core path.

### Forced Re-Review Rules

User Evidence Layer can force expert re-review:

- If a low-tech user cannot understand the main flow, UX must re-review.
- If a privacy/trust persona does not believe the boundary, Trust must
  re-review.
- If any user thinks the product will automatically send, delete, publish, or
  modify private content, the work cannot PASS.
- If the core target user does not find the workflow useful, Product must
  re-review.
- If the AI-power persona is not excited by the AI wedge, Product and AI
  Integration must re-review.
- If the accessibility persona cannot complete the core path, the work cannot
  enter implementation or release.
- If users ask for high-risk AI autonomy, move that request to Future
  Owner-gated scope.

## Install-And-Use Default

Users should not need to manually create a consultant team.

After project install, the project should have a default local configuration:

```text
.aiplus/consultant-team.toml
```

If the file already exists, install/update should preserve it and only add
missing compatible fields when safe.

Agent guidance should tell agents:

```text
Before CEO/review/QA/product/design/release/AI-integration work:
- read .aiplus/consultant-team.toml
- use the configured Consultant Team
- if config is missing or malformed, use the safe AI-native default and report
  NEEDS_FIX for the config
```

Users may later customize the team through natural language, for example:

- "Make this project panel focus on developer-tool users."
- "This project is medical; strengthen Trust, Safety, and Accessibility."
- "This is an AI power-user tool; strengthen AI Integration and efficiency
  review."

## Project Configuration Shape

`consultant-team.toml` is a local routing policy. It is not a permission file
and never grants approval for Owner-gated actions.

Recommended MVP structure:

```toml
schema_version = "0.1"
default_tier = "LIGHT"
max_total_rounds = 5
default_owner_language = "zh-CN"

[project]
name = "Example"
product_type = "AI-native product"
primary_goal = "use AI to improve speed, quality, and user control"
ai_depth = "core_product"
risk_profile = "high_efficiency_with_guardrails"

[priorities]
efficiency = "primary"
ai_leverage = "primary"
user_control = "high"
reliability = "high"
privacy = "guardrail"
security = "guardrail"
accessibility = "required"

[budgets]
light_max_rounds = 1
medium_max_rounds = 3
heavy_max_rounds = 5
hard_stop_on_budget_exceeded = true

[[members]]
id = "ai_integration"
name = "AI Integration / LLM Experience"
default_tiers = ["LIGHT", "MEDIUM", "HEAVY"]
can_edit_files = false
can_trigger_owner_gate = true

[[members]]
id = "trust_safety"
name = "Trust / Privacy / Safety"
default_tiers = ["MEDIUM", "HEAVY"]
can_edit_files = false
can_trigger_owner_gate = true

[[triggers]]
id = "ai_feature"
patterns = ["AI integration", "LLM", "tool use", "memory", "agent autonomy"]
tier = "MEDIUM"
members = ["ai_integration", "trust_safety"]

[[triggers]]
id = "release"
patterns = ["release", "tag", "publish", "artifact"]
tier = "HEAVY"
members = ["release_automation", "trust_safety", "runtime_qa"]
stop_gate = true
```

## Autonomous Trigger Policy

Agents may autonomously invoke consultant workflows for local planning, review,
QA, and docs work. They do not need to ask Owner before using consultant
workflow.

This does not grant permission for dangerous actions.

Still Owner-gated:

- git push
- git tag
- GitHub Release
- artifact upload
- package publish
- deploy
- global config edit
- external account mutation
- secret exposure
- private data upload
- destructive memory/profile migration
- send/delete/publish/mutate external content

### Must-Trigger Situations

At least a consultant check is required when work involves:

- release, hotfix, tag, artifact, publish, or deploy
- memory, compact, profile, identity, or secret broker core behavior
- AI integration, LLM autonomy, tool use, memory, or background behavior
- user-facing product, onboarding, pricing, permissions, or trust copy
- multi-project or subproduct coordination
- secret, private, global config, telemetry, payment, or external account risk
- user asks for CEO, review, QA, brainstorm, multi-agent, or independent review
- previous PASS lacks source/build/test evidence
- source build failure
- installed binary and source mismatch

### Usually Not Needed

Consultant workflow is usually not needed for:

- simple explanation
- one command lookup
- small typo
- single low-risk file change
- user explicitly asks not to use consultant workflow

If the task is non-trivial and consultant workflow is skipped, the agent must
say why.

## Router + Specialist Lenses

Auto Team Consultant should not trigger the whole team by default. It should
route to the smallest useful set of specialist lenses.

### Router Score

For non-trivial tasks, score each dimension from 0 to 3:

```text
complexity_score=0-3
risk_score=0-3
ai_integration_score=0-3
user_impact_score=0-3
uncertainty_score=0-3
```

### Levels

```text
L0 Direct
L1 Self-Check
L2 Single Specialist
L3 Pair Review
L4 Mini Council
L5 Full Council / Owner Gate
```

Recommended thresholds:

```text
L0 Direct: total <= 2 and no single score >= 2
L1 Self-Check: total 3-4
L2 Single Specialist: total 5-7 or any single score = 2
L3 Pair Review: total 8-10 or two scores = 2
L4 Mini Council: total 11-13 or any single score = 3
L5 Full Council / Owner Gate: total >= 14, or publish/release/secret/global
config/external account risk
```

### Specialist Lenses

Use lenses as needed:

- Product / Boundary
- AI Integration / LLM Experience
- Engineering / Architecture
- QA / Regression
- Trust / Privacy / Safety
- UX / User Understanding
- Docs / Onboarding
- Release / Automation
- Strategic Critic
- Process / Orchestration QA

Examples:

- Small docs issue: Docs / Onboarding
- AI prompt issue: AI Integration Reviewer
- Memory behavior change: AI Integration + Trust + QA
- Release: Release + Trust + QA + Core Council
- First-screen product concept: UX + selected User Evidence persona
- Direction conflict: Strategic Critic + Core Product Council

### Lens Limits

Prevent overuse:

- L2: at most 1 specialist
- L3: at most 2 specialists
- L4: at most 4 specialists
- L5: Full Council allowed, Owner gates explicit

No repeated discussion round without new evidence.

Every escalation must state:

```text
why_this_level=...
why_not_lighter=...
```

Every skipped lens must have a reason:

```text
skipped_lenses_with_reason=[...]
```

## User Evidence Granularity

Do not always run a full 6-person panel.

Use:

- Single Persona for small copy, error text, one button, one README paragraph.
- Small Panel for install/onboarding, permission/trust copy, compact/memory/profile
  main flows, or likely misunderstanding.
- Full Panel for public release, major UI prototype, pricing, high-risk AI
  autonomy, send/delete/publish/background monitoring, or Owner request.

## Trigger Accountability

Non-trivial work should include:

```text
ROUTER_PACKET
task_summary:
complexity_score:
risk_score:
ai_integration_score:
user_impact_score:
uncertainty_score:
total_score:
max_score:
level: L0 | L1 | L2 | L3 | L4 | L5
invoked_lenses:
skipped_lenses_with_reason:
owner_gate: yes | no
next_action:
```

Final packets for non-trivial work should include:

```text
CONSULTANT_WORKFLOW_USED=YES|NO
CONSULTANT_WORKFLOW_REASON=...
TRIGGER_ID=...
SELECTED_TIER=L0|L1|L2|L3|L4|L5
MEMBERS_USED=[...]
MEMBERS_SKIPPED_WITH_REASONS=[...]
CONSULTANT_TEAM_CONFIG_USED=YES|NO
CONSULTANT_TEAM_CONFIG_PATH=...
USER_EVIDENCE_LAYER_USED=YES|NO|NOT_APPLICABLE
AUTO_TEAM_TRIGGER_STATUS=PASS|NEEDS_FIX|NOT_APPLICABLE
```

If a must-trigger task skipped consultant workflow without a concrete reason,
review must mark:

```text
AUTO_TEAM_TRIGGER_STATUS=NEEDS_FIX
```

## Result Packet Standard

Every specialist packet should include:

```text
VERDICT=PASS | NEEDS_FIX | BLOCKED
SCOPE=...
TEAM_TYPE=Core Product Council | Product Market Team | AI Integration Team | UX Design Team | Trust Safety Team | Implementation QA Team | User Evidence Layer | Other
FILES_CHANGED=[...]
FILES_CREATED=[...]
FILES_REMOVED=[...]
COMMANDS_RUN=[...]
TESTS_RUN=[...]
SCANS_RUN=[...]
FINDINGS=[...]
REQUIRED_FIXES=[...]
USER_EVIDENCE=[...]
UNVERIFIED_ITEMS=[...]
KNOWN_LIMITATIONS=[...]
SECRET_PRIVATE_BOUNDARY_STATUS=PASS | NEEDS_FIX | BLOCKED
GLOBAL_CONFIG_STATUS=UNTOUCHED | TOUCHED | BLOCKED
TELEMETRY_STATUS=ABSENT | PRESENT | BLOCKED
OWNER_GATES_TRIGGERED=YES | NO
READY_FOR_CORE_COUNCIL=YES | NO
READY_FOR_PLATFORM_CEO=YES | NO
NEXT_RECOMMENDED_ACTION=...
```

QA and review packets must also state:

- source under test
- installed binary used or not
- cwd
- exact commands
- exit status
- reason for each `NOT_RUN`
- evidence type: code, docs, user evidence, design-only, or runtime evidence

## Anti-False-PASS Rules

A PASS is invalid if:

- source does not compile
- only stale installed binary was tested
- command was documented but not run
- tests were not run and no reason was given
- docs say a feature exists but source lacks it
- secret/private/global config scans were skipped for boundary-sensitive work
- release claim lacks artifact/checksum evidence
- simulated specialist lens is presented as real independent agent work
- `READY_FOR_RELEASE_PREP=YES` is claimed while Owner gates remain pending
- User Evidence Layer triggered mandatory re-review and it was ignored
- user-facing flow fails the 5-second understanding test and still claims PASS

## Token-Saving Discipline

Rules:

- Do not paste long logs into chat.
- Summarize and point to files.
- Use concise packets.
- Use tables or checklists for status.
- Avoid repeating full prompts when only the delta matters.
- Prefer compact prepare/checkpoint before long reports.
- Avoid HEAVY workflow for simple tasks.
- Split large tasks in parallel, but require compact final packets.

## Reusable Templates

Auto Team Consultant should provide templates for:

- Consultant Team kickoff prompt
- Core Product Council prompt
- Product / Market Team prompt
- AI Integration Team prompt
- UX / Plain-English Team prompt
- Trust / Safety Team prompt
- Implementation / QA Team prompt
- User Evidence Layer prompt
- 5-second test prompt
- Review prompt
- Fix round prompt
- Release readiness prompt
- Hotfix triage prompt
- Post-release monitoring prompt
- vNext planning prompt
- Subproduct sync prompt

Each template should include:

- goal
- scope
- non-goals
- Owner gates
- required first actions
- workflow tier
- team role
- evidence type
- result packet
- acceptance criteria
- stop conditions

## Runtime Adapter Requirements

Codex, Claude Code, and OpenCode guidance should state:

- Auto Team Consultant is not a background daemon.
- It activates through agent instructions, task classification, and autonomous
  trigger policy.
- Agents may self-trigger consultant workflow for local planning/review/QA/docs.
- Dangerous actions still require Owner approval.
- Real sub-agents may be used only when runtime supports them.
- If no real sub-agent ran, label work as `simulated specialist lens`.
- Do not claim independent review if no independent agent ran.
- Read `.aiplus/consultant-team.toml` before CEO/review/QA/product/design/
  release/AI-integration work.
- For product, UX, AI, trust, pricing, and release work, use this Consultant
  Team Decision System.

## Acceptance Criteria For This System

The system is ready when:

- Core Product Council is defined.
- Five specialist teams are defined.
- AI Integration / LLM Experience is enabled by default.
- Project-Specific User Evidence Layer is defined.
- Default and project-specific user panels are documented.
- 5-second understanding test exists.
- Forced re-review rules exist.
- Install-time default config is documented.
- Agent auto-read behavior is documented.
- Autonomous trigger rights exist.
- Router scoring exists.
- L0-L5 granular levels exist.
- Lens/member limits exist.
- Trigger accountability fields exist.
- Result Packet standard exists.
- Anti-false-PASS rules exist.
- Token-saving discipline exists.
- Runtime adapter requirements are documented.
- Owner gates remain separate from consultant routing.

## Summary

Auto Team Consultant should become:

```text
an install-and-use, project-specific, AI-native consultant decision system
that maximizes AI efficiency while preserving safety guardrails and Owner gates.
```

It should not become:

```text
a rigid set of fixed role prompts or a full-team meeting for every task.
```

The default behavior is: small tasks stay light, medium tasks get the right
specialist lenses, and high-risk tasks escalate with evidence and Owner gates.
