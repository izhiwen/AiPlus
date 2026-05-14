# Bluesky Thread

Bluesky audiences trend more "thoughtful long-form" than Twitter.
You can be more reflective. 300 chars per post (vs Twitter's 280).

---

## Post 1

After a year of full-time AI pair-programming, I made an honest list of
the six coordination failures that cost me hours every day:

forgets across sessions, loses context after compact, agents racing for
lead, estimates anchored on human hours, plans skipping security/UX, one
agent wearing every hat.

## Post 2

So I built AiPlus — Rust CLI, 5 modules, all project-local.

Memory: redacted JSONL, no vector DB. Compact: structured handoff +
auto-resume + token-savings reporting. Consultant: virtual review
board fired before plans. Team: 8 permanent roles with isolated git
worktrees. Velocity: human-hour anchoring detection.

github.com/izhiwen/AiPlus

## Post 3

The meta-frame I'll cop to: I used AI agents to build the toolchain
that manages AI agents.

The recursion has been the most useful debugging loop. Every flaw the
toolchain has, I hit personally before I patched it.

## Post 4

Cleanest design choice: each agent gets its own git worktree.

Two engineers work in parallel; conflicts surface through git instead
of silent context-window overwrites.

It's also the choice I revisit most often. Worktrees are sharp.

## Post 5

Spin-off for applied economists: AiEconLab (AEL).

PI / Theorist / RA-Stata / Referee / Replicator + 12 experts including
an LLM-as-Measurement Specialist. Plan-time consultant with 5 econ-
research seats. Default Python + Stata + LaTeX.

github.com/izhiwen/AiEconLab

## Post 6

Real-world battle-test on my own research papers: not done yet. Design
vs reality gap is unknown.

What is done: 15 structural acceptance tests pass, install works on
macOS/Linux/Windows, single-maintainer disclaimer is explicit.

If you try it and something breaks, open a GitHub issue.

## Post 7

Apache-2.0. Single maintainer (me, on the job market, applied econ
PhD).

Pre-built binaries: github.com/izhiwen/AiPlus/releases/latest
Both repos public. CI green.

Tell me what breaks.
