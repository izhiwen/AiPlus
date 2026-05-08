# AiPlus Rust Release Checklist

Status: `V0_1_0_RELEASE_APPROVED_SCOPE`

This checklist tracks the Owner-approved v0.2.1 GitHub Release scope. It does
not approve package registries, Homebrew, marketplace publication, telemetry, or
system/global install paths.

## Owner Gates

- [x] Owner approves public repo name and structure.
- [x] Owner selected Apache-2.0 for the Rust mainline/public-ready package.
- [ ] Owner approves any future license change away from Apache-2.0 or public
      legal wording change.
- [x] Owner approves creating or using `github.com/izhiwen/aiplus`.
- [x] Owner approves git push for reviewed v0.2.1 source/docs changes.
- [x] Owner approves creating the `v0.2.1` git tag.
- [x] Owner approves the `v0.2.1` GitHub Release.
- [x] Owner approves uploading the verified macOS Apple Silicon binary and
      `checksums.txt`.
- [ ] Owner approves any package registry publication.
- [x] Owner approves publishing `install.sh` for user-level
      `~/.local/bin/aiplus` installs.
- [ ] Owner approves any Homebrew, npm wrapper, marketplace, system/global
      install, or package registry channel.

## Local QA Gates

- [ ] `cargo fmt --check`
- [ ] `cargo test`
- [ ] `cargo run -p aiplus-cli -- --help`
- [ ] `cargo clippy --all-targets --all-features -- -D warnings`
- [ ] `cargo metadata --format-version 1 --no-deps`
- [ ] `sh install.sh --dry-run`
- [ ] Fake-HOME installer smoke verifies checksum and runs installed binary.
- [ ] Manual smoke: `install codex`
- [ ] Manual smoke: `install claude-code`
- [ ] Manual smoke: `install opencode`
- [ ] Manual smoke: `install all`
- [ ] Manual smoke: `status`
- [ ] Manual smoke: `doctor`
- [ ] Manual smoke: `update`
- [ ] Manual smoke: `add auto-team-consultant --dry-run`
- [ ] Manual smoke: `compact init`
- [ ] Manual smoke: `compact validate`
- [ ] Manual smoke: `compact prepare`
- [ ] Manual smoke: `compact score`
- [ ] Manual smoke: `compact checkpoint`
- [ ] Manual smoke: `compact checkpoint --level light|standard|full`
- [ ] Manual smoke: `compact resume`
- [ ] Manual smoke: `uninstall --dry-run`

## Static Safety Gates

- [ ] No `compactctl.mjs` in Rust install footprint.
- [ ] No active `Command::new("node")`.
- [ ] Cargo metadata reports `Apache-2.0`.
- [ ] `LICENSE` exists in Rust package root.
- [ ] Child module licenses are preserved and documented.
- [ ] No target/build artifacts in release docs or package staging area.
- [ ] No `.DS_Store`.
- [ ] No private data, secrets, raw logs, screenshots, or media artifacts.
- [ ] No telemetry or runtime network callbacks.
- [ ] No global config writes.
- [ ] No publication actions outside the Owner-approved v0.2.1 GitHub Release
      scope executed.
- [ ] No overclaim wording such as certified, compliant, official, endorsed,
      guaranteed safe, or production-ready.

## Distribution Docs Gates

- [ ] `docs/public-repo-plan.md`
- [ ] `docs/distribution-plan.md`
- [ ] `docs/binary-artifact-matrix.md`
- [ ] `docs/migration-from-node-cli.md`
- [ ] `docs/qa-release-readiness.md`
- [ ] `docs/safety.md`
- [ ] Beginner README path does not recommend Node CLI.
- [ ] Node CLI is clearly archived/reference-only.
- [ ] Public release docs state what has and has not been released.
- [ ] Owner gates are explicit.
- [ ] Beginner README commands avoid undefined placeholders.
- [ ] Installer behavior is documented with no silent shell profile edits.

## Binary Artifact Gates

- [ ] Target triples reviewed.
- [ ] Build commands reviewed.
- [ ] Checksum plan reviewed.
- [ ] Signature/notarization status documented.
- [ ] Tested status documented per platform.
- [ ] Archive contents reviewed.
- [ ] Archive contains Apache-2.0 `LICENSE`.
- [ ] `checksums.txt` matches uploaded artifacts.
- [ ] `install.sh` downloads only release assets and installs only `aiplus`.
- [x] Owner approved the v0.2.1 upload scope before release artifact creation.
