# r/Econometrics / r/Economics Post

**Subreddit candidates**:
- r/Econometrics (smaller, methodologically focused — best)
- r/Economics (larger, less technical — secondary)
- r/EconPapers (working paper crowd — tertiary)
- r/AskEconomics (probably wrong audience)

**Title**:
> AiEconLab: open-source AI research-agent toolkit for applied economists. PI / Theorist / RA-Stata / Referee / Replicator. Includes LLM-as-Measurement Specialist with validity protocol for text-as-data work.

---

## Body

I'm an applied-economics PhD candidate (Pitt, working on econ history).
I built AiEconLab (AEL) because I spent the past year watching AI
coding agents drift through my research workflow the same way they
drift through software workflows — but the failure modes for research
are worse:

- The agent writes the regression before identification is settled
- The agent invents references and lit citations
- Robustness checks get retrofit instead of designed at plan time
- LLM-scored variables enter the regression with no validity protocol
- IRB / restricted-data boundaries get crossed silently

AEL installs a permanent virtual research team into your project:

**8 core roles** (each with a full persona, knowledge boundaries,
escalation rules, and 5 worked examples):

- **Advisor** — strategic conversation, framing (you talk to me directly)
- **PI** — execution coordinator, dispatches and reports
- **Theorist** — identification, model structure, falsification design
- **PM** — scope, acceptance criteria, deadlines
- **RA-Stata** — main regressions, tables, figures
- **RA-Python** — data cleaning, scraping, archive ingestion (dormant by default)
- **Referee** — internal top-5 / field-top journal pre-review
- **Replicator** — clean-room reproducibility audit

**12 experts on-demand**: Lit Reviewer, Writer, Econometrician (deep —
weak IV, BJS imputation, shift-share inference), Reproducibility
Engineer, Historical Sources Specialist, Job Talk Coach, Visualization
Specialist, Ethics/IRB Reviewer, **LLM-as-Measurement Specialist**, and
3 v0.2 stubs (Survey/Experiment, Computation, Co-Author Liaison).

**Plan-time consultant team** with 5 seats designed from first principles
for applied-econ review:

1. Design Credibility — half-page yes/no on identification credibility
2. Contribution Framing — 3-5 closest comparables, one differential claim
3. Day-1 Reproducibility — Makefile/env/seed/archive provenance from day 1
4. IRB / Disclosure Gate — protocol-scope + small-cell risk
5. LLM-as-Measurement — fires only when LLMs are measurement
   instruments; validity battery design before scoring runs

The LLM-as-Measurement Specialist is the one I want to highlight here.
If you've used LLMs to score open-ended survey responses, archival text,
political ideology, sentiment, etc., the validity question — "how do you
know the score measures what you say it measures, not GPT's idiosyncratic
training data?" — kills more papers at desk-reject than any other issue.
The Specialist owns: multi-model panel design, hand-coded subsample,
held-out test docs, inter-rater agreement metrics, prompt versioning,
leakage prevention, AEA Data Editor compatibility. Worked example
attached as a sibling repo:
[Multi-LLM-Validation-Demo](https://github.com/izhiwen/Multi-LLM-Validation-Demo)
— 294 19th-century Classical Chinese archival documents scored across
GPT/Claude/Gemini/Qwen/DeepSeek, pairwise correlations 0.85–0.95.

**Default toolchain**: Python + Stata + LaTeX (R + Julia supported when
declared).

**STOP-gates that the team never auto-approves**: journal submission,
working-paper posting, R&R response sending, data sharing, authorship
changes. Plus 7 more in DESIGN.md §16.

**What this is NOT**:
- A replacement for your judgment as PI
- A reference manager
- A statistical analysis tool (it *coordinates* tools; doesn't replace them)
- IRB advice (we surface the question; you and your institutional IRB office decide)

**Install**:

```
# Install AiPlus first (>= 0.5.5)
curl -L https://github.com/izhiwen/AiPlus/releases/latest/download/aiplus-aarch64-apple-darwin.tar.gz | tar xz
sudo mv aiplus /usr/local/bin/

# Add AiEconLab to your research project
cd MyResearchProject
aiplus add aieconlab
aiplus install codex   # or claude-code, opencode
```

Apache-2.0. Open source. Honest disclaimer: I'm 1 person, on the job
market, and the design hasn't been battle-tested on a full paper-
to-submission cycle yet. The structural tests pass, the install flow
works, the personas are written; what I don't know is which design
choices break under real R&R deadline pressure.

If you're applied econ and try it, the GitHub issue tracker is the
fastest way to send back what worked and what didn't.

**Repo**: https://github.com/izhiwen/AiEconLab
**Main platform**: https://github.com/izhiwen/AiPlus
**Sibling worked example**: https://github.com/izhiwen/Multi-LLM-Validation-Demo
