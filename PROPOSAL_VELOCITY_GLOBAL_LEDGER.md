# Proposal — AiPlus cross-project velocity ledger (v2 spec)

**Status**: DRAFT — awaiting Owner approval on Q1–Q5 before any source edit.
**Spec source**: v2 prompt dated 2026-05-14 (supersedes v1).
**Branch**: `feat/velocity-global-ledger`
**Dogfood est_id**: `est_1778739100712` (p50=60min, p90=120min, MATCHED_RECORDS=0 — exactly the failure mode this work fixes).

---

## Reconnaissance findings (read-only inspection)

| Question | What I found |
|---|---|
| Where does velocity CLI live? | `crates/aiplus-cli/src/main.rs:371` declares the `Velocity` subcommand inline (single struct, dispatched at `:958`). **There is no `crates/aiplus-cli/src/velocity/` subdir** — the spec mentions one; minor inaccuracy. All CLI plumbing changes will land in `main.rs`. |
| `deny_unknown_fields` on velocity types? | **None.** All velocity structs use `#[serde(rename_all = "camelCase", default)]`. Schema forward-compat at the Rust layer is **already in place** — spec item #5's BLOCK fix is a no-op. Will add regression test + document the invariant in DESIGN.md §Schema versioning. |
| `additionalProperties: false` on JSON schemas? | **None** (`grep` returned 0 across `assets/aiplus-agent-velocity/core/schemas/*.json`). Same situation — forward-compat by accident; we lock it in by policy. |
| Current `est_id` format | `est_{unix_ms}` via `generate_estimate_id()` at `velocity.rs:618`. Q4 hinges on whether to migrate to ULID. |
| Current schema_version | `VELOCITY_SCHEMA_VERSION = "1"` (string `"1"`, not `"0.1"`). Spec wants bump to `"0.2.0"`. To minimize migration cost we'll bump to `"2"` (string), keeping the format. |
| Sensitive-text rejection | `reject_sensitive_velocity_text()` exists at `velocity.rs:1226` — runs `sensitive_findings()` against record JSON before write. We can re-use this for project-local but **global writes drop the `task` field entirely** (structural privacy ≫ pattern detection). |
| Concurrency primitives currently | None visible — `append_jsonl` does `OpenOptions::new().append(true).open(...)` then `write_all`. Single-process safe by default; multi-process unsafe. Spec #11 requires explicit `O_APPEND` (which is what `append(true)` maps to on macOS/Linux — actually safe for ≤PIPE_BUF writes) plus `flock` for the JSON read-modify-write files. |
| 96 projects on this Mac | Verified ballpark via `aiplus prune-projects` (kept=93 in earlier session). Migration tool must scale to ~100 sources. |

**Net surprise**: forward-compat is already free. The real engineering work is **dual-write + dedupe + concurrency + privacy projection**, not unblocking Rust serde.

---

## Recommended answers (TL;DR)

| Q | Recommended | One-line tradeoff |
|---|---|---|
| Q1 — merge rule | **(b) Project-recent-heavy** (50 local + 150 global) | Equal weight (a) collapses when any project has ≥200 records; user-configurable (d) is YAGNI |
| Q2 — retention | **1000 normal + 100 rare** in global | ~1 MB cap is predictable; doubles project's 200 in informational value while staying small |
| Q3 — duplicate `est_id` | **(c) Reject at write, idempotent no-op + stderr warn** | LWW is invisible; project-wins makes migration race-y |
| Q4 — multi-machine sync forward-compat | **(c) Design for ULID + append-only invariants, don't ship sync** | Cheap to design in now (one ID generator change); expensive to retrofit later |
| Q5 — opt-out semantics | **(c) Three-state explicit** `share_to_global_mode: read_write \| read_only \| none`, default `read_write` | Asymmetric (b) is real but confusing as a default; (c) is one extra config value for full flexibility |

**If you want to skip the detail**: reply `Q1=b Q2=ok Q3=c Q4=c Q5=c default=read_write` and I'll proceed.

---

## Q1 — Merge rule (which records get scored for `estimate`)

**Recommendation: (b) project-recent-heavy. 50 local + 150 global, dedupe by id, sort by recency.**

