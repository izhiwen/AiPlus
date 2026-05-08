# Example: CEO Dispatching Agents

Synthetic scenario: A CEO session routes creation of a local workflow package.

```text
task_id: docs-001
workflow_tier: MEDIUM
agent_role: Docs / Onboarding
scope: Draft README and usage docs for a synthetic local workflow kit.
claimed_files: [README.md, docs/usage.md]
allowed_files: [README.md, docs/usage.md]
forbidden_files: [skills/auto-team-consultant/SKILL.md, examples/*]
conflict_rule: Do not edit files outside claimed_files.
inputs: [product definition, public safety disclaimer]
acceptance_criteria: README explains the kit in 5 minutes and includes boundaries.
owner_gate: none for local docs edits
expected_result_packet: result-packet.md
```

```text
task_id: safety-001
status: completed
claimed_files: [SECURITY.md, docs/public-safety-boundaries.md]
changed_files: [SECURITY.md, docs/public-safety-boundaries.md]
commands_run: [synthetic local text scan]
verification_evidence: [Disclaimer present; no guarantee claims found]
risks_or_open_questions: [Owner must still approve any publish or install action]
not_done: [No real security audit performed]
handoff_notes: [Boundaries are documentation-only]
owner_gate_needed: no
recommended_next_action: Integrator should verify against release checklist.
```
