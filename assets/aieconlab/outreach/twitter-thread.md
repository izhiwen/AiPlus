# Twitter / X Thread

**Audience**: AI coding tool community + econ academic Twitter. Skew
slightly toward the AI/tool community in this thread; do a separate
econ-only thread for academic Twitter.

**Pre-thread reply rule**: have replies ready for the first 30 minutes
after posting. Engagement in those 30 minutes is the algorithm signal.

---

## Tweet 1 (the hook)

> Spent a year pair-programming with AI agents full-time.
>
> Found six failure modes that cost me hours every day:
> – forgets across sessions
> – loses context after compact
> – multiple agents racing for the lead
> – estimates anchored on human hours
> – plans that skip security/UX
> – one agent wearing every hat
>
> So I built a thing.

## Tweet 2

> AiPlus is a Rust CLI that treats all six. Five modules. Everything
> stays local to your project's `.aiplus/`. No daemon. No cloud.
> No global config edits.
>
> github.com/izhiwen/AiPlus
> v0.5.9 ships pre-built binaries for macOS / Linux / Windows.

## Tweet 3 (the meta)

> The meta-frame I'll cop to: I used AI agents to build the toolchain
> that manages AI agents.
>
> That recursion has been the most useful debugging loop of the whole
> thing — every flaw it has, I hit personally before I patched it.

## Tweet 4 (a concrete design choice)

> Best design choice I've made:
>
> Each "agent" has its own git worktree.
>
> Two engineers can work in parallel; conflicts surface through git
> instead of silent context-window overwrites.
>
> Worst design choice I've reconsidered most often: same thing. Worktrees
> are a sharp tool.

## Tweet 5 (the sibling)

> There's also an econ-research version: AiEconLab.
>
> PI / Theorist / RA-Stata / Referee / Replicator instead of CEO /
> Architect / Engineer / QA.
>
> Includes an LLM-as-Measurement Specialist for papers using LLMs to
> score text data (validity protocol, multi-model panel, hand-coded
> subsample).

## Tweet 6 (the disclaimer)

> Real-world battle-test on my own research papers: not done yet.
> Design vs reality gap is unknown.
>
> What I do know: the install flow works on 5 platforms, the 15
> structural acceptance tests pass, and `aiplus compact savings` shows
> real tokens saved across my sessions.
>
> Single maintainer. Open issues welcome.

## Tweet 7 (the call to action)

> Try it:
>
> ```
> curl -L github.com/izhiwen/AiPlus/releases/latest/download/aiplus-aarch64-apple-darwin.tar.gz | tar xz
> sudo mv aiplus /usr/local/bin/
> cd MyProject && aiplus install codex
> ```
>
> github.com/izhiwen/AiPlus
>
> Tell me what breaks.

---

## Alternative: short single-tweet version

For one-shot exposure when threading isn't appropriate:

> Built a Rust CLI that gives AI coding agents (Codex / Claude Code /
> OpenCode) a permanent team, memory, compact-aware handoff, and
> token-savings tracking.
>
> Five modules, all local-first. No daemon. No cloud.
>
> v0.5.9 multi-platform binaries.
>
> github.com/izhiwen/AiPlus

---

## Econ-focused tweet (separate thread for econ Twitter)

> Built an AI research-agent toolkit for applied economists.
>
> PI / Theorist / RA-Stata / Referee / Replicator. Plan-time consultant
> with Design Credibility / Contribution Framing / Day-1
> Reproducibility / IRB Gate / LLM-as-Measurement seats.
>
> Apache-2.0. AiEconLab (AEL).
>
> github.com/izhiwen/AiEconLab
