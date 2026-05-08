# Compact Handoff

Synthetic template. Replace placeholders before use.

## Protocol Version

0.2.0

## Last Updated

<ISO8601_TIMESTAMP>

## Current Goal

Initialize compact/resume handoff state for <REPO_ROOT>.

## Current Phase

IN_PROGRESS

## Session Role

Unknown

## Workflow Level

Unknown

## Output Contract

Summarize the current goal, blockers, Owner gates, and next safe action before resuming.

## Completed Work

- Created compact protocol files.

## Open Blockers

- None.

## Owner Gates

- UNKNOWN_PENDING: Owner review of compact handoff before first real use.

## Next 3 Actions

1. Review all compact files for project-specific placeholders.
2. Run `aiplus compact prepare`.
3. If readiness is `READY_TO_COMPACT`, manually use the host compact control.

## Do Not Do

- Do not include secrets, PII, raw audio, transcripts, provider payloads, or exact private paths.
- Do not claim this protocol can trigger any host agent compact control.
- Do not claim this protocol can wake a host runtime that is waiting for a user message.

## Recovery Order

1. Read this handoff.
2. Read `compact-policy.json`.
3. Review Owner Gates and Open Blockers.
4. Read `decision-log.md`.
5. Read `agent-state-ledger.md`.
6. Read `evidence-ledger.md`.
7. Run `resume`.

## Resume Sanity Check

- Current goal is clear.
- Current phase is one allowed task/result status.
- Session role and workflow level are captured, or intentionally set to Unknown.
- Next safe action is actionable.
- No sensitive material is present.
- Before compact, run `aiplus compact prepare`.
- After compact, run `aiplus compact resume` before continuing work.
- If the agent does not reply after compact, explicit AiPlus refresh triggers include
  `AiPlus 刷新`, `刷新 AiPlus`, `aiplus refresh`, `aiplus status`,
  `AiPlus status`, `继续 AiPlus`, and `resume AiPlus`.
- Generic `刷新` / `refresh` should try AiPlus first when possible, but explicit
  AiPlus triggers are safer when a project has its own refresh meaning.

## Compact Readiness States

- READY_TO_COMPACT: Handoff is current; manual compact is recommended.
- NEEDS_HANDOFF_UPDATE: Update this handoff before compact.
- BLOCKED_BY_OWNER_GATE: Resolve documented Owner gates or safety findings before compact.
- NOT_RECOMMENDED_DURING_ACTIVE_WORK: Finish or stabilize active work before compact.
- UNKNOWN_NEEDS_REVIEW: Review compact files before deciding.

## Role-Aware Recovery Notes

- Advisor: preserve recommendations, risks, CEO handoff, and boundaries.
- CEO: preserve goal, task cards, worker states, Result Packets, and acceptance criteria.
- Reviewer: preserve review scope, findings, evidence, unverified items, and verdict scope.
- Builder: preserve changed files, commands run, tests, not done, risks, and handoff notes.
