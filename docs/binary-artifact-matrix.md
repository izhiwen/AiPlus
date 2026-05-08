# Binary Artifact Matrix

Status: `PLANNED_NOT_PUBLISHED`

No binary artifacts have been created or published from this plan.

| Artifact | Target triple | Build command | Checksum plan | Signature / notarization | Tested status | Owner gate |
| --- | --- | --- | --- | --- | --- | --- |
| `aiplus-aarch64-apple-darwin.tar.gz` | `aarch64-apple-darwin` | `cargo build --release --target aarch64-apple-darwin` | `shasum -a 256 <artifact> > <artifact>.sha256` | Not signed or notarized yet | Local Apple Silicon test required | Required before upload |
| `aiplus-x86_64-apple-darwin.tar.gz` | `x86_64-apple-darwin` | `cargo build --release --target x86_64-apple-darwin` | `shasum -a 256 <artifact> > <artifact>.sha256` | Not signed or notarized yet | Intel macOS or cross-test required | Required before upload |
| `aiplus-x86_64-unknown-linux-gnu.tar.gz` | `x86_64-unknown-linux-gnu` | `cargo build --release --target x86_64-unknown-linux-gnu` | `sha256sum <artifact> > <artifact>.sha256` | Not signed | Linux x86_64 test required | Required before upload |
| `aiplus-aarch64-unknown-linux-gnu.tar.gz` | `aarch64-unknown-linux-gnu` | `cargo build --release --target aarch64-unknown-linux-gnu` | `sha256sum <artifact> > <artifact>.sha256` | Not signed | Linux ARM64 test required | Required before upload |
| `aiplus-x86_64-pc-windows-msvc.zip` | `x86_64-pc-windows-msvc` | `cargo build --release --target x86_64-pc-windows-msvc` | `CertUtil -hashfile <artifact> SHA256` or `sha256sum` from release builder | Not signed | Windows x86_64 test required | Required before upload |

## Archive Contents

Each archive should contain:

```text
aiplus
README.md
LICENSE
checksums or checksum pointer
```

Windows archive should contain `aiplus.exe`.

## Release Gate

Before publishing any artifact:

- run the QA matrix in [qa-release-readiness.md](qa-release-readiness.md)
- run smoke tests on the target platform or mark the platform untested
- verify `aiplus --help`
- verify `aiplus install codex` in a temp project
- verify `doctor`, `status`, `compact validate`, `uninstall --dry-run`
- verify archive includes Apache-2.0 `LICENSE`
- generate checksums
- review archive contents for private paths and build artifacts
- get Owner approval for tag and GitHub Release
