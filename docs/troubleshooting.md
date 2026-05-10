# Troubleshooting

Common issues and how to diagnose them.

## First: Run Doctor

```bash
aiplus doctor
aiplus memory doctor
aiplus profile doctor
aiplus secret-broker status
```

`aiplus doctor` checks installation, manifest, adapters, and compact state. `memory doctor` checks memory files and records. `profile doctor` checks profile files and identity TOML validity.

## Common Issues

### `aiplus: command not found`

The CLI is not on PATH.

```bash
# Check if installed
ls ~/.local/bin/aiplus

# If missing, reinstall
curl -fsSL https://raw.githubusercontent.com/izhiwen/aiplus/main/install.sh | bash

# If present but not on PATH
export PATH="$HOME/.local/bin:$PATH"
```

### OpenCode config has invalid JSON

```bash
aiplus doctor
```

If you see `NEEDS_FIX opencode config JSON parse failed`, the `.opencode/config.json` file has invalid JSON. Fix the JSON syntax and re-run doctor. AiPlus also rejects config files with a top-level `aiplus` key in the OpenCode config.

### Installed binary is older than source

If you built from source but the installed binary is stale:

```bash
# Check installed version
aiplus --version

# Rebuild and reinstall
cd aiplus-public
cargo build --release
cp target/release/aiplus ~/.local/bin/aiplus
```

Or use the self-update mechanism:

```bash
aiplus self update --dry-run
aiplus self update --yes
```

### Compact is not auto-triggering

AiPlus cannot trigger host compact. It can only remind and prepare. The compact must be triggered by you or the host agent.

Check readiness:

```bash
aiplus compact remind
```

If `REMINDER_DECISION=wait`, the handoff is not current enough. If `REMINDER_DECISION=blocked`, there is a safety gate preventing compact.

### Memory write blocked by redaction

If `memory add` returns `MEMORY_REDACTION_STATUS=BLOCKED`, the text contains a detected sensitive pattern. Remove secrets, API keys, private keys, JWT tokens, phone numbers, or transcript-like content and retry.

### Profile not found

```bash
aiplus profile status
```

If the profile does not appear in `profiles=[...]`, it is not installed. Install it:

```bash
aiplus profile install my-profile --user --source /path/to/source --yes
```

### Legacy profiles showing up

```bash
aiplus profile status
```

If `legacy_profiles=[...]` is non-empty, clean up after installing the canonical profile:

```bash
aiplus profile cleanup --user --dry-run
aiplus profile cleanup --user --yes
```

### Secret broker returns "not configured"

```bash
aiplus secret-broker status
aiplus secret-broker list
```

Secrets require a private profile with `secret-aliases.tsv` and either `BWS_ACCESS_TOKEN` or a macOS Keychain entry. Values that are empty, whitespace-only, or `PENDING_OWNER_INPUT_DO_NOT_USE` are treated as not configured.

### `compact watch` leaves a running process

`compact watch --interval` handles SIGINT (Ctrl+C) and SIGTERM. If a process is orphaned:

```bash
ps aux | grep 'aiplus.*watch'
# If found, kill it
kill <pid>
```

### Dangling symlinks in .aiplus/

```bash
aiplus doctor
```

If doctor reports `NEEDS_FIX dangling symlink`, remove the broken symlink:

```bash
ls -la .aiplus/
rm .aiplus/<broken-link>
aiplus install codex
```

## Getting More Help

- [FAQ](faq.md)
- [Memory Guide](memory-guide.md) — memory redaction, types, search
- [Compact Guide](compact-guide.md) — remind, prepare, checkpoint, resume
- [Glossary](glossary.md) — term definitions
