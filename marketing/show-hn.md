# Show HN draft

**Title (max 80 chars):**
Show HN: AiPlus – a multi-agent toolkit I built for my own AI coding workflow

**Body (200–250 words):**

I've been pair-programming with AI coding agents full-time for the better
part of a year, and the same six failures kept costing me hours every
week: state forgotten between sessions, plans erased by mid-task compact,
multiple agents stepping on each other in shared repos, time estimates
anchored to human-engineer hours (so "5 hours" meant 20 minutes — every
time), plans that quietly skipped security and UX until release week, and
one agent forced to wear CEO/reviewer/builder/advisor in the same context
window.

AiPlus is five small Rust modules that treat each failure as a coordination
problem rather than a prompt-engineering one. Everything is project-local
— nothing uploads, nothing syncs to cloud, nothing edits global agent
config.

A few things that might be technically interesting:

- **Auditor independence (三权分立):** the agent that audits another agent's
  work runs from a separate git worktree, reads a different context root,
  and never sees the builder's plan. An auditor that shares context isn't
  auditing.
- **AI-native estimation:** every estimate and actual completion is logged;
  p50/p90 are calibrated from your own history. "5 hours" stops being a
  unit.
- **Real git worktree per agent**, not folder copies — conflicts surface
  as actual git conflicts.

Install:

```
curl -fsSL https://raw.githubusercontent.com/izhiwen/aiplus/main/install.sh | bash
aiplus install codex   # or claude-code, opencode, all
```

Still open: cross-provider auditor independence (same-family priors leak),
multi-day task velocity tracking. Would value pushback from anyone who's
solved either differently.

---

**Posting notes (for owner, not part of post):**

- Best posting window: Tuesday/Wednesday/Thursday, 9–11am US Eastern.
- Front-page lifetime is usually 4–8 hours; first-hour upvotes matter
  most.
- Be in front of the post for the first 60 minutes to answer the early
  technical comments — HN ranks heavily on early engagement.
- "Show HN" prefix is required by HN guidelines; don't drop it.
- Don't ask for upvotes anywhere. Don't post the link on Twitter/X
  simultaneously — it can flag the submission.
