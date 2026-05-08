# QA Release Readiness

Status: `V0_1_3_RELEASE_QA`

This checklist tracks the Owner-approved v0.2.0 GitHub Release QA scope. It
does not approve package registries, Homebrew, marketplace publication,
telemetry, or system/global install paths.

## Required Local Commands

Run from the repository root:

```bash
cargo fmt --check
cargo test
cargo run -p aiplus-cli -- --help
cargo clippy --all-targets --all-features -- -D warnings
cargo metadata --format-version 1
sh install.sh --dry-run
```

## Manual Smoke Matrix

Use temp directories only:

```bash
tmp=$(mktemp -d)
cd "$tmp"
aiplus install codex
aiplus status
aiplus doctor
aiplus update
aiplus add auto-team-consultant --dry-run
aiplus compact init
aiplus compact prepare
aiplus compact score
aiplus compact validate
aiplus compact checkpoint --level standard
aiplus compact resume
aiplus uninstall --dry-run
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
rg -n 'node .*compactctl|compactctl\.mjs (init|validate|checkpoint|resume)' README.md README.zh-CN.md docs crates assets
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

Before the v0.2.0 GitHub Release:

- Owner reviewed repo name and extraction plan.
- Owner confirmed Apache-2.0 license/public wording remains correct.
- Owner approved the `v0.2.0` tag and GitHub Release channel.
- QA matrix is re-run after extraction.
- Binary artifact matrix is updated with tested status.
- Checksums are generated for any release artifacts.

Any package registry, Homebrew, npm wrapper, marketplace, telemetry, or
system/global install path remains out of scope.
