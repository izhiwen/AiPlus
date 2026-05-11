# Migration From Node CLI

Status: `RUST_MAINLINE`

The Rust `aiplus` binary is the mainline CLI. The legacy Node CLI is
archived/reference-only at v0.2.1 and is not included in this public source
package.

## What Changes

Old archived reference invocation:

```bash
node <PRIVATE_AIPLUS_WORKSPACE>/legacy/aiplus-cli/bin/aiplus.mjs install codex
```

Rust mainline invocation:

```bash
aiplus install codex
```

For local source testing:

```bash
cd "$HOME/aiplus"
cargo run -p aiplus-cli -- install codex
```

## Preserved UX

```bash
aiplus install codex
aiplus install claude-code
aiplus install opencode
aiplus install all
aiplus status
aiplus doctor
aiplus update
aiplus add auto-team-consultant
aiplus uninstall --dry-run
```

Already-open session activation remains:

```text
刷新
```

English:

```text
refresh
```

## Compact Migration

Compact behavior is Rust-native. Ordinary users can ask the agent in natural
language, for example "prepare compact", "save progress", or "continue". The
agent uses these backend commands when needed:

```bash
aiplus compact prepare
aiplus compact score
aiplus compact init
aiplus compact validate
aiplus compact checkpoint
aiplus compact resume
```

Rust installs do not include or require `compactctl.mjs`.

## Existing Projects

Run:

```bash
aiplus status
aiplus doctor
aiplus update
```

If a module is missing:

```bash
aiplus add compact-reminder
aiplus add auto-team-consultant
```

If you only want to inspect removals:

```bash
aiplus uninstall --dry-run
```

## Node Reference Policy

Keep the archived Node CLI only in the private/local AiPlus workspace for
behavior audits and emergency reference fixes. Do not add new features there
unless Owner explicitly approves a reference fix.

Do not delete the Node tree or public module histories without Owner approval.
