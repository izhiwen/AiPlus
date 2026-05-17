// Issue #74 / Track A.3: doctor must classify stale-registry entries
// as INFO, not NEEDS_FIX.
//
// The `~/.config/aiplus/installed-projects.json` registry accumulates
// an entry for every project AiPlus has ever installed into. When a
// project directory is later deleted, the registry retains the entry
// pointing to a non-existent path. Before this fix, `aiplus doctor`
// flipped DOCTOR_STATUS to NEEDS_FIX for that purely cosmetic
// condition — diluting the meaning of NEEDS_FIX (which should mean
// "something is actually broken").
//
// After the fix:
//   - The stale-entries line uses an INFO prefix.
//   - DOCTOR_STATUS stays PASS when stale entries are the ONLY issue.
//   - The fix hint (`run aiplus prune-projects --yes`) still shows.

use std::fs;
use std::path::Path;
use std::process::{Command, Output};

fn bin() -> &'static str {
    env!("CARGO_BIN_EXE_aiplus")
}

fn run(cwd: &Path, args: &[&str], expected: i32) -> Output {
    let mut command = Command::new(bin());
    command
        .args(args)
        .current_dir(cwd)
        .env("HOME", cwd.join("fake-home"))
        .env("CODEX_HOME", cwd.join("fake-codex-home"))
        .env("XDG_CONFIG_HOME", cwd.join("fake-xdg"));
    let output = command.output().expect("run aiplus");
    assert_eq!(
        output.status.code(),
        Some(expected),
        "{} failed\nstdout:\n{}\nstderr:\n{}",
        args.join(" "),
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    output
}

fn prepare(target: &Path) {
    fs::create_dir(target.join("fake-home")).unwrap();
    fs::create_dir(target.join("fake-codex-home")).unwrap();
    fs::create_dir(target.join("fake-xdg")).unwrap();
}

/// Write a registry file at the right XDG_CONFIG_HOME location with
/// `entries` rows. The path field is taken verbatim — pass paths that
/// don't exist on disk to simulate stale entries.
///
/// JSON schema matches what `read_registry` deserializes: snake_case
/// `schema_version` + `installed_projects`, entries with `path` +
/// `first_installed` + `last_updated` + `runtimes`.
fn write_registry(target: &Path, entries: &[(&str, &str)]) {
    let registry_path = target
        .join("fake-xdg")
        .join("aiplus")
        .join("installed-projects.json");
    fs::create_dir_all(registry_path.parent().unwrap()).unwrap();
    let mut body = String::from(r#"{"schema_version":"1.0","installed_projects":["#);
    for (i, (path, ts)) in entries.iter().enumerate() {
        if i > 0 {
            body.push(',');
        }
        body.push_str(&format!(
            r#"{{"path":"{path}","first_installed":"{ts}","last_updated":"{ts}","runtimes":["codex"]}}"#
        ));
    }
    body.push_str("]}");
    fs::write(&registry_path, body).unwrap();
}

#[test]
fn doctor_with_only_stale_registry_entries_returns_pass_status() {
    let temp = tempfile::tempdir().unwrap();
    let target = temp.path();
    prepare(target);

    // Install AiPlus into the project so doctor has a real install
    // to evaluate.
    run(target, &["install", "codex"], 0);

    // Inject 3 stale entries (pointing to non-existent paths) on top
    // of the legitimate current-project entry the install just wrote.
    // We rewrite the whole registry to keep this deterministic.
    write_registry(
        target,
        &[
            (target.to_str().unwrap(), "2026-05-14T00:00:00.000Z"),
            ("/tmp/aiplus-gone-1", "2026-05-14T00:00:00.000Z"),
            ("/tmp/aiplus-gone-2", "2026-05-14T00:00:00.000Z"),
            ("/tmp/aiplus-gone-3", "2026-05-14T00:00:00.000Z"),
        ],
    );

    let out = run(target, &["doctor"], 0);
    let text = String::from_utf8_lossy(&out.stdout);

    // Headline: with stale entries as the only "issue", DOCTOR_STATUS
    // must be PASS — not NEEDS_FIX.
    assert!(
        text.contains("DOCTOR_STATUS=PASS"),
        "stale-registry-only state should report DOCTOR_STATUS=PASS, got:\n{text}"
    );

    // The stale-entries line uses the INFO prefix, not NEEDS_FIX.
    assert!(
        text.contains("INFO registry has 3 stale entries"),
        "stale-entries line should have INFO prefix, got:\n{text}"
    );
    assert!(
        !text.contains("NEEDS_FIX registry has"),
        "stale-entries must not surface as NEEDS_FIX:\n{text}"
    );

    // The fix hint is still attached.
    assert!(
        text.contains("aiplus prune-projects --yes"),
        "fix hint missing:\n{text}"
    );
}

#[test]
fn doctor_with_no_stale_registry_still_passes() {
    // Sanity: the new INFO classification must not break the clean
    // case. A fresh install with zero stale entries should still
    // print `PASS registry has 0 stale entries` (not INFO).
    let temp = tempfile::tempdir().unwrap();
    let target = temp.path();
    prepare(target);

    run(target, &["install", "codex"], 0);
    let out = run(target, &["doctor"], 0);
    let text = String::from_utf8_lossy(&out.stdout);

    assert!(
        text.contains("DOCTOR_STATUS=PASS"),
        "clean install should pass:\n{text}"
    );
    assert!(
        text.contains("PASS registry has 0 stale entries"),
        "zero stale entries should still print PASS (not INFO):\n{text}"
    );
}

#[test]
fn doctor_accepts_legacy_registry_with_trailing_delimiters() {
    // A prior writer bug could leave a valid registry object followed
    // by stray JSON delimiters. Doctor should not fail the current
    // project when the first registry object is recoverable.
    let temp = tempfile::tempdir().unwrap();
    let target = temp.path();
    prepare(target);

    run(target, &["install", "codex"], 0);

    let registry_path = target
        .join("fake-xdg")
        .join("aiplus")
        .join("installed-projects.json");
    let mut registry = fs::read_to_string(&registry_path).unwrap();
    registry.push_str("\n}]\n}");
    fs::write(&registry_path, registry).unwrap();

    let out = run(target, &["doctor"], 0);
    let text = String::from_utf8_lossy(&out.stdout);

    assert!(
        text.contains("DOCTOR_STATUS=PASS"),
        "recoverable trailing registry delimiters should not fail doctor:\n{text}"
    );
    assert!(
        text.contains("PASS registry parses as JSON with schema_version=1.0"),
        "registry parse check should pass after recovery:\n{text}"
    );
}

#[test]
fn doctor_with_genuine_needs_fix_still_reports_needs_fix() {
    // Regression guard: the INFO classification must only soften
    // stale-registry entries. Genuine install-correctness failures
    // (here: corrupt the manifest schemaVersion) must still flip
    // DOCTOR_STATUS to NEEDS_FIX.
    let temp = tempfile::tempdir().unwrap();
    let target = temp.path();
    prepare(target);

    run(target, &["install", "codex"], 0);

    // Break the manifest by setting an unsupported schemaVersion.
    let manifest_path = target.join(".aiplus/manifest.json");
    let manifest = fs::read_to_string(&manifest_path).unwrap();
    let broken = manifest.replace(
        "\"schemaVersion\":",
        "\"schemaVersion\":\"0.0.99-broken\",\"_orig_schemaVersion\":",
    );
    fs::write(&manifest_path, broken).unwrap();

    // Also inject some stale registry entries so we exercise the
    // mixed case: NeedsFix failure + Info failure together must
    // still report NEEDS_FIX overall.
    write_registry(
        target,
        &[("/tmp/aiplus-gone-stranger", "2026-05-14T00:00:00.000Z")],
    );

    let out = Command::new(bin())
        .args(["doctor"])
        .current_dir(target)
        .env("HOME", target.join("fake-home"))
        .env("CODEX_HOME", target.join("fake-codex-home"))
        .env("XDG_CONFIG_HOME", target.join("fake-xdg"))
        .output()
        .expect("run aiplus doctor");
    let text = String::from_utf8_lossy(&out.stdout);

    assert!(
        text.contains("DOCTOR_STATUS=NEEDS_FIX"),
        "genuine schemaVersion failure must still flip DOCTOR_STATUS, got:\n{text}"
    );
    // The stale-entries line is INFO; the schemaVersion failure is NEEDS_FIX.
    assert!(
        text.contains("INFO registry has 1 stale entries"),
        "stale entries still surface as INFO even alongside NeedsFix:\n{text}"
    );
}
