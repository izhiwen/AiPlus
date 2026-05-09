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

The v0.4.6 one-command installer is verified for macOS Apple Silicon first. Other
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
aiplus update all
aiplus self update --dry-run
aiplus compact savings
aiplus pricing status
aiplus profile status
aiplus secret-broker status
aiplus uninstall --dry-run
```

## Private User Profile And Secret Broker

AiPlus can also install a user-level private profile and resolve approved
runtime secrets without putting private content into public repos.

```bash
aiplus profile install <private-profile-name> --user --source /path/to/private-profile --dry-run
aiplus profile install <private-profile-name> --user --source /path/to/private-profile --yes
aiplus profile status
aiplus profile cleanup --user --dry-run
aiplus profile cleanup --user --yes
aiplus profile migrate <legacy-profile> <canonical-profile> --user --yes
aiplus profile disable <private-profile-name> --user --yes
aiplus profile uninstall <private-profile-name> --user --yes
aiplus secret-broker status
```

Private profiles live under `~/.config/aiplus/profiles/<private-profile-name>/`.
They store working preferences and collaboration rules only. They must not
contain API keys, Bitwarden tokens, passwords, prompt transcripts, project files,
or compact checkpoints.

`aiplus profile status` lists active canonical profiles in `profiles=[...]`.
Legacy compatibility profiles may appear separately in `legacy_profiles=[...]`;
run `aiplus profile cleanup --user --yes` after the canonical profile is
installed to back up and remove legacy active registrations.

Secret access goes through `aiplus secret-broker`. By default,
`aiplus secret-broker resolve <alias>` verifies access without printing the
secret value. `aiplus secret-broker list` shows aliases installed by the private
profile package. Public AiPlus does not bundle private alias namespaces.
For Bitwarden, AiPlus resolves the alias key/name to the Bitwarden secret ID in
memory, then fetches the value through `bws`; it prints only metadata such as
`secret_id_found=yes`, never the secret ID or secret value by default.

Real Bitwarden smoke checks require the `bws` CLI and a read-only machine
account token available through `BWS_ACCESS_TOKEN` or the macOS Keychain. For
tools that need a key, use:

```bash
aiplus secret-broker run -- <command...>
```

The child command receives approved secrets in its environment. AiPlus will not
print or persist those values, but the child command itself could still print,
log, transmit, or store them. Use `run --` only with commands you trust for the
specific action.

AiPlus may read `BWS_ACCESS_TOKEN` for the current process or a macOS Keychain
entry created by `aiplus secret-broker token set`. It does not store Bitwarden
machine tokens in repo files, `.aiplus/`, `.codex/compact/`, shell profiles,
logs, docs, compact savings ledgers, or release artifacts.

Natural language mappings can be supplied by private profiles. Secret status
requests should map to short metadata-only checks and must never expose secret
values.

## Updating AiPlus

In an agent session, you can say:

```text
update AiPlus
```

The agent should report scope first:

```text
I will update the aiplus CLI and this project's AiPlus modules. I will not edit
global agent config or upload project data.
```

Then it should run:

```bash
aiplus update all
```

Specific commands:

```bash
aiplus self update --dry-run  # check the global/user-level CLI update
aiplus self update --yes      # update the user-level aiplus command
aiplus update                 # update only this project's .aiplus/ modules
aiplus update all             # update CLI, then this project, then advise doctor
```

Chinese update triggers such as `升级 AiPlus`, `把 AiPlus 全部更新到最新版`,
`只更新这个项目的 AiPlus`, and `更新 aiplus 命令` are supported in installed
agent guidance.

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
aiplus compact savings
```

If `aiplus` is not found, install AiPlus or fix PATH instead of falling back to
Node:

```bash
curl -fsSL https://raw.githubusercontent.com/izhiwen/aiplus/main/install.sh | bash
```

Then reopen the terminal or ensure `~/.local/bin` is on PATH.

## Compact Savings Estimate

AiPlus estimates compact savings from local aggregate compact metadata. It does
not require pricing setup, model setup, provider account connection, billing API
access, or manual model price input.

Ask in the agent session:

```text
show compact savings
```

or run:

```bash
aiplus compact savings
```

The short report includes this compact and all-time totals:

```text
Compact savings estimate

This compact:
- Tokens saved: ~18k
- Token reduction: ~41%
- Estimated cost saved: ~$0.05
- Recovery confidence: HIGH

All time:
- Tokens saved: ~184k
- Average reduction: ~38%
- Estimated cost saved: ~$0.46
- Pricing coverage: 8/10 compacts

Estimate only, not billing data.
```

All-time reduction is weighted:
`totalEstimatedTokensSaved / totalEstimatedBaselineTokens * 100`. It is not a
simple average of per-compact percentages.

AiPlus stores aggregate savings events in
`.codex/compact/savings-ledger.jsonl`. The ledger must not store prompts,
transcripts, project file contents, raw checkpoint text, billing data, or usage
history. If pricing for a detected model is unavailable, AiPlus still reports
token savings and reduction percentage; USD savings are shown as unavailable or
partial.

Savings event semantics:

- `prepare`: projected readiness estimate; does not count toward completed
  all-time savings.
- `checkpoint`: candidate estimate; does not count toward completed all-time
  savings by itself.
- `resume`: completed compact cycle; counts once per `checkpointId`.

Re-running `resume` for the same checkpoint does not double-count all-time
totals.

Pricing cache policy:

```bash
aiplus pricing status
aiplus pricing update
```

AiPlus uses fresh cached pricing when available. If the cache is missing or
stale, AiPlus may refresh public pricing automatically; network failure never
blocks compact, checkpoint, resume, or token savings reporting. `aiplus pricing
update` explicitly refreshes public pricing data and writes the cache to the
user cache directory, normally `~/.cache/aiplus/pricing-cache.json`. The default
cache TTL is 7 days.

## Installer Safety

`install.sh` downloads a GitHub Release asset, verifies `checksums.txt`, and
installs only the `aiplus` command to `~/.local/bin/aiplus` by default. It does
not use `sudo`, silently edit shell profiles, install project modules, upload
data, add telemetry, or change global Codex, Claude Code, or OpenCode
configuration. AiPlus v0.4.6 publishes the verified macOS Apple Silicon asset
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

The legacy Node CLI is archived/reference-only and is not included in
this public source package. It is retained in the private/local AiPlus workspace
for behavior audits and emergency reference fixes. New CLI work should target
Rust.

Compact commands are Rust-native. Rust runtime assets no longer install or check
`compactctl.mjs`.

## Safety Boundary

The AiPlus CLI does not implement package publish, system/global install, global
config edits, telemetry, auto-update callbacks, provider account access, or user
data upload. `aiplus pricing update` may fetch public release/pricing metadata
and cache it locally. It does not upload prompts, project files, checkpoints,
savings ledgers, secrets, billing data, or usage history.

Validation is structural and heuristic. It is not a safety, privacy,
compliance, correctness, or release certification.
