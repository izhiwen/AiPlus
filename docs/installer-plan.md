# Installer Plan

Status: `OWNER_GATED_PLAN_ONLY`

This document plans a future installer for the `aiplus` command. It does not
publish an installer, create a GitHub Release, upload binaries, or globally
install anything.

## Intended Future User Flow

After Owner approval and release artifact publication, the intended beginner
flow is:

```bash
curl -fsSL https://raw.githubusercontent.com/izhiwen/aiplus/main/install.sh | bash
cd MyProject
aiplus install codex
```

The installer command is not the current recommended path until release binaries
and checksums exist.

## Installer Responsibilities

A future `install.sh` should:

- detect OS and CPU architecture
- download the matching GitHub Release archive
- verify SHA-256 checksums before installing
- install only the `aiplus` command, likely under `~/.local/bin/aiplus`
- print clear next steps if `~/.local/bin` is not on `PATH`
- avoid silently editing shell profiles
- avoid installing project modules automatically
- avoid touching global Codex, Claude Code, or OpenCode config
- avoid telemetry, analytics, callbacks, and remote config

Project setup remains explicit:

```bash
cd MyProject
aiplus install codex
```

## Release Artifact Requirements

Before the installer is activated:

- GitHub Release tag is Owner-approved
- release archives are built for supported platforms
- checksums are generated and reviewed
- archive contents include `aiplus`, `README.md`, and `LICENSE`
- target-platform smoke tests are completed or clearly marked untested
- installer script is reviewed for shell safety
- README states exactly what the installer writes

## Owner Gates

Separate Owner approval is required before:

- creating or pushing a git tag
- creating a GitHub Release
- uploading binary artifacts
- publishing or activating `install.sh` as the primary install path
- installing into `~/.local/bin`, `/usr/local/bin`, `~/.cargo/bin`, or any global
  path
- modifying shell profiles or global configs
- publishing package registry, Homebrew, npm wrapper, or marketplace channels

## Current Safe Path

Until those gates are approved, use the README source-build quick start from the
target project:

```bash
AIPLUS_HOME="$HOME/aiplus"; test -d "$AIPLUS_HOME" || git clone https://github.com/izhiwen/aiplus.git "$AIPLUS_HOME"; (cd "$AIPLUS_HOME" && cargo build --release); "$AIPLUS_HOME/target/release/aiplus" install codex
```
