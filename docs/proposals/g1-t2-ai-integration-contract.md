# G1 T2 AI Integration Contract

task_id: T2-ai-integration
owner: ai-integration
status: recommended_for_T3_T4
scope: agent-facing contract, spec notes, test expectations

## Decision

Use `identity context` as the role entrypoint, with an opt-in constrained memory
bundle. Keep separate `memory` commands as the canonical memory inspection and
mutation API.

Do not make role identity implicitly load memory by default. The default
behavior stays role-only for compatibility, token control, and clear separation
between "who am I acting as?" and "what context am I allowed to consider?".

Recommended session-start command:

```bash
aiplus identity context --role <role> --runtime <codex|claude-code|opencode> --with-memory --memory-budget 2000
```

Canonical separate memory commands remain:

```bash
aiplus memory status
aiplus memory context --runtime <runtime> --budget 2000 --scope project
aiplus memory show-used
aiplus memory add --scope project --kind preference --text "..."
aiplus memory forget <id>
```

## Contract

### `identity context`

Required shape:

```text
IDENTITY_CONTEXT
role=<role>
role_name=<display name>
runtime=<runtime>
activation=[...]
output_contract=<short contract>
owner_gates=[...]
permissions=none
scope=<identity source scope>
identity_grants_permission=no
memory_bundle=<none|included>
global_agent_config_edits=none
IDENTITY_CONTEXT_STATUS=PASS
```

Default behavior:

- `memory_bundle=none`.
- No memory records are printed.
- Output remains compatible with current role-only tests.

Opt-in bundle flags:

- `--with-memory`: include a memory capsule after the role contract.
- `--memory-budget <chars>`: character budget for the memory capsule, default
  `2000`, hard cap `4000` unless a future owner-gated override is added.
- `--runtime <codex|claude-code|opencode>`: affects formatting only. Selection
  semantics must be runtime-neutral.
- `--memory-scope <auto|role-personal|team|project|all>`: default `auto`.
  `auto` applies the role policy below.

Required bundle metadata:

```text
MEMORY_BUNDLE
runtime=<runtime>
memory_budget=<chars>
memory_records_used=<n>
memory_records_ignored=<n>
role_personal_total=<n>
role_personal_used=<n>
team_total=<n>
team_used=<n>
project_total=<n>
project_used=<n>
secret_values=none
memory_is_instruction=no
MEMORY_BUNDLE_STATUS=PASS
```

The memory records section must be grouped in this order:

1. Owner gates
2. Role-personal context
3. Team coordination context
4. Project context
5. Verification or handoff evidence

### Memory Selection

Eligible records:

- `status` is `active` or `tentative`.
- Not expired and not stale.
- Redaction scan passes, otherwise the record summary is replaced with
  `[REDACTED]` and counted as ignored for the printable bundle.
- Deduplicate by `(normalized summary, record_type)`, preferring the newest
  record.

Priority order within a budget:

1. `owner_gate`
2. `project_decision`
3. `risk`
4. `workflow_rule`
5. `owner_preference`
6. `handoff_note`
7. `project_fact`
8. `verification_evidence`
9. `role_identity`
10. `skill_candidate`

The bundle must stop at the character budget and report ignored counts. It must
not silently exceed the budget for any runtime.

### Scope Buckets

The bundle reports three count buckets even if the backing store uses different
internal scopes today:

- `role-personal`: profile or role-scoped preferences and identity-adjacent
  context relevant to the active role.
- `team`: coordination state, task routing, handoffs, owner gates, and active
  role interactions that a coordinator needs to staff work.
- `project`: repo-local decisions, facts, workflow rules, risks, and evidence.

This is an output contract. T3 may initially derive buckets from existing
`scope`, `record_type`, `subject`, and `tags`; it does not require a storage
schema migration for T2.

### Role Defaults

Coordinator roles:

- Applies to `ceo`, `advisor`, and future coordinator-like roles.
- `--memory-scope auto` includes role-personal, team, and project buckets.
- Budget split target: owner gates first, then up to 25 percent role-personal,
  up to 35 percent team, remaining project.
- Include active task ownership, routing constraints, owner gates, decisions,
  risks, and handoff notes.

Builder roles:

- Applies to `builder`, `engineer-a`, `engineer-b`, and future implementer-like
  roles.
- `--memory-scope auto` includes role-personal and project buckets, plus only
  team handoff records directly relevant to the assigned task.
- Budget split target: owner gates first, then up to 20 percent role-personal,
  up to 10 percent targeted team handoff, remaining project.
