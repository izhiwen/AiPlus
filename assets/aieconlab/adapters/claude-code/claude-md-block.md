## AiEconLab (AEL) is installed in this project

AiEconLab replaces single-agent drift with a permanent virtual research
team. 8 core roles + 12 expert specialists, all routable as Claude Code
subagents after `aiplus add aieconlab`. Full operating manual:
`.aiplus/agents/personas/` and `.aiplus/modules/aieconlab/`.

### 8 core roles (route via Agent tool when conditions match)

- `aieconlab-advisor` — reflects on framing and identification; pairs with PI.
- `aieconlab-pi` — owns task scoping, dispatch, milestone tracking; escalates Owner-gated actions.
- `aieconlab-theorist` — identification strategy, model, formalization.
- `aieconlab-pm` — scope, deadlines, milestones, status reporting.
- `aieconlab-ra-stata` — regression specs, main empirical analysis.
- `aieconlab-ra-python` — data cleaning, scraping, merging, GIS.
- `aieconlab-referee` — internal pre-submission devil's-advocate review.
- `aieconlab-replicator` — clean-room rerun for replication packages.

### 12 expert specialists (consulted by PI when a core role is not enough)

- `aieconlab-coauthor-liaison` · `aieconlab-computation` · `aieconlab-econometrician` (deep ID, weak-IV, SE theory)
- `aieconlab-ethics-irb` · `aieconlab-historical-sources` · `aieconlab-job-talk-coach`
- `aieconlab-lit-reviewer` · **`aieconlab-llm-measurement`** (the AEL headline expert) · `aieconlab-reproducibility`
- `aieconlab-survey-experiment` · `aieconlab-viz-specialist` · `aieconlab-writer`

### Natural-language → routing map

| User signal | Route to |
|---|---|
| "扫一下我这个识别有问题吗" / "is my ID strategy ok" | aieconlab-theorist (or aieconlab-econometrician for inference-level depth) |
| "帮我清这份数据" / "scrape this site" | aieconlab-ra-python |
| "跑主回归" / "main regression spec" | aieconlab-ra-stata |
| "投稿前挑刺" / "pre-review this rebuttal" | aieconlab-referee |
| "为复现打包" / "ship replication package" | aieconlab-replicator (run) + aieconlab-reproducibility (build apparatus) |
| "用 LLM 给文本打分" / "LLM-as-measurement" | aieconlab-llm-measurement |
| "RCT 设计" / "power analysis" / "pre-registration" | aieconlab-survey-experiment |
| "这图讲不清楚" / "figure isn't working" | aieconlab-viz-specialist |
| "改写引言" / "rewrite intro" | aieconlab-writer |
| "需要 IRB 评估" / "anonymize" / "restricted data" | aieconlab-ethics-irb |

### Coordinator discipline

The PI scores incoming tasks LIGHT / MEDIUM / HEAVY and routes accordingly.
LIGHT tasks (typo fix, one-line clarification) skip the consultant team
entirely. MEDIUM tasks consult 2–3 experts matching the risk axes. HEAVY
tasks (paper plan, major revision, identification change) run the full
table including user personas. AEL ships a research-tuned consultant-team
config that replaces AiPlus's default SWE consultant team.

### What AEL does NOT auto-do

PI never approves journal submission, working-paper posting, sending a
referee response, data sharing, or authorship-order changes on the
Owner's behalf. PI prepares and recommends; the Owner gives the green
light. Personal memory is per-role and never leaks across role
boundaries without an explicit cross-role memory write.

### Toolchain expectations

Default toolchain: Python + Stata + LaTeX. RA-Stata writes Stata code;
ra-python writes Python; writer/viz-specialist output LaTeX-ready text
and figures. Subagents should respect this convention unless the Owner
asks for a different language.

### Full reference

- Persona system prompts: `.aiplus/agents/personas/<role>.md`
- Role configs (memory dirs, workspace branches, escalation): `.aiplus/agents/<role>.toml`
- Consultant team config: `.aiplus/consultant-team.toml` (AEL-tuned at install time)
- AEL module metadata: `.aiplus/modules/aieconlab/aiplus-module.json`
