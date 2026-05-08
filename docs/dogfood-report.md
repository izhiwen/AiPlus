# Dogfood Report

Date: 2026-05-08

## Automated Evidence

- `cargo test`: PASS
- Rust integration tests cover:
  - doctor missing manifest diagnostics
  - doctor malformed manifest diagnostics
  - doctor wrong-installer manifest diagnostics
  - install codex dry-run and real install
  - status
  - doctor
  - update
  - add dry-run
  - unknown module `MODULE_NOT_AVAILABLE`
  - uninstall dry-run
  - Claude Code, OpenCode, and all-runtimes doctor pass
  - unknown empty directory uninstall guard
  - `install --runtime codex`
  - `install --all-runtimes`
  - Rust-native `compact validate`
  - Rust-native `compact prepare`
  - Rust-native `compact score`
  - Rust-native `compact checkpoint --level`
  - Rust-native `compact resume`
  - compact commands with `PATH` excluding Node
  - static source scan for no `Command::new("node")`
  - dangling `AGENTS.md` symlink rejection

## Manual Smoke Template

```bash
tmp=$(mktemp -d)
cd "$tmp"
AIPLUS_HOME="$HOME/aiplus"
cargo run --manifest-path "$AIPLUS_HOME/Cargo.toml" -p aiplus-cli -- install codex
cargo run --manifest-path "$AIPLUS_HOME/Cargo.toml" -p aiplus-cli -- status
cargo run --manifest-path "$AIPLUS_HOME/Cargo.toml" -p aiplus-cli -- doctor
```

## Compact Status

Compact is Rust-native for `init`, `prepare`, `score`, `validate`, `checkpoint`, and `resume`. The
CLI prints `COMPACT_RUST_NATIVE_STATUS=PASS` for compact commands.

## Public-ready Candidate Smoke

Latest local evidence:

- `cargo fmt --check`: PASS
- `cargo test`: PASS
- `cargo run -p aiplus-cli -- --help`: PASS
- `cargo clippy --all-targets --all-features -- -D warnings`: PASS
- temp `install codex`: PASS
- temp `status`: PASS
- temp `doctor`: PASS
- temp `update`: PASS
- temp `add auto-team-consultant --dry-run`: PASS
- temp `compact init`: PASS
- temp `compact validate`: PASS
- temp `compact prepare`: PASS when readiness is ready; otherwise prints readiness state
- temp `compact score`: PASS when readiness is ready; otherwise prints readiness state
- temp `compact checkpoint`: created checkpoint and printed
  `COMPACT_RUST_NATIVE_STATUS=PASS`; seeded templates may return
  `UNKNOWN_NEEDS_REVIEW` until Owner gates are resolved
- temp `compact resume`: PASS
- temp `uninstall --dry-run`: PASS
- temp `install claude-code` + `doctor`: PASS
- temp `install opencode` + `doctor`: PASS
- temp `install all` + `doctor`: PASS
- installed target scan for `compactctl.mjs`: PASS, no installed result