| Option | When it wins | When it breaks |
|---|---|---|
| **(a) Equal weight, latest 200 from union** | Small project + big global cohort → user sees broad data | Any project with ≥200 recent records → global tier contributes **zero**. This is the AiPlus repo itself today. |
| **(b) Project-recent-heavy: 50 local + 150 global** ✅ | Both tiers always contribute. Local recency wins for context; global breadth tempers it. | Adds a config-driven split that becomes a new design surface. Mitigation: ship as a constant first; expose to config if needed. |
| (c) Same-context-heavy: weight by `(task_type, model)` | Maximally relevant matches | Doubles bucket-walk complexity; collapses on cold start (which is the very case global was supposed to fix) |
| (d) User-configurable enum | Maximum flexibility | Three valid modes × forward-compat invariants → 12 valid migration paths. YAGNI for one user. |

**Why not (a)**: AiPlus repo has > 200 records in its own velocity store; "equal weight" would mean global tier is **silently inert** on the repo that built the feature. Same will happen on any active paper project. (b) is the only option that **provably always lets global contribute** until you've done >150 tasks of the same type.

---

## Q2 — Retention windows

**Recommendation: 1000 normal + 100 rare in `~/.config/aiplus/velocity/`. Project stays 200 + 20.**

Quick math at observed sizes (RunRecord JSON ~700-900 bytes after dropping `task`):
- `runs.jsonl`: 1000 × 900B ≈ 900 KB
- `estimates.jsonl`: 1000 × 500B ≈ 500 KB
- `rare-cases.jsonl`: 100 × 1.2KB ≈ 120 KB
- `anchor-signals.jsonl`: bounded similarly

Total ≤ ~2 MB per user lifetime. Fits comfortably in `~/.config/` budget. Rotation rule = existing rotation logic (`apply_retention`) just scoped to global dir.

**Alternative considered**: 5000+100 (5× project). Rejected — diminishing returns; bias-detection signal saturates ~500 records.

---

## Q3 — Duplicate `est_id` between local and global

**Recommendation: (c) Reject duplicate at write time. Idempotent: second write of same id is a no-op + stderr warning.**

Three failure modes for the alternatives:

