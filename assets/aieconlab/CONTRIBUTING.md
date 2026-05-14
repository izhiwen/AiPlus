# Contributing to AiEconLab

Thanks for considering a contribution to AiEconLab (AEL). The project is
tightly scoped — applied-economics research role separation and execution.
We welcome contributions that stay within that scope.

## Before you start

1. **Open an issue first** for anything larger than a typo fix. AEL has
   a tight scope and we want to avoid well-intentioned PRs that end up
   colliding with related AiPlus modules (`aiplus-agent-team`,
   `aiplus-auto-team-consultant`, etc.). A 1-paragraph issue describing
   what and why is enough.
2. **Run the acceptance test** before opening a PR:
   ```bash
   bash tests/acceptance.test.sh
   ```
   All 15 invariants should pass. If you change one of the structural
   contracts (number of core roles, number of experts, consultant team
   layout, etc.), update `.aiplus/aieconlab/acceptance/v0.1.0/schema.yaml`
   and `tests/acceptance.test.sh` together.

## What kind of contributions fit

✅ **In scope**:
- Persona refinements (Identity, Voice, Forbidden Actions, examples)
- New expert in the directory (with TOML + full persona + acceptance
  invariant) — please open an issue first to discuss whether it fits
- Improvements to the consultant team config (new triggers, refined
  output_artifact contracts, additional scoring signals)
- Bilingual parity (zh-CN ↔ en) when one side drifts
- Runtime adapter implementations (codex / claude-code / opencode)
- Documentation, examples, and worked walkthroughs
- Bug fixes to scripts, schemas, manifests

❌ **Out of scope** (try a different repo or open a discussion first):
- Software-engineering team roles (those belong in
  `aiplus-agent-team`)
- Changes to the AiPlus substrate (agent-memory, compact-reminder,
  auto-team-consultant) — open issues there
- New runtime adapter for a host AiPlus doesn't support yet (please
  discuss in `AiPlus` first)
- Adding teaching, grant-writing, or conference-logistics features
  (out of scope per DESIGN.md §21)

## Persona pattern

Each role config (`core/templates/<role>.toml`) and its persona
(`core/templates/personas/<role>.md`) should mirror the existing
pattern. Core role personas have 6 sections:

1. Identity & Voice
2. Knowledge Boundaries
3. Escalation Behavior
4. Memory Namespace
5. Forbidden Actions
6. Example Prompts and Responses (≥ 3 worked examples)

Expert personas can be lighter (~70-100 lines) but should still cover
Identity, Voice, Knowledge Boundaries, Activation triggers,
Workflow, Escalation, Forbidden Actions, and Examples.

The acceptance test checks that core personas have ≥ 3 examples and
a Forbidden Actions section.

## Bilingual parity

The user-facing READMEs are bilingual (en + zh-CN). When you change
one, please update the other in the same PR. If you can't read or
write Chinese, open an issue describing the change and we'll handle
the parity.

## Adapter parity

If you change the CLI surface or any role's invocation aliases, please
update all three adapter READMEs (`adapters/codex/`,
`adapters/claude-code/`, `adapters/opencode/`).

## Commit messages

Bilingual title preferred when the change is user-facing:
`English title / 中文标题`

Body: explain the *why*, not just the *what*. Reference issues with
`Refs: #N` or `Closes: #N`.

## Where to ask

- **Questions about AEL design**: open a GitHub issue with the
  `question` label
- **Bug reports**: use the bug template
- **Feature ideas**: open an issue with the `enhancement` label —
  discuss before coding
- **Security or IRB-sensitive issues**: see [`SECURITY.md`](SECURITY.md)
