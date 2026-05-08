# Claude Code Adapter

This directory is the Claude Code adapter source used by the Rust-first `aiplus` CLI.

Most users should install it from their project with:

```bash
aiplus install claude-code
```

Then type `刷新` or `refresh` in the already-open Claude Code session.

## What This Adapter Provides

It guides session-local consultation in Claude Code. It does not install anything globally, run agents by itself, edit files by default, publish changes, contact external accounts, or approve Owner-gated actions.

Example dry run:

```text
Use auto-team-consultant. Dry run only. Role=Advisor. Review this synthetic onboarding prompt for calendar access. Return Consultant Packet only. Do not edit files.
```

For packet selection, use `../../core/templates/TEMPLATE_INDEX.md`.

## Reference Files

- `commands/team-consult.md`: general Consultant Packet routing
- `commands/advisor-handoff.md`: Advisor-to-CEO handoff support
- `commands/ceo-routing.md`: CEO task routing support
- `commands/pressure-test.md`: simulated pressure-test prompt
- `agents/team-advisor.md`: team advisor lens
- `agents/process-reviewer.md`: process and routing reviewer lens
- `skills/auto-team-consultant/SKILL.md`: Skill-style operating rules

## Command Map

| Command | Use When | Expected Output |
| --- | --- | --- |
| `commands/team-consult.md` | You need role detection, LIGHT/MEDIUM/HEAVY routing, lens selection, or a concise recommendation. | Consultant Packet |
| `commands/advisor-handoff.md` | Advisor or Reviewer output should transfer execution coordination to CEO mode. | CEO-ready handoff |
| `commands/ceo-routing.md` | CEO mode needs to decompose work, define file claims, and request result packets. | routing plan and Task Cards |
| `commands/pressure-test.md` | User-facing perception risk needs simulated stakeholder input. | `SIMULATED_PRESSURE_TEST_ONLY` findings |

## Agent Map

| Agent | Use When | Notes |
| --- | --- | --- |
| `agents/team-advisor.md` | You need a broad team-routing lens before choosing a path. | Keep advice concise and role-aware. |
| `agents/process-reviewer.md` | You need process, handoff, Owner gate, or routing QA. | Return blockers, risks, missing checks, and verdict where requested. |

## Boundaries

- Project-local docs/templates only; no global Claude Code config modification by default.
- No automatic agent execution, file edits, publishing, pushing, tagging, release creation, deployment, telemetry, uploads, or external account contact.
- Stop for Owner approval before Owner-gated actions.
- Pressure-Test output is synthetic only and must be labeled `SIMULATED_PRESSURE_TEST_ONLY`; it is not real user research, validation, safety approval, or release approval.
