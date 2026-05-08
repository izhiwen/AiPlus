# Distribution Plan

Status: `V0_1_0_RELEASE_SCOPE`

This document describes the v0.1.1 binary and installer distribution scope.

## v0.1: Source Candidate

Audience: Owner review and technical testers.

Current one-command installer:

```bash
curl -fsSL https://raw.githubusercontent.com/izhiwen/aiplus/main/install.sh | bash
```

If the binary is already on PATH in a local test environment:

```bash
cd MyProject
aiplus install codex
```

Activation for already-open sessions:

```text
刷新
```

English:

```text
refresh
```

v0.1.1 release scope:

- verified macOS Apple Silicon binary
- `checksums.txt`
- `install.sh`
- installer support for verified macOS Apple Silicon only
- source-build path retained under Advanced / Developer docs

## Later GitHub Releases Binary Plan

Planned channel: GitHub Releases with prebuilt archives and checksums.

Artifacts are listed in [binary-artifact-matrix.md](binary-artifact-matrix.md).
The installer design is listed in [installer-plan.md](installer-plan.md).

Required before upload:

- clean source tree
- reproducible build commands documented
- checksums generated locally
- artifacts tested on matching OS/arch where practical
- release notes reviewed
- Owner approves tag and GitHub Release

## Later Channels

These are optional future channels and are not mainline until separately
approved:

- Homebrew tap/formula
- `cargo install` or crates.io package, if license and packaging strategy are
  approved
- additional installer-supported platforms after their assets are verified
- npm wrapper as a compatibility bridge only, not the mainline implementation

## Update Strategy

Current CLI update commands update installed bundled modules from the local
binary's embedded assets:

```bash
aiplus update
aiplus update auto-compact
aiplus update auto-team-consultant
aiplus add auto-team-consultant
```

No remote auto-update is implemented. Future remote update checks would require
Owner approval and explicit user-facing documentation.

## Safety Boundaries

Distribution must preserve these boundaries:

- no telemetry
- no runtime network fetches
- no global config edits
- no shell profile edits
- no system/global install; v0.1.1 only writes the user-level
  `~/.local/bin/aiplus` path
- no claim of safety/compliance/privacy certification

## License

The Rust mainline/public-ready package is Apache-2.0. Bundled child module
snapshots preserve their existing licenses:

- `aiplus-auto-compact`: Apache-2.0
- `aiplus-auto-team-consultant`: MIT

Licensing is not a safety, privacy, compliance, correctness, or release
certification.