- **(a) Global wins (LWW)**: project completes a task, writes locally, syncs to global; later a different machine's clock skew rewrites the global record with an *older* run. Invisible data loss.
- **(b) Project wins**: same id appears in two projects (collision via clock-collision in unix_ms id) — neither side resolves cleanly.
- **(c) Reject + warn** ✅: makes migration idempotent (spec acceptance #5 wants this), makes concurrency robust (8 writers can't accidentally double-count if they share an id), and surfaces clock-collision bugs immediately instead of hiding them.

Implementation: a `BTreeSet<String>` of seen ids per JSONL file, loaded once at writer-init. Append-time check is O(log n) lookup before flush.

**Edge case**: dedup `BTreeSet` for `runs.jsonl` at 1000 records uses ≤ 24KB resident — negligible.

---

## Q4 — Forward-compat for multi-machine sync

**Recommendation: (c) Design for it; don't ship sync.**

The commitments this option locks in:

1. **ID generator switches to ULID** (26-char Crockford base32, sortable, machine-collision-resistant)
   - Backward: old `est_{unix_ms}` ids stay valid forever (string comparison still works for sort-by-recency since `est_` < any ULID alphabet char, and unix_ms IDs sort by time within their own family).
   - Forward: new IDs are ULIDs. Two machines writing concurrently to a shared NFS/iCloud-synced dir cannot collide. ULID's 80 bits of random make `est_id` collision probability < 10⁻¹⁸ per write.
2. **Append-only invariant on JSONL files** — no in-place edit of past records. Rotation = file truncate from oldest + re-append, never mid-line edit. This makes the JSONL files **safe to sync** via any naive file-replication tool (iCloud, Syncthing) that does not understand JSONL semantics.
3. **JSON files stay read-modify-write** with `flock + rename`. They are *not* sync-safe to multi-machine without conflict resolution. Document as "machine-local on multi-machine deployments."

**Cost**: one new dep — `ulid = "1"` (or hand-rolled ULID in ~30 LOC; both acceptable). I'll choose hand-rolled to avoid the dep (spec says "no new third-party deps unless justified"). 30 LOC of Rust + 4 unit tests.

**Why not (b) — "no, forever local"**: I trust your future self to want this within 18 months. The cost of retrofitting ULIDs after 5,000 records exist is migration script + dedup-windowing + Owner-attention. The cost of paying now is one constant change.

**Why not (a) — "yes, ship sync"**: out of scope per the v2 spec.

---

## Q5 — Opt-out semantics (NEW in v2)

**Recommendation: (c) Three-state explicit. Default new projects to `read_write`.**

```toml
# .aiplus/velocity/config.json (new field)
share_to_global_mode = "read_write"  # | "read_only" | "none"
```

| Mode | Reads global at `estimate` time? | Writes to global at `complete` time? | Use case |
|---|---|---|---|
| `read_write` (default) | Yes | Yes | Default for new projects + AiPlus's own dogfooding |
| `read_only` | Yes | No | "I want to learn from my other projects but this one is private (client work, IRB-restricted)" |
| `none` | No | No | Full isolation. Matches v1 "opt-out" semantic but without ambiguity. |

The two-state alternative `share_to_global: false` is ambiguous (does false mean "don't write" or "be invisible"?). The three-state enum costs 1 extra config value, eliminates the ambiguity, and matches a real workflow Owner has (the user research interview scripts already imagine an "IRB-restricted paper" scenario).

**Privacy crosswalk**:
- `none` users get zero global footprint (their CONFIDENCE=low never improves from this feature)
- `read_only` users free-ride on others' history without contributing — explicit consent that they're a one-way receiver
- `read_write` users contribute structured labels only (no free-text task) — privacy floor in spec #7

---

## Non-Q-decision design notes (FYI, not asking)

### Storage layout

```
~/.config/aiplus/velocity/         (mode 0700)
├── config.json                    (mode 0600, schema_version="2")
├── estimates.jsonl                (mode 0600, append-only, ≤1000 records)
├── runs.jsonl                     (mode 0600, append-only, ≤1000 records)
├── anchor-signals.jsonl           (mode 0600, append-only)
├── rare-cases.jsonl               (mode 0600, append-only, ≤100 records)
├── multipliers.json               (mode 0600, RMW, flock + atomic rename)
├── aggregates.json                (mode 0600, RMW, flock + atomic rename)
└── rotation-state.json            (mode 0600, RMW, flock + atomic rename)
```

### Minimal record projection for global

Global `RunRecord` keeps:
`schema_version`, `id`, `created_at`, `task_type`, `repo_area`, `model`, `workflow_level`,
`original_estimate_minutes`, `human_baseline_minutes`, `actual_active_minutes`,
`wall_clock_minutes`, `tool_wait_minutes`, `blocked_minutes`, `outcome`,
`verification_depth`, `quality_verdict`, `rework_count`, `owner_gate_hit`,
`overestimate_ratio`, `human_time_bias`, `slow_reason`, `seed`.

Global `RunRecord` **drops**:
- `task_id`, `estimate_id` (project-cwd-traceable)
- `project_id`, `repo_area`'s project-specific suffix
- `agent_role` (could be project-specific role name in AEL)
- `runtime` (machine-fingerprint adjacent)
- `actual_time_source` (`"manual"` vs `"capture"` could leak workflow shape)
- `redaction_status`, `raw_content_stored`, `secret_values_stored`, `memory_integration`
- Everything not strictly needed by `compute_ai_native_estimate` + `detect_bias`

This is the **structural privacy** spec #7 demands. The redaction-status fields are dropped because they're meaningful only against the original `task` text.

### Concurrency strategy by file (spec #11)

- **JSONL**: `OpenOptions::new().append(true)` already uses `O_APPEND` on POSIX. **Asserted-in-test** that each record is < 4096 bytes (PIPE_BUF on macOS+Linux). No additional lock needed.
- **JSON (read-modify-write)**: `fs2::FileExt::lock_exclusive()` on a hold file (`<file>.lock`), then `read(file)` → mutate → `write(<file>.tmp)` → `rename(<file>.tmp, file)` → drop lock. Atomicity = `rename(2)` on same filesystem.
- **iCloud/Dropbox warning**: detect at doctor-time via `realpath ~/.config/aiplus/velocity` — if path contains `Mobile Documents/` or `Dropbox/` or symlink target is in known sync root, doctor reports `global_ledger_health=NEEDS_FIX` with hint to move it.

### Migration path

`aiplus velocity import-from-project <path>` — opt-in only. Reads `<path>/.aiplus/velocity/{runs,estimates,rare-cases,anchor-signals}.jsonl`, projects to minimal schema, drops `task` field, appends to global with dedup. Prints `IMPORTED=N SKIPPED_DUPLICATE=M FAILED=K`.

For mass-migration of all 96 projects: a future `aiplus velocity import-all` (out of scope here) that walks the registry — Owner can pull the trigger when this lands and gets validated.

### CHANGELOG entry (drafted, not committed yet)

```markdown
### 0.5.x — cross-project velocity sharing

- New: `~/.config/aiplus/velocity/` global ledger. Records flow from
  every project that opts in, structured labels only (no free-text
  task descriptions). Brand-new projects immediately benefit from
  your cross-project history.
- New: `aiplus velocity report --scope local|global|both` (default both)
- New: `aiplus velocity import-from-project <path>` for one-shot
  migration
- Config: `.aiplus/velocity/config.json` gains `share_to_global_mode`
  (`read_write` | `read_only` | `none`, default `read_write`)
- Doctor: new fields `local_records_count`, `global_records_count`,
  `synced_records_count`, `share_to_global_mode`, `global_ledger_health`
- Schema: velocity records now use ULID ids for new entries (old
  `est_{unix_ms}` ids stay valid and readable).
- Privacy floor: the global ledger is structurally incapable of
  storing free-text task descriptions, file paths, project names, or
  machine identifiers. Project-local ledger is unchanged.
- 中文：跨项目 velocity 信息共享落地。新项目立即能用其他项目的
  历史校准 AI 速度估算。全局 ledger 只存结构化标签，永远不
  存任务文本/文件路径/项目名/机器标识。每个项目可独立选择
  `read_write`/`read_only`/`none`。
```

---

## Acceptance crosswalk (spec § "Acceptance criteria")

| # | Criterion | Approach |
|---|---|---|
| 1 | `cargo test --workspace` PASS | Add ≥5 new tests across the changed surface; existing tests unchanged. |
| 2 | doctor reports the 6 new fields | Extend `DoctorReport` struct + CLI rendering in `main.rs`. |
| 3 | brand-new project sees `MATCHED_RECORDS ≥ 1` if global has matches | Modify `compute_ai_native_estimate` to merge global before bucketing. |
| 4 | `report` prints LOCAL_* + GLOBAL_* blocks | New `--scope` flag; default both. |
| 5 | migration idempotent | Dedup by `est_id` / `run_id` at write time (Q3=c). |
| 6 | opt-out works (Q5=c) | Read short-circuit on `none`; write short-circuit on `none|read_only`. |
| 7 | privacy structural test | `grep` for sensitive text in global JSONL after a write that includes it → 0 matches; check `task` field absent (not empty). |
| 8 | 8 × 50 concurrency stress | New `tests/velocity_concurrency.rs`. |
| 9 | schema forward-compat | New `tests/velocity_forward_compat.rs`. |
| 10 | file permissions 0700/0600 | `set_permissions` after create; verified in doctor. |
| 11 | CHANGELOG bilingual | drafted above. |
| 12 | this proposal exists with Q1-Q5 pinned | ← you are here |

---

## What I will NOT do without Owner consent

- Touch `Cargo.toml` version
- Push tags / draft GitHub Releases
- Add a new third-party dep (will hand-roll ULID instead)
- Implement cross-machine sync code
- Change CLI surface beyond `--scope` + `import-from-project`
- Modify any module outside `crates/aiplus-cli/src/main.rs` + `crates/aiplus-core/src/velocity.rs` + the `assets/aiplus-agent-velocity/` tree
- Bulk-migrate the 96 existing projects (`import-from-project <path>` is per-path only here)

---

## PROPOSAL_DRAFTED

`PROPOSAL_DRAFTED: /Users/steve/Dropbox/Project/AiPlus/aiplus-public/PROPOSAL_VELOCITY_GLOBAL_LEDGER.md`

**Awaiting Q1–Q5 decisions.** Default-recommendation reply for the impatient:

> `Q1=b Q2=ok Q3=c Q4=c Q5=c default=read_write`
