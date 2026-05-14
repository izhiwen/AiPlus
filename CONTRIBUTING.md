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

## Merge policy & branch protection

The `main` branch is protected (P2.4 of the v0.5.16 follow-up goal).
The settings live in GitHub repo → Settings → Branches and are also
visible via `gh api repos/izhiwen/aiplus/branches/main/protection`.

What's enforced:

- **Required status checks before merge** (all 7 must be green):
  `fmt`, `clippy`, `test`, plus the four `install-smoke (...)` matrix
  jobs on macos-14 / ubuntu-22.04 / ubuntu-24.04 / windows-latest.
  A PR with any red check cannot be merged through the normal flow.
- **No force-pushes to `main`**: `allow_force_pushes=false`.
- **No deletions of `main`**: `allow_deletions=false`.

What's *not* enforced:

- **Reviews aren't required.** Single-maintainer repo; review counts
  would just block work. The PR template's checklist serves the same
  purpose.
- **Admins can override** (`enforce_admins=false`). `gh pr merge --admin`
  bypasses the failing-check gate. This is the escape hatch used when
  a pre-existing test failure on main blocks unrelated PRs. Audit
  trail is the merge commit author + the GitHub Actions logs.

When to admin-merge:

- ✅ A test on main is flaky / pre-existing-broken and your PR's
  failing CI matches that test (verify by running `cargo test` against
  bare main). Note this in the PR description.
- ✅ Emergency rollback that itself fixes the failing test.
- ❌ Your change introduced the failing test — fix the change instead.

Direct pushes to `main` aren't blocked from the admin account (single
maintainer), but every merge should go through a PR so the CI history
and the merge-commit audit trail stay coherent.

## Where to ask

- **Bugs**: use the bug template
- **Feature ideas**: open an issue with the `enhancement` label
- **Security**: see [`SECURITY.md`](SECURITY.md)
- **Architecture / design questions**: open an issue with the
  `question` label
