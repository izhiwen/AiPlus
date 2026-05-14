# r/programming Post

**Title**:
> AiPlus: I built a Rust CLI that gives AI coding agents a permanent team and memory (because mine kept forgetting and drifting)

---

## Body

I've been using AI coding agents (Codex, Claude Code, OpenCode) full-time
for nearly a year. About four months in I kept hitting the same six
coordination failures:

1. Cross-session amnesia — Monday it knows your conventions, Wednesday
   it asks again.
2. Post-compact context loss — token wall hits mid-feature, 30 minutes
   of plan gone.
3. Multiple agents step on each other — no one defined who's CEO, who
   builds, who reviews.
4. Estimates anchored on human-engineer hours — agent says "5 hours",
   does it in 20 minutes; says "5 hours" again next week.
5. Plans that skip security / onboarding / AI-integration concerns
   until release week.
6. One agent wearing every hat in the same context window — drift,
   pollution, each role done shallowly.

So I built AiPlus, five Rust modules that, together, fix all six. Each
module is independent (you can install just one). Everything stays
local in your project's `.aiplus/` — no cloud, no daemon, no telemetry.

GitHub: https://github.com/izhiwen/AiPlus

**Technical choices worth discussing**:

1. **State-level permanence, not process-level**. Each agent persona +
   memory + workspace lives as files on disk. The agent *process* is
   ephemeral — spawned on demand. This means: no daemon, recoverable
   across restarts, portable across machines, fits the host runtimes
   (which are all ephemeral). Trade-off: warm-start cost on every
   invocation. I added a "warm bench" cache to mitigate.

2. **Git worktrees as agent workspaces**. Each code-touching role gets
   its own worktree on its own branch. Two engineers can work in
   parallel; conflicts surface through git rather than silent
   overwrites. This is the single design choice I'd revisit most often
   if I were starting over — worktrees are a sharp tool.

3. **Three-layer memory** (personal / team / project). Project memory
   wins on conflict. Team-of-the-day decisions never override durable
   project consensus. Conflict resolution is structural, not LLM-arbitrated.

4. **Local JSONL records, no SQLite/vector-DB**. Audit-able, git-able,
   greppable. Records rotate at 200 entries (rare cases at 20).
   Aggregate multipliers survive raw-record rotation. The argument
   against vector search: the agent already has retrieval; what I need
   is *audit*, not *similarity*.

5. **Multi-runtime via project-local adapters**. Each runtime (codex,
   claude-code, opencode) gets a `.codex/` / `.claude/` / `.opencode/`
   directory with managed files. AiPlus never touches your global
   `~/.codex/` config.

**Stats**: 16K LOC Rust, 5 modules, v0.5.9 cut, pre-built binaries for
5 platforms.

**Known unknowns**:
- I haven't dogfooded it on the longest possible session (multi-day
  agent work). Compact-reminder claims should hold but unverified at
  that scale.
- Single maintainer. Will not pretend otherwise.

Curious which design choice gets pushback first.
