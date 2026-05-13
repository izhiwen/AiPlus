---
title: "Why I built AiPlus — a toolchain for my own AI coding workflow"
date: 2026-05-13
author: izhiwen
---

I was four hours into a refactor when my agent stopped mid-thought,
scrolled up, and asked me a question I'd answered forty minutes earlier.
The session had compacted somewhere around hour three. The plan I'd written
before lunch — six concrete steps, half of them already done — was gone. I
retyped what I could remember. I lost the thread anyway. It was the third
time that week, and it wasn't going to be the last.

## Six failures, on repeat

That refactor wasn't unusual. After the better part of a year of
pair-programming with AI coding agents full-time — Codex, Claude Code,
OpenCode, rotating per task —
six specific failure modes had started repeating themselves often enough
that I could name them.

The first is the obvious one: cross-session amnesia. On Monday I'd teach
the agent my naming convention. On Wednesday it would ask again. By Friday
I'd have explained the same architectural decision four times, and I was
genuinely starting to wonder whether I needed a shell script to paste my
own preferences back into the prompt every morning. (I tried that. It
worked for about a week, then the conventions evolved and the script lied.)

The second is post-compact context loss — the four-hour-refactor problem
above. Compact is supposed to be a graceful summarization, and most of the
time it is. But mid-task on a long piece of work, it ends up summarizing
exactly the thing you wish it hadn't.

The third I noticed once I'd started running multiple agents in the same
repo: three agents in one project, none of them designated CEO, all three
convinced they were in charge. They'd quietly re-plan each other's work.
Two of them would converge on the same file from different branches.

The fourth was subtler. I'd ask an agent how long a refactor would take.
"Five hours," it would say, with the certainty of a senior engineer who'd
done this before. Twenty minutes later it was done. Next week, same task,
same five-hour quote, same twenty-minute completion. No one was keeping
books. The unit was wrong but nothing surfaced it.

The fifth I noticed at release week, painfully: agents draft plans that
quietly skip the things that bite hardest in production. Onboarding ease.
Security boundaries. Privacy assumptions. Real-world execution pitfalls.
The classes of concern that experienced reviewers raise *before*
implementation, an unsupervised agent will quietly defer until reviewers
do — or until users do, which is worse.

The sixth is the most structural: one agent wearing every hat. CEO,
reviewer, builder, advisor, all crammed into the same context window.
Roles drift inside one transcript. Implementation hacks bleed into product
reasoning. Review nitpicks bleed into architectural decisions. The agent
ends up doing every job shallowly because the work *is* that structured
and one prompt-history isn't.

## AI-native time estimation

The fourth failure is the one I think about most, because it's the one
most people get backwards.

It's tempting to say AI agents are "faster human engineers" and just
discount their estimates. But that's not what's happening. Human-hour
estimates are a coordination convention — they're how engineering teams
negotiate sprint scope, capacity, parallel branches. They encode
interruption cost, switching overhead, the cost of explaining to a
teammate. None of those apply when the agent is the implementer. An AI
agent doesn't lose 20 minutes to context-switch. It doesn't need lunch.
It doesn't have to be careful about stepping on a colleague's branch.

So "five hours" isn't merely wrong by a factor of 15 — it's the wrong
*unit*. The fix isn't to estimate harder. The fix is to rebuild the unit
of estimation. I started logging every estimate and every actual
completion, and computing p50 and p90 from my own history. After a couple
hundred data points, the calibration started outperforming my gut. The
estimate "five hours" never appears in AiPlus's velocity output. The
estimate is more like: *AI-native p50: 18 minutes, p90: 41 minutes, based
on 23 similar tasks in this repo.* That number is useful. The five-hour
anchor was theater.

## Role pollution, and what audit independence actually means

The other insight came from staring at the sixth failure long enough.

The intuitive fix to "one agent wears every hat" is to give the agent
better prompts: clearer role instructions, sharper context-switches, more
disciplined transitions. I tried this for a while. It doesn't work, for a
structural reason: the role itself isn't the failure. The failure is that
the prompt history of role A is in the same window as the prompt history
of role B. Switching role labels doesn't unpollute the context.

The real fix is division of labor — separate workspaces, separate memory,
separate persona files, separate transcripts. The same reason real
engineering teams have a PM and an Architect and a Reviewer is the reason
AI workflows need them: the work is structured that way.

There's a sharper variant of this principle that took me longer to
internalize: **an agent that audits another agent's work cannot share that
agent's context, tools, or task source.** If it does, it isn't auditing —
it's just self-attestation in a costume. AiPlus's audit gate enforces this:
the auditor reads a different context root, runs from a different
worktree, and never sees the builder's plan. There's a Chinese phrase for
this — 三权分立, three-power separation. The auditor is independent or
it's worthless. There's no middle setting. This was the design decision
that made me trust my own multi-agent system enough to leave a task
running unattended for two hours.

## What's still unsolved

A few things are still open and I want to be honest about them.

Cross-provider auditor independence is harder than the same-provider case.
If Codex builds and Codex audits, even with separate context roots, there's
a shared model-family prior I can't fully isolate. The clean version of
audit independence wants a different model family for the auditor; I
haven't shipped that and it's a v0.3 problem.

The AI-native estimation calibration is fine once it has ~200 data points
per task type, but the cold-start months are noisy. I haven't found a good
way to bootstrap from someone else's history without importing their bias
along with it.

And the velocity module doesn't yet handle multi-day tasks well. Anything
that spans more than one compact cycle ends up with a fragmented
completion record. The right fix is probably tighter coupling with the
compact module — that's on the list.

## Closing

I'm posting this less to evangelize a tool and more to put a stake in the
ground: most AI coding tooling today optimizes the inner loop — better
prompts, smarter completions, faster diffs. The outer loop — memory across
sessions, division of labor across agents, calibrated time accounting,
audit independence — is where the real hours bleed. AiPlus is what I built
so I'd bleed fewer of them. If you've solved any of these differently I'd
genuinely like to read your code.
