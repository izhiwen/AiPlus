# Installer Plan

Status: `ACTIVE_FOR_V0_1_0`

This document describes the v0.1.1 installer for the `aiplus` command.

## Current User Flow

The beginner flow is:

```bash
curl -fsSL https://raw.githubusercontent.com/izhiwen/aiplus/main/install.sh | bash
cd MyProject
aiplus install codex
```

The installer downloads a GitHub Release asset and verifies `checksums.txt`
before writing the binary.

## Installer Responsibilities

`install.sh`:

- detect OS and CPU architecture
- install v0.1.1 only on platforms with a verified published asset
- download the matching GitHub Release archive
- verify SHA-256 checksums before installing
- install only the `aiplus` command under `~/.local/bin/aiplus` by default
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

## v0.1.1 Release Artifact Requirements

The v0.1.1 installer is activated for macOS Apple Silicon after these checks:

- GitHub Release tag is Owner-approved
- release archive is built for verified macOS Apple Silicon
- checksums are generated and reviewed
- archive contents include `aiplus`, `README.md`, and `LICENSE`
- target-platform smoke tests are completed or clearly marked untested
- installer script is reviewed for shell safety
- README states exactly what the installer writes

Other platforms must use Developer Build instructions until their assets are
published and verified.

## Owner Approval Status

Owner approved these v0.1.1 actions:

- creating the v0.1.1 tag and GitHub Release
- uploading the verified macOS Apple Silicon binary
- uploading `checksums.txt`
- publishing `install.sh`
- installing the `aiplus` command under `~/.local/bin/aiplus`

## Still Owner-Gated

Separate Owner approval is still required before:

- installing into `/usr/local/bin`, `~/.cargo/bin`, or any system/global path
- modifying shell profiles or global configs
- publishing package registry, Homebrew, npm wrapper, or marketplace channels

## Dry Run

Preview without installing:

```bash
sh install.sh --dry-run
```
