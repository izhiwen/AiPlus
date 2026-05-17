# Dispatch Batching Protocol

**Status**: internal protocol draft for PERF-1.
**Scope**: CEO/PI coordinator dispatches that can safely start reviewer and QA
sidecars alongside a primary builder.

## When to batch

Batch a dispatch when all of these are true:

- The primary role is a builder or implementation owner.
- Reviewer can evaluate the task card, scope, risks, and likely regression
  surface before the builder finishes.
- QA can prepare acceptance checks, fixture expectations, or verification
  commands before the builder finishes.
- The task has stable acceptance criteria and clear file boundaries.
- Owner gates are constraints, not requested actions.

Typical batch:

```text
primary: engineer-a | engineer-b | architect | pm | ra-stata | ra-python
sidecars: reviewer, qa
shared_task: <task card>
reviewer_suffix: review correctness, safety, and regression risk
qa_suffix: verify acceptance criteria and test evidence
```

## When not to batch

Do not batch when reviewer or QA needs the builder's final diff before doing
useful work. Use sequential dispatch instead for:

- final code review of an already-written patch;
- post-implementation QA that must inspect generated artifacts;
- tasks where the builder is expected to discover or change requirements;
- owner-gated actions such as push to main, release, publish, deploy, global
  config edits, secret exposure, external-account actions, telemetry, or private
  data upload;
- ambiguous tasks where batching would create duplicate or contradictory work.

## Required task-card fields

Every batched task card must include:

- `goal`: concrete outcome for the primary role.
- `scope`: files, modules, or docs the primary role may touch.
- `do_not_touch`: files, worktrees, secrets, or global config that are out of
  bounds.
- `acceptance`: checks that define done.
- `sidecar_contract`: what reviewer and QA can do before the primary output
  exists.
- `owner_gates`: explicit statement that gated operations remain blocked unless
  the Owner approves.

## Gate handling

The coordinator must treat owner gates as a pre-batch barrier:

1. Run the normal route gate before scheduling any sidecar.
2. If the gate blocks, stop the entire batch.
3. Surface the gate id and do not route reviewer or QA as a workaround.
4. Only retry with `--owner-approved <gate-id>` after explicit Owner approval.

Gate-blocked batches should not produce reviewer or QA dispatch records.

## Verification evidence

Reviewer and QA sidecars should return concise evidence:

- Reviewer: risks found, files reviewed, blocking/non-blocking findings, and
  whether the primary task card is clear enough to execute.
- QA: fixture names, commands to run, expected outputs, and any test gaps.
- Both: whether their work was independent pre-work or dependent final review.

The primary role remains responsible for implementation evidence.

## Team-memory checkpoint

At the end of each phase, write a team-memory `decision_summary` tagged in the
text with the goal id, such as `perf-1`, containing:

- phase completed;
- tests or commands run;
- before/after metrics when applicable;
- blockers or owner gates;
- next phase.

Do not store raw transcripts, secrets, private paths, or long command output in
team memory.

## Metrics

Batch metrics should include:

- `batchId`
- `role`
- `kind`: `primary` or `sidecar`
- `outcome`
- `worktree`: `created`, `reused`, `skipped`, or `failed`
- `cacheInvalidated`
- `elapsedMs`

The coordinator should use these metrics for before/after velocity comparisons,
not as a public benchmark claim.
