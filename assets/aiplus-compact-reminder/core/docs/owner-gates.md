# Owner Gates

Owner gates are explicit human or owning-agent decisions that control whether compacting can proceed. They are carried in every required Markdown state file so a resume never loses safety-critical approval state.

Allowed statuses:

- `APPROVED`: the owner has cleared the gate.
- `DENIED`: the owner has blocked compacting or the dependent action.
- `UNKNOWN_PENDING`: the owner state is unknown and must be reviewed.

## Enforcement

`DENIED` causes checkpoint to return `BLOCKED_DO_NOT_COMPACT` with exit code `1`.

`UNKNOWN_PENDING` causes checkpoint to return `UNKNOWN_NEEDS_REVIEW` with exit code `2`.

Only clean validation with no denied or pending owner gates can return `SAFE_TO_COMPACT`.

## Authoring Rules

Keep each gate line concrete and redacted. Include the decision owner or source when it is safe to do so, but do not include private contact data, raw transcripts, or private paths.

Use owner gates for irreversible actions, unclear authority, external access, missing files, policy exceptions, and any compact boundary where the next agent might otherwise infer approval from incomplete context.
