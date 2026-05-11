# AiPlus Module Contract Plan

Status: draft foundation for the Rust module system.

## Goal

AiPlus is the platform/toolbox. `aiplus-core` is the stable internal module
foundation, and `aiplus-cli` is the command wrapper for argument parsing,
dispatch, output formatting, and exit-code mapping.

## ABI Meaning

For AiPlus, ABI means the stable AiPlus external interface:

- CLI behavior and command shapes
- stdout/stderr markers
- exit codes
- file schemas and manifest format
- bundled module snapshot layout
- public Rust crate API

It does not mean a C ABI unless a future implementation explicitly adds one.

## CLI Behavior Contract

- `aiplus install <runtime>` installs project-local assets only.
- `aiplus update [module]` updates project-local `.aiplus/` modules only.
- `aiplus add <module>` adds a bundled module to an existing AiPlus install.
- `aiplus doctor` validates manifest, runtime adapter files, and module
  required files.
- `aiplus rollback --dry-run` reports rollback scope without modifying files.
- `aiplus rollback --id latest --yes` may restore only AiPlus-managed files from
  a rollback record.
- Runtime adapters are explicit contracts for Codex, Claude Code, and OpenCode.

## stdout Marker Contract

Existing stable markers remain command-owned output:

- install/update/add/uninstall: `INSTALL_STATUS=PASS`, `UPGRADE_STATUS=PASS`,
  `UPDATE_STATUS=PASS`, `ADD_STATUS=PASS`, `UNINSTALL_STATUS=PASS`
- doctor/status/refresh: `DOCTOR_STATUS=PASS`, `STATUS=PASS`,
  `AIPLUS_REFRESH_STATUS=PASS`
- compact: `COMPACT_RUST_NATIVE_STATUS=PASS`
- memory/identity/skill-candidate/secret-broker: existing command-specific
  markers remain unchanged.
- rollback foundation: `ROLLBACK_STATUS=DRY_RUN` or `ROLLBACK_STATUS=PASS`.

CLI owns marker wording. Core returns typed status, plans, and diagnostics.

## Exit-Code Contract

- `0`: command completed successfully or dry-run completed successfully.
- `1`: command failed or refused a risky operation.
- `2`: usage error, unknown subcommand, validation review-needed, or malformed
  arguments where existing behavior already uses usage exit.

CLI maps core errors to these exit codes.

## Manifest / Schema Contract

Project manifest:

- Stored at `.aiplus/manifest.json`.
- Uses `schemaVersion`, `installer`, `installerVersion`, `runtimeAdapters`,
  `modules`, and `managedFiles`.
- Unknown, malformed, or future schema versions block automatic migration unless
  a command explicitly implements a reviewed migration path.

Module manifest:

- Stored as `aiplus-module.json` at each bundled module root.
- Validated by `aiplus-core` against
  `crates/aiplus-core/schemas/aiplus-module.schema.json`.
- Existing Rust constants may remain as a static registry, but core validates
  those constants against module manifests.

## Module Metadata Contract

Each bundled module declares:

- `schemaVersion`
- `name`
- `displayName`
- `version`
- `source`
- `license`
- `requiredFiles`
- `managedFiles`
- `runtimeAdapters`
- `installHints`
- `doctorChecks`
- `publicPrivateBoundary`
- `secretBoundary`
- `legacyStatus`

The first dogfood modules are `compact-reminder`, `auto-team-consultant`, and
`agent-memory`.

## Install / Update / Write Boundary

- Core owns registry, module manifest parsing, embedded asset access, required
  file validation, safe path helpers, and rollback plan data structures.
- CLI owns user-facing command output and calls existing safe write functions.
- Writes are limited to project-local `.aiplus/`, `.codex/compact/`, runtime
  adapter files, and `AGENTS.md` managed blocks.
- Core must not write global configs, shell profiles, global agent configs, or
  system paths.

## Rollback Contract

- Any overwrite of an AiPlus-managed file creates a backup and rollback record
  first.
- Rollback records live under `.aiplus/backups/<id>/rollback-plan.json`.
- `rollback --dry-run` never modifies files.
- Rollback restores only files listed in the rollback plan.
- Unknown files are reported and skipped.
- Rollback never deletes non-AiPlus-created user content.

## Public / Private / Secret Boundary

- Public release assets must not include `aiplus-work-with-zhiwen` private
  profile content.
- Public docs may describe private profile mechanics without revealing private
  profile material.
- Secret status/list/doctor commands remain metadata-only.
- The CLI must not print or persist secret values, Bitwarden tokens, API keys,
  provider payloads, raw transcripts, `.env` contents, or private profile
  content.

## Release Artifact Boundary

Local release-candidate preparation is allowed. Uploads, tags, GitHub Releases,
package registry publication, and remote publishing require explicit Owner
approval.
