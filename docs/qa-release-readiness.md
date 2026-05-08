# QA Release Readiness

Status: `OWNER_REVIEW_READY_CANDIDATE`

This checklist prepares the Rust CLI for Owner review. It does not approve
publication.

## Required Local Commands

Run from the repository root:

```bash
cargo fmt --check
cargo test
cargo run -p aiplus-cli -- --help
cargo clippy --all-targets --all-features -- -D warnings
cargo metadata --format-version 1
```

## Manual Smoke Matrix

Use temp directories only:

```bash
tmp=$(mktemp -d)
cd "$tmp"
AIPLUS_HOME="$HOME/aiplus"
cargo run --manifest-path "$AIPLUS_HOME/Cargo.toml" -p aiplus-cli -- install codex
cargo run --manifest-path "$AIPLUS_HOME/Cargo.toml" -p aiplus-cli -- status
cargo run --manifest-path "$AIPLUS_HOME/Cargo.toml" -p aiplus-cli -- doctor
cargo run --manifest-path "$AIPLUS_HOME/Cargo.toml" -p aiplus-cli -- update
cargo run --manifest-path "$AIPLUS_HOME/Cargo.toml" -p aiplus-cli -- add auto-team-consultant --dry-run
cargo run --manifest-path "$AIPLUS_HOME/Cargo.toml" -p aiplus-cli -- compact init
cargo run --manifest-path "$AIPLUS_HOME/Cargo.toml" -p aiplus-cli -- compact validate
cargo run --manifest-path "$AIPLUS_HOME/Cargo.toml" -p aiplus-cli -- compact checkpoint
cargo run --manifest-path "$AIPLUS_HOME/Cargo.toml" -p aiplus-cli -- compact resume
cargo run --manifest-path "$AIPLUS_HOME/Cargo.toml" -p aiplus-cli -- uninstall --dry-run
```

Repeat install + doctor for:

```bash
aiplus install codex
aiplus install claude-code
aiplus install opencode
aiplus install all
```

## Expected Markers

- `INSTALL_STATUS=PASS`
- `STATUS=PASS`
- `DOCTOR_STATUS=PASS`
- `UPDATE_STATUS=PASS`
- `ADD_DRY_RUN=PASS`
- `UNINSTALL_DRY_RUN=PASS`
- `COMPACT_RUST_NATIVE_STATUS=PASS`
- `GLOBAL_CONFIG_UNTOUCHED`

`compact checkpoint` may return `UNKNOWN_NEEDS_REVIEW` when seeded templates
contain pending Owner gates. That is expected safety behavior.

## Static Scans

```bash
rg -n 'Command::new\("node"\)' crates tests docs
find . -path './target' -prune -o -name compactctl.mjs -print
find . -path './target' -prune -o -name .DS_Store -print
rg -n 'AIPLUS''_SOURCE|/path/to/aiplus/target/release/aiplus' README.md README.zh-CN.md docs
rg -n 'UNLICENSED' .
rg -n 'Apache-2.0|LICENSE' README.md README.zh-CN.md Cargo.toml crates docs
rg -n 'guaranteed safe|certified|compliant|secure by default|production-ready|official|endorsed|privacy guaranteed|safety approved' .
rg -n 'npm publish|cargo publish|git push|git tag|GitHub Release|global install|telemetry|network' .
```

Classify matches in docs as forbidden-action warnings or historical reference
records. Active source must not implement forbidden actions.

## Release Readiness Decision

Before any public release:

- Owner reviews repo name and extraction plan.
- Owner confirms Apache-2.0 license/public wording remains correct.
- Owner approves tag and release channel.
- QA matrix is re-run after extraction.
- Binary artifact matrix is updated with tested status.
- Checksums are generated for any release artifacts.
