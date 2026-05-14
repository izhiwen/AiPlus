// Invariant test for `install.sh` fallback version constant.
//
// `install.sh` has a hard-coded fallback used only when both `gh api` and
// `curl https://api.github.com/.../releases/latest` lookups fail (offline /
// API down). Historically this constant drifted (last-known-good v0.5.11 was
// still in the file when current Latest was v0.5.15). See issue #35.
//
// Enforce: the fallback constant must equal the current `aiplus-cli`
// Cargo.toml version. Whenever the release process bumps Cargo.toml, this
// test forces the installer fallback to bump in the same commit.

use std::path::PathBuf;

#[test]
fn install_sh_fallback_matches_cli_cargo_version() {
    let cli_version = env!("CARGO_PKG_VERSION");
    let expected = format!("v{cli_version}");

    // crates/aiplus-cli/tests/ -> repo root
    let repo_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(|p| p.parent())
        .expect("repo root")
        .to_path_buf();
    let install_sh = repo_root.join("install.sh");
    let body = std::fs::read_to_string(&install_sh)
        .unwrap_or_else(|e| panic!("read {}: {e}", install_sh.display()));

    // Find the line: VERSION="${VERSION:-vX.Y.Z}"
    let needle = "VERSION=\"${VERSION:-";
    let line = body
        .lines()
        .find(|l| l.contains(needle))
        .unwrap_or_else(|| panic!("install.sh missing VERSION fallback line"));

    assert!(
        line.contains(&format!("{needle}{expected}}}")),
        "install.sh fallback line {line:?} should contain VERSION:-{expected} \
         to match aiplus-cli Cargo.toml version {expected}; bump install.sh \
         whenever you bump Cargo.toml."
    );
}
