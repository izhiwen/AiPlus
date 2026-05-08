# Changelog

## 0.1.3-rust-mainline

- Rewrote public README and README.zh-CN beginner flow to use copy-pasteable
  source-build commands instead of undefined `<AIPLUS_SOURCE>` placeholders.
- Standardized human-facing product naming to `AiPlus` while keeping the
  command, binary, repo, and crate identifiers as `aiplus`/`aiplus-cli`.
- Added an Owner-gated installer plan for future GitHub Release binaries and
  checksum-verifying install script.
- Added Rust-first `aiplus` CLI workspace.
- Added local vendored AiPlus module asset snapshot.
- Added project-local install/update/add/status/doctor/uninstall workflows.
- Added Codex, Claude Code, OpenCode, and all-runtimes adapter support.
- Added Rust parity and safety tests.
- Documented Node CLI as archived historical reference.
- Replaced compact bridge limitation with Rust-native compact status
  `COMPACT_RUST_NATIVE_STATUS=PASS`.

## Public-ready candidate docs

- Documented recommended public repo name `aiplus`.
- Documented public repo structure with Rust workspace as root.
- Added Owner-gated distribution plan.
- Added planned binary artifact matrix.
- Added migration guide from archived Node CLI.
- Added QA release-readiness checklist.
- Kept crate version `0.1.3` for manifest compatibility.
- Applied Owner-approved Apache-2.0 licensing to the Rust mainline/public-ready
  package metadata and docs.
