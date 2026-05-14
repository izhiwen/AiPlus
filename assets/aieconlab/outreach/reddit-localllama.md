# r/LocalLLaMA Post

**Flair**: `Resources`

**Title**:
> [Tool] AiPlus — local-first CLI that gives AI coding agents (Codex/Claude Code/OpenCode) memory, compact handoff, and a virtual team. v0.5.9 multi-platform binaries.

---

## Body

Posting because I think this community will care: this is a CLI for
making long AI coding sessions less wasteful.

**The pain it treats**:

If you've ever lost 30 minutes of plan after a compact, asked the agent
the same architectural question on Monday and Wednesday, or had three
agents race to lead the same task — those are the failure modes I built
this around.

**What it actually does** (5 modules, all local-first):

1. **agent-memory** — JSONL store under `.aiplus/memory/`, 12 redaction
   patterns strip secrets before write. No vector DB, no cloud.
2. **compact-reminder** — tells you when it's a good time to compact
   (token threshold + task-handoff-point detection), prepares a
   structured handoff, auto-resumes from a verified capsule after.
   `aiplus compact savings` shows tokens + $ saved.
3. **auto-team-consultant** — virtual review board fired before plans.
   5 expert seats + user personas. Light/medium/heavy tiers so it
   scales the consult to the task.
4. **agent-team** — replaces single-agent drift with a permanent team
   (Advisor / CEO / Architect / PM / 2× Engineer / Reviewer / QA).
   Each role has isolated git worktree + memory namespace.
5. **agent-velocity** — records every estimate vs actual, detects when
   the agent is anchoring on human-engineer hours, feeds back into next
   estimate.

**Everything stays in `.aiplus/`**. No daemon, no cloud sync, no global
config edits, no telemetry, no upload.

**Multi-runtime**: works with Codex, Claude Code, OpenCode. Adapters
land per-runtime; the agent-team commands are the same surface across.

**Multi-platform** in v0.5.9: pre-built binaries for macOS (Intel +
Apple Silicon), Linux (x86_64 + aarch64), and Windows. Or build from
source with `cargo build --release`.

**Why I'm telling you specifically**: this community has the highest
density of people running long agent sessions and hitting the
compact/memory/coordination pain. The token-savings module
(`aiplus compact savings`) reports real numbers — I'd love to know if
your actual sessions confirm or refute the design's intuitions.

**Install**:
```
curl -L https://github.com/izhiwen/AiPlus/releases/latest/download/aiplus-aarch64-apple-darwin.tar.gz | tar xz
sudo mv aiplus /usr/local/bin/
cd MyProject
aiplus install codex   # or claude-code, opencode
```

**Repo**: https://github.com/izhiwen/AiPlus

**Caveats**:
- Single maintainer (me). Not production-grade in the "company depends
  on this" sense.
- v0.5 series shipped fast (9 releases in 5 days). Some rough edges.
- Real-world battle test on the maintainer's own research paper hasn't
  happened yet — design vs reality gap is unknown.

If you try it, please open a GitHub issue with what worked and what
broke. The CLI surface is still evolving and your friction is the only
real signal I have.
