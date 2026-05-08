# AiPlus

AiPlus helps AI coding agents keep project-local memory, handoffs, and review
workflows for Codex, Claude Code, and OpenCode.

`AiPlus` is the product name. `aiplus` is the CLI command, binary, crate, and
repository name.

## Quick Start

Install the `aiplus` command:

```bash
curl -fsSL https://raw.githubusercontent.com/izhiwen/aiplus/main/install.sh | bash
```

Then install AiPlus into your project:

```bash
cd MyProject
aiplus install codex
```

If the project already has an older AiPlus install, the same command safely
upgrades AiPlus managed files, creates backups under `.aiplus/backups/`, and
preserves `.codex/compact/` state.

Then type this in the already-open Codex, Claude Code, or OpenCode session for
that same project:

```text
AiPlus refresh
```

When you want to compact or save progress, stay in the agent session and say:

```text
prepare compact
```

or:

```text
save progress
```

After compact, if the agent does not reply, say:

```text
continue
```

Chinese equivalents also work:

```text
AiPlus 刷新
帮我准备 compact
保存进度
继续
```

Generic `刷新` / `refresh` should still try AiPlus first after installation. If
your project also uses `刷新` for its own state refresh, use `AiPlus 刷新` or
`aiplus refresh` to avoid ambiguity. The agent should report current Auto
Compact, Auto Team Consultant, and compact-state status before unrelated project
refresh when you ask for AiPlus.

For Claude Code:

```bash
aiplus install claude-code
```

For OpenCode:

```bash
aiplus install opencode
```

The v0.2.1 one-command installer is verified for macOS Apple Silicon first. Other
platforms should use [Developer Build](#developer-build) until their release
assets are published and verified.

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
aiplus refresh
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

## Compact And Resume

You do not need to remember compact commands.

In your agent session, say:

```text
prepare compact
```

or:

```text
save progress
```

The agent will use AiPlus backend tools to validate readiness and prepare a
checkpoint. If it is ready, the agent should answer in plain language:

```text
Ready to compact.

After compact:
- If I continue automatically, you do not need to do anything.
- If I do not reply, send: continue

I will resume from here.
```

After compact, say:

```text
continue
```

AiPlus resumes best-effort:

- If the agent continues automatically, you do not need to do anything.
- If the agent does not reply, send `continue`.

AiPlus cannot force host compact, click UI compact, call `/compact` for you, or
wake the agent if the host requires user input.

Advanced users and maintainers can run the backend commands directly:

```bash
aiplus compact prepare
aiplus compact score
aiplus compact checkpoint --level standard
aiplus compact resume
```

If `aiplus` is not found, install AiPlus or fix PATH instead of falling back to
Node:

```bash
curl -fsSL https://raw.githubusercontent.com/izhiwen/aiplus/main/install.sh | bash
```

Then reopen the terminal or ensure `~/.local/bin` is on PATH.

## Installer Safety

`install.sh` downloads a GitHub Release asset, verifies `checksums.txt`, and
installs only the `aiplus` command to `~/.local/bin/aiplus` by default. It does
not use `sudo`, silently edit shell profiles, install project modules, upload
data, add telemetry, or change global Codex, Claude Code, or OpenCode
configuration. AiPlus v0.2.1 publishes the verified macOS Apple Silicon asset
first; additional platform assets remain planned.

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

The old docs used `<AIPLUS_SOURCE>` to mean "the folder where you cloned the
AiPlus repo." Do not type angle-bracket placeholders literally.

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

The legacy Node CLI is archived/reference-only at v0.2.1 and is not included in
this public source package. It is retained in the private/local AiPlus workspace
for behavior audits and emergency reference fixes. New CLI work should target
Rust.

Compact commands are Rust-native. Rust runtime assets no longer install or check
`compactctl.mjs`.

## Safety Boundary

The AiPlus CLI does not implement publish, push, tag, release creation,
system/global install, global config edits, telemetry, auto-update, or runtime
network fetches. The v0.2.1 installer writes only the user-level
`~/.local/bin/aiplus` command.

Validation is structural and heuristic. It is not a safety, privacy,
compliance, correctness, or release certification.
