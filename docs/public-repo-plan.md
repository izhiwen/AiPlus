# Public Repo Plan

Status: `PUBLIC_READY_CANDIDATE`

This plan is documentation only. It does not create repositories, push commits,
create tags, publish releases, or submit packages.

## Naming Recommendation

Preferred public repo name: `aiplus`.

Reasons:

- It matches the user command: `aiplus`.
- It matches the product/module family name.
- It avoids confusing the archived Node directory `legacy/aiplus-cli` with the Rust
  mainline.
- It leaves room for future module and runtime adapters without making the repo
  name language-specific.

Do not use `aiplus-rust` as the public user-facing repo name unless Owner decides
the repo is temporary or Rust-only by policy.

## Recommended Public Structure

The public repo root should be the Rust workspace:

```text
aiplus/
  README.md
  README.zh-CN.md
  MODULES.md
  Cargo.toml
  Cargo.lock
  crates/
    aiplus-cli/
      Cargo.toml
      src/main.rs
      tests/parity.rs
  assets/
    README.md
    aiplus-compact-reminder/
    aiplus-auto-team-consultant/
  docs/
    architecture.md
    safety.md
    public-repo-plan.md
    distribution-plan.md
    binary-artifact-matrix.md
    migration-from-node-cli.md
    qa-release-readiness.md
    node-parity.md
    dogfood-report.md
  tests/
    README.md
  CHANGELOG.md
  RELEASE_CHECKLIST.md
```

## Boundaries

- Rust workspace is mainline.
- `assets/` contains bundled module snapshots used by local installs.
- Public module repos remain independent records.
- Archived Node reference remains outside the beginner path.
- No public release artifacts are created until Owner approval.
- Rust mainline/public-ready package license is Apache-2.0.
- Bundled child module snapshots preserve existing licenses.

## Extraction Checklist

- [ ] Owner approves creating or reusing `github.com/izhiwen/aiplus`.
- [ ] Move Rust workspace contents to public repo root.
- [ ] Preserve `Cargo.lock`.
- [ ] Preserve `assets/` as bundled snapshots with provenance notes.
- [ ] Preserve `LICENSE` and Apache-2.0 Cargo metadata.
- [ ] Document bundled child module licenses.
- [ ] Keep README beginner commands copy-pasteable without undefined
      placeholders.
- [ ] Keep archived Node reference in this local workspace or a clearly labeled
      archive location only if Owner approves moving it.
- [ ] Re-run QA matrix after extraction.
- [ ] Re-run safety scan for private paths and local-only assumptions.
- [ ] Owner approves tag/release before any public artifact upload.

## Not Approved By This Plan

- `git push`
- creating a remote repo
- creating or pushing tags
- GitHub Releases
- `cargo publish`
- npm wrapper publication
- Homebrew tap or release
- shell installer publication
- global install
- telemetry or auto-update
