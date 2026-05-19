# Coordinator Parallel Implementation Notes

Status: Phase 1 design draft
Date: 2026-05-18
Authoritative briefing: `aiplus-agent-team/docs/decisions/coordinator-parallel-impl-briefing.md`
Goal: `aiplus-agent-team/docs/proposals/goal-G-AT-COORDINATOR-PARALLEL-1.md`

## 1. Dispatch Model Decision

Decision: build a new `coordinator_batch` peer-based primitive beside the
existing Perf-1 `route_batch`.

`route_batch` already proves the useful mechanics: `thread::spawn`, one shared
`Arc<Mutex<WorktreePool>>`, and per-role dispatch metrics. Its public shape is
not a clean fit for adaptive coordinator:

- it requires one `primary_role` and a sidecar list;
- it decorates reviewer/qa tasks through `sidecar_task`;
- it marks only the primary as `DispatchKind::Primary`;
- the printed batch line is framed as primary + sidecars.

Adaptive coordinator roles are execution peers. PM, architect, engineers,
reviewer, QA, and auto-summoned experts each receive `coordinator_role_task`
framing and have no code-level dependency chain. `coordinator_batch` therefore
keeps `route_known_role` as the single dispatch implementation, but fans out
the already-planned `staffing_roles` as peers.

Trade-off:

- Reusing `route_batch` unchanged would be the smallest patch, but would encode
  an artificial primary/sidecar model into coordinator dispatch and would
  either lose `coordinator_role_task` framing or duplicate it awkwardly.
- A new peer primitive duplicates only the small spawn/join loop. It preserves
  the route semantics and isolates coordinator-specific failure aggregation.

## 2. Partial-Failure Policy

Every staffed role runs in its own thread. One role returning `Err` or
panicking does not cancel or interrupt the other roles. The parent thread joins
all handles, collects failures into a vector, prints a concise partial-failure
summary, and returns one aggregated error after all workers finish.

Policy details:

- Success workers keep their normal dispatch logs and metrics.
- Failed workers record failure through `route_known_role` when the error is in
  its normal path; panics are caught at join time and reported as worker panic.
- Parent returns non-zero if any role failed, but only after collecting every
  worker result.
- The shared `WorktreePool` is protected by `std::sync::Mutex`; a panicking
  worker can poison the mutex. Existing call sites already convert poison to a
  normal error (`worktree pool lock poisoned`), so sibling roles continue until
  they need that mutex and then fail cleanly rather than deadlocking.

## 3. WorktreePool Race Assessment

`WorktreePool` itself stores only an in-memory role -> path map. In both the
existing Perf-1 path and the new coordinator path it is wrapped in
`Arc<Mutex<WorktreePool>>`, so all `acquire` calls are serialized. That means
6-way HEAVY fan-out cannot concurrently mutate `leases` or concurrently call
`git worktree add` through the same pool.

The lock is held across worktree discovery and creation. That is conservative
and may reduce parallelism while worktrees are being created, but it avoids the
race where two workers create the same role worktree. The role runtime work and
dispatch logging remain parallel after acquisition. No `worktree_pool.rs` code
change is planned unless tests expose contention or poison handling problems.

## 4. Test Plan And Baseline Method

New integration test: `crates/aiplus-cli/tests/coordinator_parallel_smoke.rs`.

Coverage:

- 6-way fan-out: installed agent-team HEAVY task dispatches all expected roles
  with one coordinator batch id.
- Partial failure: env fixture fails one role; the remaining roles still record
  successful metrics and parent returns non-zero with a partial-failure message.
- WorktreePool contention: six configured roles require worktrees in one HEAVY
  coordinator batch; metrics must show role worktree outcomes without hangs or
  duplicate role rows.
- Wall-clock baseline: set six role-specific delay env vars. Serial baseline
  is six delays; parallel acceptance is <= 1.5x one delay and >= 2x faster than
  the serial estimate.

Performance evidence uses deterministic delay env vars rather than live model
API calls, so the measurement covers dispatch overhead and thread fan-out
without network variance.

## 5. Draft CHANGELOG 0.6.3 Text

```text
## 0.6.3

- Adaptive coordinator dispatch now runs staffed roles in parallel instead of
  serially, making HEAVY tasks complete closer to the slowest role than the sum
  of all role startup times.
- Coordinator dispatch now reports partial failures after all role workers
  finish, so one failed role no longer hides sibling-role results.
- Added regression coverage for 6-way coordinator fan-out, partial failure, and
  worktree-pool contention.
```

## Phase 3 Evidence

Recorded 2026-05-18 on branch `feat/coordinator-parallel-1`.

Implementation:

- Added `coordinator_batch` beside the existing Perf-1 `route_batch`.
- `run_adaptive_route` now fans out `staffing_roles` through
  `coordinator_batch` instead of a serial `for` loop.
- Added `DispatchKind::CoordinatorPeer` so coordinator metrics do not pretend
  there is a primary/sidecar dependency.
- Added fixture-only failure env support
  (`AIPLUS_PERF1_FAIL_ROLE` / `AIPLUS_COORDINATOR_FAIL_ROLE`) for partial-fail
  regression coverage.
- `worktree_pool.rs` was not changed; the existing `Arc<Mutex<WorktreePool>>`
  call pattern serializes `acquire` safely under 6-way fan-out.

Focused tests:

- `cargo test -p aiplus-cli --test coordinator_parallel_smoke -- --nocapture`
  - Result: 4 passed.
  - Covers 6-way fan-out, partial failure, wall-clock delay benchmark, and
    6-way WorktreePool contention.
- `cargo test -p aiplus-cli --test perf1_dispatch_parallel -- --nocapture`
  - Result: 3 passed.
- `cargo test -p aiplus-cli --test perf1_worktree_pool_cache -- --nocapture`
  - Result: 2 passed.
- `cargo test -p aiplus-cli --test v03_adaptive_coordinator -- --nocapture`
  - Result: 1 passed.

Final test gates:

- `cargo test -p aiplus-cli`
  - Result: 352 passed, 1 ignored, 36 suites, 58.68s.
- `cargo test`
  - Result: 547 passed, 1 ignored, 38 suites, 78.11s.
- `aiplus agent route --score-only "实现支付接口"`
  - Result: planning unchanged: complexity 5, risk 0.85, tier HEAVY, staff
    `[pm,architect,engineer-a,engineer-b,reviewer,qa]`.

Wall-clock benchmark:

- Fixture: six coordinator roles each delayed by 900ms using
  `AIPLUS_PERF1_DELAY_<ROLE>_MS=900`.
- Before serial estimate from old loop: 6 * 900ms = 5.40s.
- After measured parallel dispatch wall-clock: 0.94s.
- Speedup: approximately 5.7x over serial estimate; meets >= 2x acceptance.
- Metrics: 6 coordinator peer rows written in
  `.aiplus/agents/dispatch-metrics.jsonl`.

Clippy note:

- `cargo clippy -p aiplus-cli --all-targets -- -D warnings` was run twice per
  retry-once rule and failed deterministically on pre-existing warnings in
  untouched files, primarily `aiplus-core` plus existing `aiplus-cli` lint
  debt. No clippy diagnostics pointed to new `coordinator_batch` or
  `coordinator_parallel_smoke.rs` code. Out-of-scope lint cleanup was not
  attempted.
