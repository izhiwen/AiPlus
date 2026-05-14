# EJMR Post

**Caveat first**: EJMR is a culture-specific forum with its own norms.
Use only if you have a specific reason to (e.g., the AI thread is
already discussing tools and you want to add a credible data point).
Avoid if your goal is general signal-boost — Twitter and Bluesky reach
more econ readers per unit effort, and don't carry EJMR's reputation
overhead.

**If you do post**:

- Use a throwaway acct (EJMR convention)
- Don't lead with "I built". Lead with the problem.
- Be ready for sneering. Don't engage emotionally.

---

## Thread title options

> AI research agents for applied econ — a tool that actually
> understands what a referee asks

> Open source: PI / Theorist / RA / Referee personas for AI coding
> agents in research workflows

> Got tired of GPT/Claude inventing citations in lit reviews. Built
> something.

---

## Body

Anyone else hit the wall where AI coding agents are great for
implementation but terrible at the rest of the research workflow?

Mine kept doing the three predictable things:

(a) writing the regression spec before identification was settled
(b) hallucinating papers in literature reviews ("Smith and Jones (2019)
   show ..." — paper doesn't exist)
(c) treating LLM-scored variables like they're hand-coded ground truth

So I built an open-source toolkit, AiEconLab (AEL), that installs a
virtual research team into your project. Personas have explicit
forbidden actions, escalation rules, and STOP-gates (no auto-
submission, no auto-WP-posting, no auto-data-sharing, no auto-
authorship changes).

Includes a LLM-as-Measurement Specialist for papers using LLMs to
score text data — designs the validity battery before scoring runs.
Worked example: scored 294 archival 19th-century Chinese documents
across 5 frontier LLMs, pairwise correlations 0.85–0.95, used as the
validity backbone for my JMP.

Default Python + Stata + LaTeX. Apache-2.0. Single maintainer (me).

Real-world battle-test on a full paper-to-submission cycle: not done
yet. Design vs reality gap unknown. The personas pass structural tests
but I haven't seen them fail under real R&R deadline pressure yet.

Repo: github.com/izhiwen/AiEconLab
Sibling worked example: github.com/izhiwen/Multi-LLM-Validation-Demo

Curious if anyone here has been burned by LLM-derived measurements in
their own work and whether the validity-protocol-as-persona approach
seems useful or theatrical.
