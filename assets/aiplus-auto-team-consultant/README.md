# AiPlus Auto Team Consultant

AiPlus Auto Team Consultant is an independent AiPlus subproduct and project-local module for already-open AI agent sessions. It helps the current agent decide whether a task needs a quick check, one focused specialist view, or a bounded team discussion.

It is part of the AiPlus ecosystem, and it can also be understood or adopted by itself from this repo. AiPlus is the main ecosystem and CLI distribution entry. AiPlus Auto Team Consultant is one module in that family; it does not pretend to be the full AiPlus CLI.

It is not a separate running app, does not upload your data, does not change global Codex / Claude Code / OpenCode settings, and does not automatically execute dangerous actions.

## Start Here

Use this when you want your current AI agent session to make better routing decisions:

- Should this be `LIGHT`, `MEDIUM`, or `HEAVY`?
- Is direct advice enough, or should one specialist lens review it?
- Does a CEO prompt need review before execution?
- Is there an Owner gate or safety boundary?
- Would simulated pressure-test input help?

After refresh, the agent should behave differently by using these local instructions to choose the smallest useful review depth, explain skipped lenses, label simulated pressure-tests, and stop before Owner-gated actions.

## Path A: Recommended AiPlus Ecosystem Path

Install AiPlus first, then install this module into your project. Replace `MyProject` with your project folder:

```bash
curl -fsSL https://raw.githubusercontent.com/izhiwen/aiplus/main/install.sh | bash
cd MyProject
aiplus install codex
```

Then in your already-open agent session, type:

```text
aiplus refresh
```

Chinese triggers are also supported:

```text
AiPlus 刷新
刷新 AiPlus
```

If the project already has an older AiPlus install, `aiplus install codex` safely upgrades AiPlus managed files, backs up replaced managed files under `.aiplus/backups/`, and preserves existing `.codex/compact/` state.

## Path B: Existing `aiplus` Command

From your project. Replace `MyProject` with your project folder:

```bash
cd MyProject
aiplus install codex
```

Then in your already-open agent session, type:

```text
aiplus refresh
```

Other explicit AiPlus refresh triggers:

```text
AiPlus status
resume AiPlus
AiPlus 刷新
刷新 AiPlus
aiplus status
继续 AiPlus
```

Generic `刷新` / `refresh` should still try AiPlus first after installation. If
your project also uses `刷新` for its own state refresh, use `AiPlus 刷新` or
`aiplus refresh` to avoid ambiguity. That tells the current agent session to
report AiPlus status before unrelated project refresh, reload the project-local
AiPlus instructions, report Auto Team Consultant status, and use the Auto Team
Consultant routing behavior.

## Path C: Advanced Module-Only Adoption

If you only want AiPlus Auto Team Consultant, you can use this repo directly as a reference source. Advanced users can inspect or copy the project-local templates, skills, prompts, adapter files, and synthetic examples into their own workflow.

This is not the ordinary install path. Most users should use the AiPlus CLI path above.

## Runtime Choices

```bash
aiplus install codex
aiplus install claude-code
aiplus install opencode
aiplus install all
```

Use the runtime that matches the agent you are using. `all` installs project-local support for all three.

## What It Does

AiPlus Auto Team Consultant gives the current agent a simple routing protocol:

- `LIGHT`: quick check for simple tasks
- `MEDIUM`: focused review for important prompts, docs, plans, or implementation choices
- `HEAVY`: full council for high-risk or major decisions

The goal is practical judgment, not bureaucracy. The agent should default to `LIGHT` and escalate only when the task risk justifies it.

It helps the current agent decide:

- when to use a quick single-perspective check
- when to ask one focused specialist view
- when to run a bounded team discussion
- when a CEO Prompt needs review
- when a safety or Owner gate is required
- when simulated pressure-test input is useful

## When To Use It

Use it when you want the current agent to:

- review a prompt before execution
- decide whether a specialist lens is needed
- prepare a CEO-style task handoff
- separate blockers from concerns during review
- ask for review after implementation work
- pressure-test user-facing copy or flow as simulated input
- keep Owner-gated actions explicit

## What To Type In The Agent Session

After running `aiplus install ...`, try:

```text
Use auto-team-consultant. Role=Advisor. Review this synthetic onboarding prompt for calendar access. Return Consultant Packet only. Do not edit files.
```

Or for a Builder handoff:

```text
Use auto-team-consultant. Role=Builder. Summarize changed files, verification run, known risks, and who should review next.
```

For compact readiness, ordinary users do not need to remember compact commands.
Natural language is the primary interface:

```text
prepare compact
save progress
continue after compact
```

The agent should use AiPlus backend tools such as `aiplus compact prepare` before
compact and `aiplus compact resume` after compact. These commands are internal
tools for agents, manual fallbacks for advanced users, and maintainer debugging
commands. AiPlus Auto Team Consultant should preserve role context in compact
handoffs: Advisor recommendations, CEO task cards, Reviewer findings, Builder
changed files, Owner gates, and the next safe action.