- Exclude unrelated staffing chatter, broad team state, and reviewer-private
  notes unless explicitly requested by a coordinator.

Reviewer roles:

- Include owner gates, project decisions, risks, acceptance criteria, and
  verification evidence.
- Avoid builder-private implementation notes unless they have been promoted to
  project memory or attached as review evidence.

## Current CLI Mismatch

Observed source behavior on 2026-05-15:

- `memory context` parses `--scope`, but `command_memory()` calls
  `memory_context(args.runtime, args.budget)` and drops `args.scope`.
- `memory_context()` selects from all active records by budget and does not
  filter by scope.
- `identity context` accepts only `--role`; it emits role metadata and no memory
  bundle.
- `PROFILE_BUNDLE_PLAN.md` says identity should try project identity first and
  fall back to installed profile identity, but current `read_identity()` reads
  only `.aiplus/identities/<role>.identity.toml` after `identity_init()`.

These are engineer-a implementation tasks. T2 does not edit Rust source.

## Test Expectations

T3 implementation tests should cover:

1. `identity context --role ceo` remains role-only and reports
   `memory_bundle=none`.
2. `identity context --role ceo --with-memory --memory-budget 2000` reports
   `MEMORY_BUNDLE`, used/ignored counts, and no secret values.
3. Bundle output never exceeds the requested memory budget, apart from fixed
   metadata lines.
4. Coordinator auto scope includes team coordination records before generic
   project facts when both fit.
5. Builder auto scope excludes unrelated team coordination records and includes
   project workflow rules and assigned handoff notes.
6. Reviewer auto scope includes decisions, risks, acceptance criteria, and
   evidence, but excludes builder-private notes.
7. `memory context --scope project` returns only project bucket records.
8. `memory context --scope role-personal` returns only role-personal bucket
   records or a clear zero-record PASS if none exist.
9. Redaction-sensitive memory summaries are not printed in either standalone
   memory context or identity-bundled memory context.
10. Runtime formatting for `codex`, `claude-code`, and `opencode` preserves the
    same selection counts for the same input fixture.

T4 adapter or prompt tests should cover:

1. Natural language "new CEO" maps to role-only identity by default unless the
   adapter/session-start policy requests `--with-memory`.
2. Session-start prompts may use the bundled command for ergonomics, but
   "what memories were used?" maps to `aiplus memory show-used` or standalone
   `memory context`.
3. Memory writes, forgets, profile sync, and skill candidates remain separate
   commands and never ride through `identity context`.

## Validation Matrix

G1 deliverables:

- D1 decision: Pass. Recommend identity entrypoint with opt-in memory bundle,
  separate memory commands as canonical.
- D2 agent-facing contract: Pass. Command shape, output fields, scope buckets,
  role defaults, and tests are specified here.
- D3 token caps and counts: Pass. `--memory-budget`, hard cap, used/ignored
  counts, and bucket counts are required.
- D4 coordinator vs builder scopes: Pass. Role defaults distinguish
  coordinator, builder, and reviewer memory exposure.
- D5 current CLI mismatch: Pass. Known source gaps are listed without editing
  engineer-a-owned implementation.
- D6 cross-runtime ergonomics: Pass. Runtime flag is formatting-only and tests
  require equivalent selection across Codex, Claude Code, and OpenCode.

Requirements:

- R1 no implementation ownership breach: Pass. No Rust source edits.
- R2 no T3 runtime prompt or managed-block edits: Pass.
- R3 owner gates preserved: Pass. Identity grants no permission; memory writes
  and broad overrides remain separate.
- R4 safety and privacy: Pass. Redaction, no secret values, no external action,
  no global config edits.
- R5 compatibility: Pass. Role-only identity remains default.
- R6 testability: Pass. Fixture-oriented test expectations are enumerated.

## Risks and Open Questions

- The `role-personal` and `team` buckets are not first-class current memory
  scopes. T3 should derive them without schema migration unless engineer-a
  decides a storage change is necessary.
- The exact profile identity fallback behavior belongs to implementation and
  may need owner review if it reads private profile files in public-release
  workflows.
- A future owner-gated `--memory-budget >4000` override could be useful for
  large-context models, but it should not be part of G1.

## Recommended Next Action

Engineer-a should implement the compatibility-preserving flag path:

1. Wire `memory context --scope`.
2. Add reusable memory selection with bucket counts.
3. Add `identity context --with-memory --memory-budget --runtime --memory-scope`.
4. Add fixtures for coordinator, builder, reviewer, and runtime-equivalence
   cases.
