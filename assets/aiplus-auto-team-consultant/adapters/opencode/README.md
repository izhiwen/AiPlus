# OpenCode Adapter

This directory is the OpenCode adapter source used by the Rust-first `aiplus` CLI.

Most users should install it from their project with:

```bash
aiplus install opencode
```

Then type `刷新` or `refresh` in the already-open OpenCode session.

## What This Adapter Provides

It guides session-local consultation inside an OpenCode project. It does not run agents by itself, edit files by default, publish changes, contact external accounts, or approve Owner-gated actions.

Example dry run:

```text
Use auto-team-consultant. Dry run only. Role=Builder. Prepare a review request for synthetic permission-copy docs. Do not edit files.
```

For packet selection, use `../../core/templates/TEMPLATE_INDEX.md`.

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

## Prompt Map

| Prompt | Use When | Expected Output |
| --- | --- | --- |
| `prompts/auto-team-consultant.md` | You want one entry point for role detection, workflow tiering, lens selection, and Owner-gate checks. | Consultant Packet, routing plan, or requested packet |

## Boundaries

- No global OpenCode config is modified.
- No external services are contacted by this adapter.
- No telemetry is added.
- No automatic agent execution, file edits, publishing, pushing, tagging, release creation, deployment, uploads, or external account contact.
- Stop for Owner approval before Owner-gated actions.
- Simulated pressure-tests must be labeled `SIMULATED_PRESSURE_TEST_ONLY`; they are not real user research, validation, safety approval, or release approval.
