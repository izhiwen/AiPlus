# PERF-1: Dispatch pipeline acceleration

**Status**: DRAFT - Infra implementation branch.
**Drafted by**: Advisor (Claude Code session, paired with CEO codex)
**Owner**: Steve
**Date**: 2026-05-17

---

## TL;DR

`aiplus agent route` currently pays too much dispatch overhead before the
receiving role can do useful work. The slow path is mostly local orchestration:
serial reviewer/QA follow-up routing, repeated worktree discovery/provisioning,
and repeated cache invalidation that discards reusable context too broadly.

PERF-1 makes dispatch faster without weakening owner gates:

1. run required reviewer/QA sidecars in parallel with the primary builder route;
2. add a local worktree pool so hot role worktrees are reused instead of
   rediscovered/recreated on every dispatch;
3. reuse warm-bench cache entries across safe dispatches and invalidate only the
   roles whose state actually changed;
4. document batching protocol so coordinators send one structured dispatch batch
   instead of three sequential route commands.

## Goals

- Reduce wall-clock time for a builder dispatch that needs reviewer and QA
  sidecars.
- Preserve existing gate behavior: owner gates still run before worktree
  provisioning, consult writes, dispatch records, and sidecar scheduling.
- Keep implementation local and dependency-free.
- Keep CEO/G2 work isolated: do not touch `crates/aiplus-core/src/consult.rs`.

## Non-goals

- No LLM-based scheduler.
- No new dependency in `Cargo.toml`.
- No changes to agent persona content.
- No automatic push, publish, release, deploy, secret access, global config
  edit, or external-account action.

## A. Reviewer/QA Parallelization

### Problem

For non-trivial builder tasks, coordinators often route:

1. primary builder task;
2. reviewer pass;
3. QA verification.

That sequence serializes independent local setup work. Reviewer and QA can be
scheduled as sidecars after the owner gate passes because they do not need to
wait for builder implementation to start. They need the same task context,
acceptance criteria, and output contract.

### Design

Add a dispatch batch path in `agent route` that can schedule:

- primary role: the requested role;
- sidecar roles: `reviewer` and `qa` when requested by the batch spec;
- shared task text;
- per-sidecar suffixes for review and test responsibilities.

The first implementation may keep the public CLI surface conservative and test
the batch scheduler through local fixtures. The route code must make the
scheduling primitive explicit so the public CLI can expose it later without
reworking internals.

### Required behavior

- Owner gate evaluation happens once, before any sidecar starts.
- If the gate blocks, no primary or sidecar dispatch is recorded.
- Primary, reviewer, and QA dispatch records are written with a shared
  `batch_id` in metrics.
- Worktree provisioning for sidecar roles runs in parallel when those roles
  need worktrees.
- Consult findings remain best-effort and non-fatal.

### Phase 1 tests

- **T1**: a local fixture dispatches a builder role plus reviewer and QA, and
  the resulting metrics show one batch with all three roles.
- **T8**: if the task is owner-gated and not approved, no sidecar dispatch is
  written.
- **T9**: reviewer and QA sidecars are started independently; a slow QA fixture
  must not block reviewer dispatch recording.

Phase 1 is done only when T1, T8, and T9 pass.

## B. Worktree Pool + Cache Reuse

### Problem

Worktree setup is repeatedly rediscovered by role. Cache invalidation also
throws away role state whenever `route` is called, even when a safe sidecar
dispatch could reuse existing state.

### Design

Add `crates/aiplus-cli/src/agent/worktree_pool.rs` with a std-only pool:

- key: canonical role id;
- value: resolved worktree path plus metadata;
- source of truth: current git worktree list and role workspace config;
- lifecycle: acquire before dispatch, mark reused/created/skipped in metrics.

Add cache reuse rules:

- primary builder dispatch invalidates only the primary role;
- reviewer and QA sidecars may reuse cache unless explicitly stale;
- gate-blocked dispatches do not invalidate cache;
- worktree creation failures invalidate only the failed role.

### Required behavior

- Existing role worktrees are reused without calling `git worktree add`.
- Missing role worktrees are created through the existing worktree module.
- Non-git projects still route and record dispatch with a clear note.
- Metrics must show worktree `created`, `reused`, or `skipped`.

## C. Batching Protocol

### C1. Coordinator protocol document

Add `docs/team-protocols/dispatch-batching.md` describing how CEO/PI sessions
should batch dispatches:

- when to batch builder + reviewer + QA;
- when not to batch because the sidecar depends on builder output;
- required task-card fields;
- required verification evidence;
- owner-gate handling;
- team-memory checkpoint expectations.

The doc is protocol guidance only. It must not claim public release status.

## D. AEL Validation

Run a before/after dispatch check in the local AiEconLab project and capture
velocity metrics:

- baseline dispatch timing before PERF-1 path;
- accelerated dispatch timing after PERF-1 path;
- metrics line showing batch id, roles, gate result, worktree reuse status, and
  elapsed milliseconds.

## Acceptance for Done

- Step 0 spec is committed separately.
- Phase 1 T1, T8, T9 pass.
- Worktree pool implementation has focused tests.
- Cache reuse behavior has focused tests.
- `docs/team-protocols/dispatch-batching.md` exists and matches this spec.
- Local AEL validation has before/after velocity data.
- `cargo fmt --all --check` passes.
- Relevant `cargo test` targets pass, including all `perf1_*` tests.
- Final branch is rebased on current `origin/main`.
- Team memory contains phase summaries tagged `perf-1` and a final
  `perf-1-ready-for-merge` report.

## Owner gates

- OK without further approval: local branch commits, local docs, local tests,
  feature-branch push.
- Not OK: push to `origin/main`, release, publish, deploy, global config edit,
  secret exposure, external-account action.

## Implementation files

Owned by Infra Lead:

- `crates/aiplus-cli/src/agent/route.rs`
- `crates/aiplus-cli/src/agent/core.rs`
- `crates/aiplus-cli/src/agent/cache.rs`
- `crates/aiplus-cli/src/agent/worktree_pool.rs`
- `crates/aiplus-cli/tests/perf1_*.rs`
- `docs/team-protocols/dispatch-batching.md`

Do not touch:

- `crates/aiplus-core/src/consult.rs`
- any `agent/engineer-*`, `agent/qa`, or `agent/reviewer` worktree path.
