# Contributing to AiPlus

Thanks for considering a contribution. AiPlus is the agent-orchestration
toolchain that hosts a small family of modules (`agent-memory`,
`compact-reminder`, `auto-team-consultant`, `agent-team`, plus optional
`aieconlab`). Contributions are welcome on all layers.

## Before you start

1. **Open an issue first** for anything beyond a typo fix. AiPlus has
   a strict module boundary discipline (each module is independently
   versioned and tested) — discussion in an issue is faster than
   rework after the PR.
2. **Build and test** locally before opening a PR:
   ```bash
   cargo build --release --bin aiplus
   cargo test --workspace
   ```
3. **Check formatting and lints**:
   ```bash
   cargo fmt --all -- --check
   cargo clippy --workspace -- -D warnings
   ```

## What kind of contributions fit

✅ **In scope**:
- Bug fixes (CLI, agent commands, install/doctor/refresh)
- New runtime adapter (codex/claude-code/opencode parity changes)
- Improvements to existing modules (within each module's scope)
- Documentation, examples, demo recordings
- CI / release pipeline improvements
- New community files (templates, guides)

❌ **Out of scope** (or open an issue first to discuss):
- New top-level commands that overlap with existing ones — try to
  fit into the existing surface
- New external services / network calls — AiPlus stays local-first
- Anything that edits global config (`~/.codex/`, `~/.claude/`, etc.)
- Anything that requires elevated permissions / sudo

## Module boundary discipline

If you're changing more than one module in one PR, please ask first
whether splitting it makes sense. The module manifests (`MODULES.md`,
each `aiplus-module.json`) are the source of truth for boundaries.

For changes that affect the `aieconlab` module specifically, the
canonical home is the [AiEconLab repo](https://github.com/izhiwen/AiEconLab) —
this repo's `assets/aieconlab/` is a vendored snapshot rebuilt from
that source.

## Commit messages

Bilingual title preferred for user-facing changes:
`English title / 中文标题`

Body: explain the *why*, not just the *what*.

## Where to ask

- **Bugs**: use the bug template
- **Feature ideas**: open an issue with the `enhancement` label
- **Security**: see [`SECURITY.md`](SECURITY.md)
- **Architecture / design questions**: open an issue with the
  `question` label
