# AiPlus Rust Release Checklist

Status: `V0_4_7_RELEASE_APPROVED_SCOPE`

This checklist tracks the Owner-approved v0.4.8 GitHub Release scope. It does
not approve package registries, Homebrew, marketplace publication, telemetry, or
system/global install paths.

## Owner Gates

- [x] Owner approves public repo name and structure.
- [x] Owner selected Apache-2.0 for the Rust mainline/public-ready package.
- [ ] Owner approves any future license change away from Apache-2.0 or public
      legal wording change.
- [x] Owner approves creating or using `github.com/izhiwen/aiplus`.
- [x] Owner approves git push for reviewed v0.4.8 source/docs changes.
- [x] Owner approves creating the `v0.4.8` git tag.
- [x] Owner approves the `v0.4.8` GitHub Release.
- [x] Owner approves uploading the verified macOS Apple Silicon binary and
      `checksums.txt`.
- [ ] Owner approves any package registry publication.
- [x] Owner approves publishing `install.sh` for user-level
      `~/.local/bin/aiplus` installs.
- [ ] Owner approves any Homebrew, npm wrapper, marketplace, system/global
      install, or package registry channel.

## Local QA Gates

- [x] `cargo fmt --check`
- [x] `cargo test`
- [x] `cargo run -p aiplus-cli -- --help`
- [x] `cargo clippy --all-targets --all-features -- -D warnings`
- [x] `cargo metadata --format-version 1 --no-deps`
- [x] `sh install.sh --dry-run`
- [x] Fake-HOME installer smoke verifies checksum and runs installed binary.
- [x] Manual smoke: `install codex`
- [x] Manual smoke: `install claude-code`
- [x] Manual smoke: `install opencode`
- [x] Manual smoke: `install all`
- [x] Manual smoke: `status`
- [x] Manual smoke: `doctor`
- [x] Manual smoke: `update`
- [x] Manual smoke: `add auto-team-consultant --dry-run`
- [x] Manual smoke: `compact init`
- [x] Manual smoke: `compact validate`
- [x] Manual smoke: `compact prepare`
- [x] Manual smoke: `compact score`
- [x] Manual smoke: `compact checkpoint`
- [x] Manual smoke: `compact checkpoint --level light|standard|full`
- [x] Manual smoke: `compact resume`
- [x] Manual smoke: `compact savings`
- [x] Manual smoke: `compact savings --json`
- [x] Manual smoke: `pricing status`
- [x] Manual smoke: `pricing update`
- [x] Manual smoke: `profile status`
- [x] Manual smoke: `secret-broker status`
- [x] Manual smoke: `secret-broker list`
- [x] Mock smoke: `secret-broker resolve kimi|deepseek|qwen` does not print
      secret values.
- [x] Mock smoke: `secret-broker run --aliases openai,kimi` resolves only
      requested aliases.
- [x] Mock smoke: unrequested failing aliases do not block best-effort
      `secret-broker run -- <command...>`.
- [x] Mock smoke: requested failing aliases fail clearly.
- [x] Mock smoke: placeholder, empty, and whitespace-only Bitwarden values fail
      with `reason=secret_placeholder_or_empty`.
- [x] Mock smoke: requested placeholder aliases are not injected by
      `secret-broker run --aliases`.
- [x] Mock smoke: unrequested placeholder aliases do not block selective valid
      alias injection.
- [x] Mock smoke: `kimi` provider metadata uses Kimi Code endpoint and
      `kimi-for-coding`.
- [x] Real Bitwarden smoke with `bws` CLI and read-only machine token
      (`secret-broker doctor`, `list`, representative default `resolve`, and
      harmless `run --`). Mark blocked/not-run if `bws` is unavailable.
- [x] Real provider smoke: OpenAI, Kimi Code, and DeepSeek `/models` endpoints
      return HTTP 200 with response bodies suppressed.
- [x] Manual smoke: `uninstall --dry-run`

## Static Safety Gates

- [x] No `compactctl.mjs` in Rust install footprint.
- [x] No active `Command::new("node")`.
- [x] Cargo metadata reports `Apache-2.0`.
- [x] `LICENSE` exists in Rust package root.
- [x] Child module licenses are preserved and documented.
- [x] No target/build artifacts in release docs or package staging area.
- [x] No `.DS_Store`.
- [x] No private data, secrets, raw logs, screenshots, or media artifacts.
- [x] No telemetry or user-data upload.
- [x] `compact savings` reads cached pricing only by default.
- [x] `pricing update` fetches public pricing only and never uploads local data.
- [x] No global config writes.
- [x] No publication actions outside the Owner-approved v0.4.8 GitHub Release
      scope executed.
- [x] No overclaim wording such as certified, compliant, official, endorsed,
      guaranteed safe, or production-ready.

## Distribution Docs Gates

- [x] `docs/public-repo-plan.md`
- [x] `docs/distribution-plan.md`
- [x] `docs/binary-artifact-matrix.md`
- [x] `docs/migration-from-node-cli.md`
- [x] `docs/qa-release-readiness.md`
- [x] `docs/safety.md`
- [x] Beginner README path does not recommend Node CLI.
- [x] Node CLI is clearly archived/reference-only.
- [x] Public release docs state what has and has not been released.
- [x] Owner gates are explicit.
- [x] Beginner README commands avoid undefined placeholders.
- [x] Installer behavior is documented with no silent shell profile edits.
- [x] Savings estimates are labeled estimate-only and not billing data.
- [x] Unknown model pricing does not silently use generic fallback as exact
      model-specific pricing.

## Binary Artifact Gates

- [x] Target triples reviewed.
- [x] Build commands reviewed.
- [x] Checksum plan reviewed.
- [x] Signature/notarization status documented.
- [x] Tested status documented per platform.
- [x] Archive contents reviewed.
- [x] Archive contains Apache-2.0 `LICENSE`.
- [x] `checksums.txt` matches uploaded artifacts.
- [x] `install.sh` downloads only release assets and installs only `aiplus`.
- [x] Owner approved the v0.4.8 upload scope before release artifact creation.
