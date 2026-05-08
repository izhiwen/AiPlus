# AiPlus Rust CLI

This repository is the Rust mainline workspace for the AiPlus `aiplus` binary.

Recommended public repo name: `aiplus`.

The package/crate name remains `aiplus-cli` for now. The binary name is `aiplus`.

License: Apache-2.0. The license applies to the Rust mainline/public-ready
package in this workspace. Bundled child module snapshots preserve their
existing licenses. Licensing is not a safety, privacy, compliance, correctness,
or release certification.

## Beginner Flow

Build from source:

```bash
cd aiplus
cargo build --release
```

Install AiPlus into a project:

```bash
cd MyProject
<AIPLUS_SOURCE>/target/release/aiplus install codex
```

If a local test binary is already on PATH:

```bash
cd MyProject
aiplus install codex
```

Then type this in an already-open Codex, Claude Code, or OpenCode session in the
same project:

```text
刷新
```

English also works:

```text
refresh
```

## Runtime Installs

```bash
aiplus install codex
aiplus install claude-code
aiplus install opencode
aiplus install all
```

Runtime adapters are project-local: Codex updates the project `AGENTS.md`
managed block, Claude Code writes project `.claude/` files, and OpenCode writes
project `.opencode/` files.

Aliases are preserved for compatibility:

```bash
aiplus install claude
aiplus install cc
aiplus install oc
aiplus install --runtime codex
aiplus install --all-runtimes
```

## Maintenance Commands

```bash
aiplus status
aiplus doctor
aiplus update
aiplus update auto-compact
aiplus update auto-team-consultant
aiplus add auto-compact
aiplus add auto-team-consultant
aiplus compact validate
aiplus compact checkpoint
aiplus compact resume
aiplus uninstall --dry-run
```

## Public-Ready Planning Docs

- [Architecture](docs/architecture.md)
- [Public repo plan](docs/public-repo-plan.md)
- [Distribution plan](docs/distribution-plan.md)
- [Binary artifact matrix](docs/binary-artifact-matrix.md)
- [Migration from Node CLI](docs/migration-from-node-cli.md)
- [QA release readiness](docs/qa-release-readiness.md)
- [Safety boundaries](docs/safety.md)
- [Release checklist](RELEASE_CHECKLIST.md)

## Repository Layout

```text
aiplus/
  README.md
  Cargo.toml
  Cargo.lock
  crates/aiplus-cli/
  assets/
  docs/
  tests/
  CHANGELOG.md
  RELEASE_CHECKLIST.md
```

## Node Reference Status

The legacy Node CLI is archived/reference-only at v0.1.3 and is not included in
this public source package. It is retained in the private/local AiPlus workspace
for behavior audits and emergency reference fixes. New CLI work should target
Rust.

Compact commands are Rust-native. Rust runtime assets no longer install or check
`compactctl.mjs`.

## Safety Boundary

The CLI writes only project-local files:

- `.aiplus/`
- `.codex/compact/`
- project `.claude/` adapter files
- project `.opencode/` adapter files
- AiPlus managed block in project `AGENTS.md`

It does not implement publish, push, tag, release creation, global install,
global config edits, telemetry, auto-update, or runtime network fetches.
