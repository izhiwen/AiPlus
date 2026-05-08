# AiPlus Rust Release Checklist

Status: `OWNER_REVIEW_READY_CANDIDATE`

This checklist prepares for Owner review. It does not approve publication.

## Owner Gates

- [ ] Owner approves public repo name and structure.
- [x] Owner selected Apache-2.0 for the Rust mainline/public-ready package.
- [ ] Owner approves any future license change away from Apache-2.0 or public
      legal wording change.
- [ ] Owner approves creating or using a public repo.
- [ ] Owner approves any git push.
- [ ] Owner approves any git tag.
- [ ] Owner approves any GitHub Release.
- [ ] Owner approves any binary artifact upload.
- [ ] Owner approves any package registry publication.
- [ ] Owner approves any Homebrew, shell installer, npm wrapper, or global install
      channel.

## Local QA Gates

- [ ] `cargo fmt --check`
- [ ] `cargo test`
- [ ] `cargo run -p aiplus-cli -- --help`
- [ ] `cargo clippy --all-targets --all-features -- -D warnings`
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
- [ ] Manual smoke: `compact checkpoint`
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
- [ ] No publication actions executed.
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
- [ ] Public release docs state no release has happened.
- [ ] Owner gates are explicit.
- [ ] Beginner README commands avoid undefined placeholders.
- [ ] Future installer path is documented as Owner-gated until release artifacts
      and checksums exist.

## Binary Artifact Gates

- [ ] Target triples reviewed.
- [ ] Build commands reviewed.
- [ ] Checksum plan reviewed.
- [ ] Signature/notarization status documented.
- [ ] Tested status documented per platform.
- [ ] Archive contents reviewed.
- [ ] Archive contains Apache-2.0 `LICENSE`.
- [ ] Owner approves upload before any release artifact is created publicly.
