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
aiplus install codex
```

If the project already has an older AiPlus install, the same command safely
upgrades AiPlus managed files, creates backups under `.aiplus/backups/`, and
preserves `.codex/compact/` state.

Then type this in the already-open Codex, Claude Code, or OpenCode session for
that same project:

```text
刷新
```

English also works:

```text
refresh
```

The agent should treat `刷新` / `refresh` as AiPlus refresh first and reply with
current Auto Compact, Auto Team Consultant, and compact-state status before
continuing work.

For Claude Code:

```bash
aiplus install claude-code
```

For OpenCode:

```bash
aiplus install opencode
```

The v0.1.1 one-command installer is verified for macOS Apple Silicon first. Other
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

Before compact-worthy moments, ask the agent to prepare state:

```bash
aiplus compact validate
aiplus compact checkpoint
```

The agent should then suggest compact in plain language:

```text
建议现在 compact。AiPlus checkpoint 已准备好。compact 后如果宿主继续把控制权交给我，我会自动恢复；如果工具等待你发消息，随便说“继续”“刷新”“continue”“resume”或类似意思即可。
```

After host compact, AiPlus resumes best-effort:

- If the host gives control back to the agent, the agent should run
  `aiplus compact resume` automatically.
- If the host waits for a user message, say anything like `继续`, `刷新`,
  `continue`, `resume`, `refresh`, `go on`, or `接着`.

AiPlus cannot force host compact, click UI compact, call `/compact` for you, or
wake the agent if the host requires user input.

## Installer Safety

`install.sh` downloads a GitHub Release asset, verifies `checksums.txt`, and
installs only the `aiplus` command to `~/.local/bin/aiplus` by default. It does
not use `sudo`, silently edit shell profiles, install project modules, upload
data, add telemetry, or change global Codex, Claude Code, or OpenCode
configuration. AiPlus v0.1.1 publishes the verified macOS Apple Silicon asset
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

The legacy Node CLI is archived/reference-only at v0.1.3 and is not included in
this public source package. It is retained in the private/local AiPlus workspace
for behavior audits and emergency reference fixes. New CLI work should target
Rust.

Compact commands are Rust-native. Rust runtime assets no longer install or check
`compactctl.mjs`.

## Safety Boundary

The AiPlus CLI does not implement publish, push, tag, release creation,
system/global install, global config edits, telemetry, auto-update, or runtime
network fetches. The v0.1.1 installer writes only the user-level
`~/.local/bin/aiplus` command.

Validation is structural and heuristic. It is not a safety, privacy,
compliance, correctness, or release certification.