For older projects, AiPlus upgrades legacy compact handoffs conservatively during
`aiplus install ...` and `aiplus update`: it backs up the old handoff, preserves
user content, and adds missing role-aware fields. If compact readiness is blocked
by a real safety issue or denied Owner gate, AiPlus should report
`BLOCKED_DO_NOT_COMPACT` rather than creating an ordinary checkpoint.

For compact savings, users can ask:

```text
show compact savings
how many tokens did compact save?
```

The agent should map that to `aiplus compact savings`. Savings are estimates
only, not billing data and not proof of workflow quality. Auto Team Consultant
may use savings as operating context, but it must not treat savings as evidence
that a review, CEO plan, or release gate is correct.

For AiPlus updates, users can say:

```text
update AiPlus
```

Default mapping is `aiplus update all`. More specific mappings:

- `only update this project's AiPlus` -> `aiplus update`
- `update the aiplus command` -> `aiplus self update`
- `check AiPlus updates` -> `aiplus self update --dry-run` plus `aiplus status`

Before running updates, the agent should state that it will not edit global agent
config or upload project data.

For private profile and secret status, users can say:

```text
work-with-zhiwen status
secret status
check API keys
```

The agent should map those to metadata-only checks such as
`aiplus profile status`, `aiplus secret-broker status`, or
`aiplus secret-broker doctor`. Auto Team Consultant may use a user-level profile
as lower priority than the current Owner message and project rules, but it must
not copy private profile material into public docs, task packets, compact files,
or result packets.

If a task explicitly needs a key, the agent should prefer
`aiplus secret-broker run -- <command...>` so approved values enter only the
child process environment. It must never print, paste, log, summarize, compact,
or persist secret values.

## Roles In Plain Language

- `Advisor`: gives direct advice, prompt review, strategy, or CEO-ready handoff
- `CEO`: breaks work into scoped tasks, routes agents, integrates result packets
- `Reviewer`: reports findings, blockers, risks, and missing tests
- `Builder`: reports changed files, verification run, known risks, and review request

## Pressure-Test

Pressure-Test means simulated stakeholder input for user-facing perception risk.

Every pressure-test must be labeled:

```text
SIMULATED_PRESSURE_TEST_ONLY
```

It is not real user research, validation, safety approval, accessibility approval, or release approval.

## Project-Local Safety Boundary

AiPlus Auto Team Consultant is session-local decision-support.

It does not:

- automatically spawn agents by itself
- upload data
- add telemetry
- change global agent settings
- publish, push, tag, release, or deploy by itself
- approve Owner-gated actions
- replace Owner decisions
- perform real user research
- guarantee safety, compliance, correctness, privacy, legal readiness, product quality, or public-release readiness

The current agent remains responsible for scope control, verification, and Owner-gated actions.

## Runtime Support

| Runtime | Install command | What gets added | Automation level |
| --- | --- | --- | --- |
| Codex | `aiplus install codex` | project-local Codex instructions | session-local |
| Claude Code | `aiplus install claude-code` | project-local Claude Code commands/instructions | project-local |
| OpenCode | `aiplus install opencode` | project-local OpenCode commands/prompts | project-local |
| All | `aiplus install all` | all supported runtime files | project-local |

## Advanced: Core And Adapters

Most users should install with `aiplus install ...`.

This repo also keeps the reusable source files:

- `core/docs/`: runtime-neutral protocol docs
- `core/templates/`: packet and routing templates
- `adapters/codex/`: Codex instruction source
- `adapters/claude-code/`: Claude Code project-local command and agent source
- `adapters/opencode/`: OpenCode project-local config, command, agent, and prompt source
- `examples/`: synthetic examples only

If you are unsure which packet to use, see [core/templates/TEMPLATE_INDEX.md](core/templates/TEMPLATE_INDEX.md).

## Validate This Repo Locally

Maintainers can run:

```bash
find . -maxdepth 5 -type f | sort
python3 -m json.tool adapters/codex/.codex-plugin/plugin.json >/dev/null
python3 -m json.tool adapters/claude-code/.claude-plugin/plugin.json >/dev/null
python3 -m json.tool adapters/opencode/opencode.json.example >/dev/null
find . -maxdepth 5 -type f \( -name "package.json" -o -name "*.mjs" -o -name "*.js" -o -name "*.sh" \)
```

Expected: JSON parses, examples are synthetic, and no package automation appears.

## Current Status

This repo is the public source module for AiPlus Auto Team Consultant. The preferred user path is the Rust-first `aiplus` CLI:

```bash
aiplus install codex
```

No npm package, package registry publish, GitHub Release, git tag, marketplace submission, global install, telemetry, MCP server, App connector, or autonomous executor is included.
