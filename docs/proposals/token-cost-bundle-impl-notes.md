# Token Cost Bundle Implementation Notes

Status: Phase 1 design, before implementation.

## 1. Packaging Approach Decision

Decision: use Approach A for release packaging. AiPlus release jobs will build the
`aiplus` CLI from this workspace, then download the standalone
`aiplus-token-cost` binary from `izhiwen/AiPlus-Token-Cost` release `v0.1.0` and
repackage both binaries into one archive.

Approach B was checked first as requested. The workspace package
`crates/aiplus-token-cost` is a library-only subtree mirror with no `src/main.rs`
or `[[bin]]` target. `cargo build --release --target aarch64-apple-darwin -p
aiplus-token-cost --locked` completes, but it produces the library package, not a
standalone executable. Adding a binary target would touch the forbidden subtree
mirror and belongs to Phase C, so Approach B is out of scope for this phase.

## 2. Module Manifest Schema

Add bundled module `token-cost` with vendor directory
`assets/aiplus-token-cost/` and install path
`.aiplus/modules/aiplus-token-cost`.

Manifest fields follow current schema `0.1.0` with one additive optional field:

```json
"binaryAssets": [
  {
    "name": "aiplus-token-cost",
    "windowsName": "aiplus-token-cost.exe",
    "installDir": "$HOME/.local/bin"
  }
]
```

The Rust manifest parser currently uses `deny_unknown_fields`, so the schema and
Rust struct must learn this optional field. Validation should require non-empty
`name` and `installDir` for declared binary assets, while `windowsName` remains
optional for non-Windows binaries. Runtime adapters stay populated with
`codex`, `claude-code`, and `opencode` because the bundled docs are installed
for every runtime and the public CLI also exposes `aiplus agent token-cost`.

## 3. Installer Changes

`install.sh` and `install.ps1` should install both binaries when present in the
downloaded archive:

- Unix archive: `aiplus` and optional `aiplus-token-cost`.
- Windows archive: `aiplus.exe` and optional `aiplus-token-cost.exe`.
- Keep backward compatibility with older single-binary AiPlus archives by
  warning when the token-cost binary is absent instead of failing the install.
- Add a local base-url override for install demos so tests can point the
  installer at a local fixture archive without publishing a release.

## 4. README Update Plan

Update README and README.zh-CN from "six bundled modules" to "seven bundled
modules". Add AiPlus-Token-Cost as the seventh bundled module, with clear
standalone/bundled wording:

- standalone: `aiplus-token-cost`
- bundled: auto-installed by AiPlus release archives and callable through
  `aiplus agent token-cost`

Do not modify the actual `CHANGELOG.md` in this phase per the latest Owner
instruction. Draft entry for Advisor:

```markdown
## Unreleased

- Bundle `aiplus-token-cost` as the seventh public substrate module, include the
  standalone binary in AiPlus release archives, and teach installers to install
  both `aiplus` and `aiplus-token-cost`.
```

## 5. Test Plan

- `cargo fmt --check`
- `cargo test --workspace`
- `cargo clippy --workspace --all-targets -- -D warnings`
- `cargo run -p aiplus-cli --bin aiplus -- doctor` on a temporary installed
  project to confirm module manifest validation and token-cost PATH info.
- `cargo run -p aiplus-cli --bin aiplus -- agent token-cost --help` to verify
  the existing subcommand remains wired.
- Unix bundled install demo: point `install.sh` at a local fixture archive
  containing both `aiplus` and `aiplus-token-cost`, verify both are installed.
- Windows bundled install demo: when PowerShell is available locally, point
  `install.ps1` at a local fixture zip containing both `.exe` binaries and
  verify both are installed. If PowerShell is unavailable on this machine, record
  the syntax/static verification and the Unix live demo separately.

## Phase 3 Evidence

Implementation result:

- `_IMPL-OK_` assets: added `assets/aiplus-token-cost/` with manifest, README,
  README.zh-CN, LICENSE, CHANGELOG, SECURITY, MODULES, RELEASE_CHECKLIST, and
  runtime adapter notes.
- `_IMPL-OK_` release packaging: `.github/workflows/release.yml` now builds
  `aiplus`, downloads standalone `aiplus-token-cost` `v0.1.0`, and packages
  both binaries into tar.gz/zip release archives.
- `_IMPL-OK_` installers: `install.sh` and `install.ps1` install the second
  binary when present and remain backward-compatible with older single-binary
  archives.
- `_IMPL-OK_` README: README and README.zh-CN describe seven bundled modules and
  document standalone/bundled token-cost usage.
- `_IMPL-OK_` doctor/install plumbing: bundled module registry includes
  `token-cost`; manifest schema accepts `binaryAssets`; `aiplus doctor` reports
  token-cost binary PATH status as INFO.

Evidence:

```text
cargo fmt --check
PASS

cargo test --workspace
cargo test: 569 passed, 1 ignored (45 suites, 41.77s)

cargo clippy --workspace --all-targets -- -D warnings
cargo clippy: No issues found

Standalone release demo:
aiplus-token-cost 0.1.0

Bundled Unix install fixture:
DEMO_INSTALL_STATUS=PASS
installed_count=2
installed=.../bin/aiplus
installed=.../bin/aiplus-token-cost

Doctor demo in temporary HOME/project with token-cost on PATH:
PASS module manifest token-cost present
PASS aiplus-token-cost binary on PATH
PASS token-cost README.md exists
DOCTOR_STATUS=PASS

Bundled CLI regression:
aiplus agent token-cost --help
Usage: aiplus agent token-cost [OPTIONS]
```

Windows installer note: this macOS machine does not have `pwsh`, so the live
PowerShell fixture could not be executed locally. The implementation mirrors the
Unix dual-binary behavior in `install.ps1` and keeps backward-compatible
single-binary archive handling.
