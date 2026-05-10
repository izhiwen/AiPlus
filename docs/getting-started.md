# Getting Started with AiPlus

3-minute success path. Five commands. One agent refresh.

## Prerequisites

- macOS Apple Silicon (verified). Other platforms: use [Developer Build](#developer-build).
- An AI coding agent: Codex, Claude Code, or OpenCode.

## Step 1: Install the CLI

```bash
curl -fsSL https://raw.githubusercontent.com/izhiwen/aiplus/main/install.sh | bash
```

This installs `aiplus` to `~/.local/bin/aiplus`. Reopen your terminal or ensure `~/.local/bin` is on PATH.

Verify:

```bash
aiplus --version
```

## Step 2: Install into your project

```bash
cd MyProject
aiplus install codex
```

Replace `codex` with `claude-code` or `opencode` as needed. For all runtimes:

```bash
aiplus install all
```

Existing installs are safely upgraded. Backups go to `.aiplus/backups/`.

## Step 3: Refresh the agent

In your already-open agent session for that project, type:

```text
AiPlus refresh
```

The agent reads the installed AiPlus guidance and reports status.

## Step 4: Check health

```bash
aiplus doctor
```

This validates installation, manifest, memory, compact state, and adapter files. All checks should show `PASS`.

## Step 5: Save progress before compact

When you want to save progress in the agent session:

```text
save progress
```

After the host compact, if the agent does not reply:

```text
continue
```

## Developer Build

If no release asset exists for your platform:

```bash
git clone https://github.com/izhiwen/aiplus.git
cd aiplus
cargo build --release
```

Then from a target project:

```bash
~/aiplus/target/release/aiplus install codex
```

## Next Steps

- [Daily Workflows](daily-workflows.md) — natural-language command map
- [Memory Guide](memory-guide.md) — project memory, search, forget
- [Compact Guide](compact-guide.md) — remind, prepare, checkpoint, resume
- [Troubleshooting](troubleshooting.md) — doctor, common fixes
