# Template Index

Use this chooser when a session asks "which template should I use?" Pick the smallest artifact that matches the role, workflow tier, and expected output.

Start with
[`../docs/consultant-team-decision-system.md`](../docs/consultant-team-decision-system.md)
when the session needs the full project-specific consultant architecture,
Router scoring, AI Integration lens, User Evidence Layer, or trigger
accountability.

| Template | Use When | Session Role | Workflow Tier | Output |
| --- | --- | --- | --- | --- |
| `../docs/consultant-team-decision-system.md` | full consultant architecture and routing policy | any | any | system design |
| `consultant-team.default.toml` | local project routing policy seed | any | any | config template |
| `consultant-packet.md` | advice/routing decision | Advisor/Reviewer | LIGHT/MEDIUM | Consultant Packet |
| `ceo-handoff.md` | hand off execution to CEO | Advisor/Reviewer | MEDIUM/HEAVY | CEO-ready handoff |
| `ceo-routing.md` | CEO decomposes work | CEO | MEDIUM/HEAVY | routing plan |
| `task-card.md` | dispatch scoped work | CEO | MEDIUM/HEAVY | Task Card |
| `result-packet.md` | agent returns work | Builder/Reviewer/Worker | LIGHT/MEDIUM/HEAVY | Result Packet |
| `gate-packet.md` | prompt/review gate | Reviewer/CEO | MEDIUM/HEAVY | PASS/REVISE/BLOCKED gate |
| `pressure-test.md` | simulated user-facing perspective | Advisor/Reviewer/CEO | MEDIUM/HEAVY | SIMULATED_PRESSURE_TEST_ONLY |
| `workflow-tiers.md` | choose LIGHT/MEDIUM/HEAVY | any | any | tier decision |
| `team-architecture.md` | choose lenses/teams | Advisor/CEO | MEDIUM/HEAVY | lens/team plan |
| `compact-recovery-note.md` | summarize after compact | any | any | recovery note |

## Selection Rules

- Start with `../docs/consultant-team-decision-system.md` when the task is
  product/AI/trust/release oriented or when the project needs a custom
  consultant team.
- Start with `workflow-tiers.md` when only the depth is unclear.
- Use `consultant-packet.md` for advice, prompt critique, routing judgment, and lightweight review.
- Use `ceo-handoff.md` only when the current session should transfer execution coordination to CEO mode.
- Use `ceo-routing.md` and `task-card.md` together when CEO mode decomposes work for multiple agents or file scopes.
- Use `result-packet.md` when Builder, Reviewer, or Worker output needs to return verifiable work status.
- Use `gate-packet.md` for explicit PASS/REVISE/BLOCKED gates.
- Use `pressure-test.md` only for simulated user-facing perspective work, and always label output `SIMULATED_PRESSURE_TEST_ONLY`.

Templates are decision-support artifacts. They do not automate agents, publish changes, approve releases, contact external accounts, modify global config, or replace Owner review.
