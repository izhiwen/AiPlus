# Module System Foundation Task Cards

```yaml
task_id: R1-contract-research
workflow_tier: MEDIUM
agent_role: Contract Research Agent
model_hint: inherited
effort_hint: medium
scope: Find active Rust CLI module contract surface and minimal safe extraction points for crates/aiplus-core.
claimed_files: []
allowed_files: [crates/aiplus-cli/src/main.rs, crates/aiplus-cli/tests/*.rs, docs/*.md, MODULES.md]
forbidden_files: [aiplus-work-with-zhiwen private content, any writes]
conflict_rule: read-only task
inputs: Owner protocol and current Rust mainline.
acceptance_criteria: Exact functions/types/constants to move or wrap; behavior contracts; tests impacted.
owner_gate: none
expected_result_packet: Result Packet with findings and recommended next action.
```

```yaml
task_id: R1-asset-adapter-discovery
workflow_tier: MEDIUM
agent_role: Asset/Adapter Agent
model_hint: inherited
effort_hint: medium
scope: Check bundled assets, public subproduct layouts, adapters, managed block, and installed footprint.
claimed_files: []
allowed_files: [assets/**, ../aiplus-compact-reminder/**, ../aiplus-auto-team-consultant/**, ../aiplus-agent-memory/**]
forbidden_files: [aiplus-work-with-zhiwen private content, any writes]
conflict_rule: read-only task
inputs: Module manifest requirements.
acceptance_criteria: Proposed manifest fields for each module; docs/assets sync list; boundary risks.
owner_gate: none
expected_result_packet: Result Packet with proposed manifest content outline.
```

```yaml
task_id: R1-test-qa-discovery
workflow_tier: MEDIUM
agent_role: Test/QA Agent
model_hint: inherited
effort_hint: medium
scope: Map existing tests and missing coverage for core extraction, manifests, rollback, dry-run, unknown versions, and boundaries.
claimed_files: []
allowed_files: [crates/aiplus-cli/tests/**, tests/**, docs/**]
forbidden_files: [secret/private content, any writes]
conflict_rule: read-only task
inputs: Owner QA gates and required commands.
acceptance_criteria: Coverage map, exact tests to add/update, likely command risks.
owner_gate: none
expected_result_packet: Result Packet with recommended QA plan.
```

```yaml
task_id: R2-core-implementation
workflow_tier: MEDIUM
agent_role: Rust Implementation Agent
model_hint: inherited
effort_hint: high
scope: Add crates/aiplus-core and wire aiplus-cli to core module registry, asset access, manifest structs, and rollback plan structs.
claimed_files:
  - Cargo.toml
  - crates/aiplus-core/**
  - crates/aiplus-cli/Cargo.toml
  - crates/aiplus-cli/src/main.rs
  - crates/aiplus-cli/tests/parity.rs
allowed_files: same as claimed_files
forbidden_files: [install.sh, release artifacts, private repos, global config]
conflict_rule: if another worker touches claimed files, stop and hand back to CEO.
inputs: Module Contract Plan and R1 Result Packets.
acceptance_criteria: aiplus-core builds and is tested; CLI behavior markers remain stable; rollback dry-run command exists.
owner_gate: none for local edits
expected_result_packet: Result Packet with changed files and verification.
```

```yaml
task_id: R2-docs-module-manifests
workflow_tier: MEDIUM
agent_role: Docs/Template Agent
model_hint: inherited
effort_hint: medium
scope: Add module manifests/schema and sync public docs/subproduct docs only as needed for contract consistency.
claimed_files:
  - crates/aiplus-core/schemas/aiplus-module.schema.json
  - assets/*/aiplus-module.json
  - MODULES.md
  - docs/module-contract-plan.md
  - ../aiplus-compact-reminder/aiplus-module.json
  - ../aiplus-auto-team-consultant/aiplus-module.json
  - ../aiplus-agent-memory/aiplus-module.json
allowed_files: same as claimed_files plus README/CHANGELOG/RELEASE_CHECKLIST if needed
forbidden_files: [private aiplus-work-with-zhiwen content, release upload files]
conflict_rule: if overlapping code files are touched, stop and return blocked.
inputs: Module manifest requirements.
acceptance_criteria: Manifests exist for all three dogfood modules and validate through core tests.
owner_gate: none for local docs/assets
expected_result_packet: Result Packet with sync status.
```

```yaml
task_id: R3-review-qa
workflow_tier: MEDIUM
agent_role: Reviewer/QA Agent
model_hint: inherited
effort_hint: medium
scope: Review final local changes and run targeted verification gates.
claimed_files: []
allowed_files: read-only full public workspace
forbidden_files: [private content extraction, writes, release uploads]
conflict_rule: no file edits
inputs: Final diff and required QA commands.
acceptance_criteria: Findings first; PASS/REVISE/BLOCKED per gate; exact missing fixes if any.
owner_gate: release actions remain gated
expected_result_packet: Result Packet with gate evidence.
```
