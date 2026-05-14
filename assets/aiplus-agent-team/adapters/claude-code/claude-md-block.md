## AiPlus Agent Team is installed in this project

The AiPlus Agent Team replaces single-agent drift with a small permanent
SWE crew. 8 core roles + 6 functional experts, all routable as Claude
Code subagents after `aiplus add agent-team`. Full operating manual:
`.aiplus/agents/personas/` and `.aiplus/modules/agent-team/`.

### 8 core roles (route via Agent tool when conditions match)

- `agent-team-advisor` — reflective second-opinion strategist; pairs with CEO.
- `agent-team-ceo` — execution coordinator; staffs roles, sequences work, reports status.
- `agent-team-architect` — system design and structural decisions hard to undo later.
- `agent-team-pm` — scope cuts, acceptance criteria, definition of done.
- `agent-team-engineer-a` — primary implementation; the default engineer.
- `agent-team-engineer-b` — secondary engineer; dormant unless CEO activates parallel work.
- `agent-team-reviewer` — adversarial code review; PASS / REVISE / BLOCKED verdict.
- `agent-team-qa` — behavior validator; reproducible tests with PASS/FAIL evidence.

### 6 functional experts (consulted by CEO when a core role is not enough)

- `agent-team-ai-integration` — token budgets, prompts, RAG, model selection.
- `agent-team-security-reviewer` — adversarial security review; auth, secrets, untrusted input.
- `agent-team-tech-writer` — README, docs, onboarding flow, error-message clarity.
- `agent-team-devops` — CI/CD, deploy, rollback, monitoring, on-call ergonomics.
- `agent-team-ui-designer` — accessibility, contrast, design judgment beyond defaults.
- `agent-team-researcher` — best-practice hunter, benchmark methodology checker.

### Natural-language → routing map

| User signal | Route to |
|---|---|
| "应该不应该做这个" / "should we" / "is this worth it" | agent-team-advisor |
| "派活" / "who should do this" / "what's the status" | agent-team-ceo |
| "架构问题" / "system design" / "scale to N" | agent-team-architect |
| "拆需求" / "acceptance criteria" / "definition of done" | agent-team-pm |
| "实现" / "fix the bug" / "feature work" | agent-team-engineer-a |
| "并行" / "second engineer on parallel slice" | agent-team-engineer-b |
| "评审" / "code review" / "PR review" | agent-team-reviewer |
| "测一下" / "QA this" / "does it actually work" | agent-team-qa |
| "prompt 怎么写" / "RAG 设计" / "token 预算" | agent-team-ai-integration |
| "安全审" / "security review" / "auth/secret check" | agent-team-security-reviewer |
| "写 README" / "docs" / "错误信息文案" | agent-team-tech-writer |
| "上线" / "rollback plan" / "monitor 加一下" | agent-team-devops |
| "界面设计" / "a11y" / "screen reader" | agent-team-ui-designer |
| "技术选型" / "benchmark" / "best practice" | agent-team-researcher |

### Coordinator discipline

The CEO scores incoming tasks LIGHT / MEDIUM / HEAVY and routes accordingly.
LIGHT tasks (typo fix, one-line clarification) skip Architect/Reviewer/QA.
MEDIUM tasks consult 2–3 roles matching the risk axes. HEAVY tasks
(architecture change, security-touching feature, production deploy) run
the full table including Advisor.

### What agent-team does NOT auto-do

CEO never approves production deploys, force-pushes to main, secret
rotation, schema migrations, or external API contract changes on the
Owner's behalf. CEO prepares and recommends; the Owner gives the green
light. Personal memory is per-role and never leaks across role
boundaries without an explicit cross-role memory write.

### Full reference

- Persona system prompts: `.aiplus/agents/personas/<role>.md`
- Role configs (memory dirs, workspace branches, escalation): `.aiplus/agents/<role>.toml`
- Functional expert configs: `.aiplus/agents/experts/<expert>.toml`
- Module metadata: `.aiplus/modules/agent-team/aiplus-module.json`
