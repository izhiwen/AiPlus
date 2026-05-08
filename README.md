# AiPlus

AiPlus helps AI coding agents keep project-local memory, handoffs, and review
workflows for Codex, Claude Code, and OpenCode.

`AiPlus` is the product name. `aiplus` is the CLI command, binary, crate, and
repository name.

## Quick Start

Run this from the project where you want to use AiPlus:

```bash
AIPLUS_HOME="$HOME/aiplus"; test -d "$AIPLUS_HOME" || git clone https://github.com/izhiwen/aiplus.git "$AIPLUS_HOME"; (cd "$AIPLUS_HOME" && cargo build --release); "$AIPLUS_HOME/target/release/aiplus" install codex
```

Then type this in the already-open Codex, Claude Code, or OpenCode session for
that same project:

```text
刷新
```

English also works:

```text
refresh
```

If the `aiplus` command is already on your `PATH`, the project install is just:

```bash
aiplus install codex
```

## Runtime Choices

Install AiPlus for one runtime or all supported runtimes:

```bash
aiplus install codex
aiplus install claude-code
aiplus install opencode
aiplus install all
```

Runtime adapters are project-local. Codex uses the project `AGENTS.md` managed
block, Claude Code uses project `.claude/` files, and OpenCode uses project
`.opencode/` files.

## Common Checks

```bash
aiplus status
aiplus doctor
aiplus update
aiplus uninstall --dry-run
```

## What Gets Installed

AiPlus writes only project-local files:

- `.aiplus/`
- `.codex/compact/`
- project `.claude/` adapter files
- project `.opencode/` adapter files
- the AiPlus managed block in project `AGENTS.md`

Bundled modules:

- **AiPlus Auto Compact** (`auto-compact`): compact, checkpoint, validate, and
  resume workflow assets.
- **AiPlus Auto Team Consultant** (`auto-team-consultant`): Advisor, CEO,
  Reviewer, and Builder routing assets.

## Current Install Status

The copy-paste quick start above builds from source because no GitHub Release
binary or installer script has been published from this repository yet.

The old docs used `<AIPLUS_SOURCE>` to mean "the folder where you cloned the
AiPlus repo." Do not type angle-bracket placeholders literally.

## Future Installer Plan

The intended future beginner flow is:

```bash
curl -fsSL https://raw.githubusercontent.com/izhiwen/aiplus/main/install.sh | bash
cd MyProject
aiplus install codex
```

That flow is not active yet. It requires Owner approval for GitHub Release
binaries, checksums, an installer script, and any global/PATH install behavior.
The future installer must not silently edit shell profiles, install project
modules automatically, upload data, add telemetry, or change global Codex,
Claude Code, or OpenCode configuration.

See [Distribution plan](docs/distribution-plan.md) and
[Installer plan](docs/installer-plan.md).

## Developer Build

```bash
git clone https://github.com/izhiwen/aiplus.git
cd aiplus
cargo build --release
```

From a target project:

```bash
~/aiplus/target/release/aiplus install codex
```

## Public-Ready Docs

- [Module index](MODULES.md)
- [Architecture](docs/architecture.md)
- [Public repo plan](docs/public-repo-plan.md)
- [Distribution plan](docs/distribution-plan.md)
- [Installer plan](docs/installer-plan.md)
- [Binary artifact matrix](docs/binary-artifact-matrix.md)
- [Migration from Node CLI](docs/migration-from-node-cli.md)
- [QA release readiness](docs/qa-release-readiness.md)
- [Safety boundaries](docs/safety.md)
- [Release checklist](RELEASE_CHECKLIST.md)

## Node Reference Status

The legacy Node CLI is archived/reference-only at v0.1.3 and is not included in
this public source package. It is retained in the private/local AiPlus workspace
for behavior audits and emergency reference fixes. New CLI work should target
Rust.

Compact commands are Rust-native. Rust runtime assets no longer install or check
`compactctl.mjs`.

## Safety Boundary

AiPlus does not implement publish, push, tag, release creation, global install,
global config edits, telemetry, auto-update, or runtime network fetches.

Validation is structural and heuristic. It is not a safety, privacy,
compliance, correctness, or release certification.
