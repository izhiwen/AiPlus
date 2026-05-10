# Profile Bundle Guide

AiPlus private profiles can include a **supplemental bundle** of additional files beyond the core `profile.toml` and `AGENTS.profile.md`. This guide explains what each file does and how to install and inspect them.

## What Is a Profile Bundle?

A profile bundle is a directory containing a private profile source with optional supplemental files. When you run `aiplus profile install`, AiPlus copies the bundle to `~/.config/aiplus/profiles/<name>/`.

## Bundle Contents

### Core Files (always required)

| File | Purpose |
|---|---|
| `profile.toml` | Profile metadata: name, version, owner |
| `AGENTS.profile.md` | Agent guidance for this profile |

### Supplemental Files (optional)

| File/Dir | Purpose | Contains Secrets? |
|---|---|---|
| `USER.md` | Owner snapshot and stable preferences | No. Redacted on read. |
| `MEMORY.md` | Profile-level memory snapshot | No. Redacted on read. |
| `preferences/` | Preference taxonomy files | No. |
| `identities/` | Role identity definitions (`.identity.toml` files) | No. |
| `sync/` | Sync policy and project mapping files | No. |

**Important**: `sync/` contains local sync policy files that define how profile preferences map to projects. It is not cloud sync, not network sync, and does not connect to external services.

### Secret Aliases (separate location)

If the source contains `secret-aliases.tsv`, it is installed to `~/.config/aiplus/secret-broker/profiles/<name>/` — never to the profile directory itself.

## Commands

### Install a profile

```bash
aiplus profile install my-profile --user --source /path/to/profile-source --dry-run
aiplus profile install my-profile --user --source /path/to/profile-source --yes
```

Install always requires `--user` (installs to user config, not project-local). `--dry-run` shows what would be installed without writing.

Install creates a backup of the existing profile directory before overwriting.

### Update a profile

```bash
aiplus profile update my-profile --user --source /path/to/profile-source
```

Same as install but without the `--yes` confirmation gate.

### Check status

```bash
aiplus profile status
aiplus profile status my-profile
```

Shows installed profiles, core file presence, and supplemental bundle status.

### Inspect context

```bash
aiplus profile context my-profile
```

Shows profile metadata and supplemental bundle file counts. Does not print file contents.

```bash
aiplus user context --profile my-profile
```

Shows the `USER.md` content with sensitive lines redacted. Output is truncated if larger than 8 KB.

### Diagnose issues

```bash
aiplus profile doctor
aiplus profile doctor my-profile
```

Checks:

- Core files exist
- Supplemental files present
- Identity TOML files are parseable and contain `name =` and `role =` fields
- Reports `secret_values=none`

### Clean up legacy profiles

```bash
aiplus profile cleanup --user --dry-run
aiplus profile cleanup --user --yes
```

Backs up and removes legacy profile registrations.

### Disable or uninstall

```bash
aiplus profile disable my-profile --user --yes
aiplus profile uninstall my-profile --user --yes
```

Both create backups before removal. Uninstall also removes secret-broker aliases.

## Safety Boundaries

- Private profile content is never copied into public release assets.
- `aiplus user context` redacts lines containing `api_key`, `secret_key`, `password`, `bearer`, `authorization:`, `private_key`, `cookie:`, or `-----begin`.
- Profile writes go to `~/.config/aiplus/profiles/` only — no global agent config edits.
- `sync/` files are local policy definitions, not network synchronization.
