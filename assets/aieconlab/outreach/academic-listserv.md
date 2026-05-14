# Academic Listserv / Departmental Email

**Audience**: Econ departments, NEP-MET methodology subscribers,
research-tool announcements list (institutional). Formal tone.

**Subject**:
> Open-source release: AiEconLab — AI research-agent toolkit for applied economists

---

## Body

Colleagues,

I'm sharing an open-source release that may be of interest to applied
economists who use AI coding agents in their research workflows.

**AiEconLab (AEL)** [github.com/izhiwen/AiEconLab] is a toolkit that
installs a permanent virtual research team into a project. It is built
on the AiPlus agent-orchestration substrate
[github.com/izhiwen/AiPlus] and provides:

**Eight core research roles**, each with a full persona, knowledge
boundaries, escalation rules, and forbidden actions:

- Advisor (strategic conversation)
- PI (execution coordination)
- Theorist (identification + model structure)
- Project Manager (scope + acceptance criteria + deadlines)
- RA-Stata (main regressions and tables)
- RA-Python (data cleaning and archive ingestion; dormant by default)
- Referee (internal top-tier journal pre-review)
- Replicator (clean-room reproducibility audit)

**Twelve experts on demand**, summoned per-task by the PI when triggers
match: Lit Reviewer, Writer/Editor, Econometrician (deep methodology),
Reproducibility Engineer, Historical Sources Specialist, Job Talk
Coach, Visualization Specialist, Ethics/IRB Reviewer, LLM-as-
Measurement Specialist, and three v0.2 stubs (Survey/Experiment
Specialist, Computation Specialist, Co-Author Liaison).

**Plan-time consultant team** with five seats designed from first
principles for applied-econ review: Design Credibility, Contribution
Framing, Day-1 Reproducibility, IRB / Disclosure Gate, and LLM-as-
Measurement. Fires only at strategic decision points (kickoff,
sample-frame change, pre-submission, R&R receipt); explicitly does
not fire on routine tasks.

**Architectural commitments**:

- All agent state, personas, memories, and worktrees stay local to
  the project's `.aiplus/` directory. No cloud sync, no upload, no
  telemetry, no edits to global agent configuration.
- Twelve STOP-gated actions never auto-execute (journal submission,
  working-paper posting, R&R response sending, data sharing,
  authorship changes, and others) — they always escalate to the
  human PI for explicit approval.
- The IRB / Disclosure Gate seat enforces protocol-scope checks
  before any task touches restricted data.
- Bilingual documentation (English + Mandarin).

**LLM-as-Measurement validity protocol**: For papers using LLMs to
score text data (sentiment, ideology, classification, etc.), the
specialist designs the multi-model cross-validation panel, hand-coded
subsample size, held-out test docs, inter-rater metrics, prompt
versioning, and leakage prevention BEFORE scoring runs. A worked
example using 294 archival 19th-century Classical Chinese documents
scored across five frontier LLMs (pairwise correlations 0.85–0.95) is
released as a companion repository:
github.com/izhiwen/Multi-LLM-Validation-Demo

**Honest disclosure**: AEL is at v0.1.0. Structural acceptance tests
pass and the install flow works on macOS / Linux / Windows. A
full real-world cycle from paper kickoff through submission has not
yet been completed using AEL — the design vs reality gap is unknown.
The maintainer (myself) is a single individual on the academic job
market.

**License**: Apache-2.0. Source code, design document, and worked
examples are public.

**Install** (requires AiPlus 0.5.5 or newer):

```
# Install AiPlus binary
curl -L https://github.com/izhiwen/AiPlus/releases/latest/download/aiplus-aarch64-apple-darwin.tar.gz | tar xz
sudo mv aiplus /usr/local/bin/

# Add AiEconLab to your research project
cd MyResearchProject
aiplus add aieconlab
aiplus install codex    # or claude-code, opencode
```

Feedback, GitHub issues, and discussion are welcome.

Best regards,
Zhiwen Wang
PhD Candidate, Department of Economics
University of Pittsburgh
zhw94@pitt.edu
zhiwen-wang.com
