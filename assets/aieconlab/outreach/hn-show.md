# Hacker News Show HN Post

**Title** (80 char max, HN counts strictly):
> Show HN: AiPlus – AI agent toolchain for coding workflows (with an econ-research sibling)

Alternative titles:
> Show HN: AiPlus – CLI that gives AI coding agents a permanent team, memory, and budget
> Show HN: I built a CLI to stop AI coding agents from forgetting, drifting, and over-quoting

Pick the most concrete one. HN rewards specificity in titles.

**URL**: `https://github.com/izhiwen/AiPlus`

---

## Body (first comment, posted immediately after submission)

I've been pair-programming with AI coding agents full-time for the better
part of a year — Codex one day, Claude Code the next, OpenCode for the
long-running stuff. About four months in I caught myself explaining the
same architectural decision to the same agent for the fourth time that
week, and I realized I was losing hours every day to the same six
coordination failures:

1. Cross-session amnesia (Monday it knew, Wednesday it forgot)
2. Post-compact context loss (mid-feature, 40 minutes of progress gone)
3. Multiple agents racing to lead the same task (no division of labor)
4. Estimates anchored to human-engineer hours (says "5 hours", finishes
   in 20 minutes; says it again next week, same problem)
5. Plans that skip security/UX/AI-integration concerns until release week
6. One agent wearing every hat in the same context window (drift +
   pollution + each hat done shallowly)

AiPlus is five small Rust modules I built to treat all six. Everything
stays local to `.aiplus/` in your project. No daemon, no cloud sync, no
global config edits, no telemetry, no upload.

The meta-frame I'll cop to: I used AI agents to build the toolchain that
manages AI agents. That recursion has been the most useful debugging
loop of the whole exercise — every flaw the toolchain has, I hit
personally before patching it.

Multi-platform v0.5.9 just shipped: pre-built binaries for macOS (Intel
+ Apple Silicon), Linux (x86_64 + aarch64), and Windows. Each module is
independently installable. Plugin authors can add third-party modules
without rebuilding the CLI via `aiplus add --from-git <URL>`.

There's also a research-side spin-off, **AiEconLab**, for applied
economists — different role vocabulary (PI / Theorist / RA-Stata /
Referee / Replicator instead of CEO / Architect / Engineer / QA),
different consultant team designed from first principles for plan-time
research review, and a specialized LLM-as-Measurement Specialist for
papers that use LLMs to score text data. Sibling repo:
github.com/izhiwen/AiEconLab.

What I'd love feedback on:

- The "permanence model" tradeoff (state-level permanent on disk,
  process-level ephemeral). Anyone tried alternatives?
- The consultant-before-plan pattern. Useful in your workflows or
  noise?
- The opt-in module split (AiEconLab is bundled but not auto-installed
  — most AiPlus users are SWEs, AEL would be left-handed scissors in
  every kitchen drawer).

Known gaps in v0.5.9:
- Real-world battle test on the maintainer's actual research papers
  hasn't happened yet — designs vs reality gap is unknown
- v0.2 stub experts (Survey/Experiment, Computation, Co-Author Liaison)
  are config stubs, not full personas
- No multi-machine sync (state-local by design, but if you want it,
  it's not there)

Repo: github.com/izhiwen/AiPlus

---

## Prepared follow-up replies

**If asked "why not use the existing X?":**
> Fair. I tried [existing tool] for about two weeks before this. The
> friction was [specific failure mode]. The thing that pushed me to
> build my own was: I wanted the agent-team metaphor (permanent roles
> with isolated workspaces) to fail visibly through git instead of
> silently through context pollution. Existing tools optimize for
> "one chat, infinite context" — AiPlus optimizes for "many agents,
> bounded context each".

**If asked "what does the consultant team actually do?":**
> It's a virtual review board fired at plan time. For SWE projects:
> Architecture / UX / Security / Pitfall / AI Integration weigh in
> before the agent writes the plan. The consultant doesn't write code;
> it surfaces "have you thought about X" questions early. The same
> engine in AEL has 5 econ-specific seats (Design Credibility,
> Contribution Framing, Day-1 Reproducibility, IRB Gate, LLM-as-
> Measurement) — same mechanism, different domain. README has a
> worked example showing the 5 outputs.

**If asked "is this a serious project or a side project?":**
> Side project that's become my daily driver. v0.5 series shipped 9
> releases in 5 days. I use it for my own research work. Whether you
> should depend on it for production work — your call; I'm a single
> maintainer and explicit about that.

**If asked "what about Aider / Cursor / Continue?":**
> Different layer. Those are IDE-level pair-programmers. AiPlus is
> orchestration *around* them — it works alongside Codex / Claude
> Code / OpenCode by giving them a persistent memory, a compact
> handoff format, a virtual team to delegate to. You'd still use
> Aider for inline code completion; AiPlus is for the longer-running
> "build this feature end-to-end" workflows.
