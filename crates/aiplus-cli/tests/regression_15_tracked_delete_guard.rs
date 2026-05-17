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

fn git(cwd: &Path, args: &[&str]) -> Output {
    let output = Command::new("git")
        .args(args)
        .current_dir(cwd)
        .output()
        .expect("run git");
    assert!(
        output.status.success(),
        "git {} failed\nstdout:\n{}\nstderr:\n{}",
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

fn legacy_schema_path(target: &Path) -> std::path::PathBuf {
    target.join(".aiplus/aieconlab/acceptance/v0.1.0/schema.yaml")
}

fn write_legacy_schema(target: &Path) {
    let schema = legacy_schema_path(target);
    fs::create_dir_all(schema.parent().unwrap()).unwrap();
    fs::write(&schema, "schemaVersion: 0.1.0\n").unwrap();
}

#[test]
fn install_refuses_to_delete_git_tracked_legacy_aieconlab_schema() {
    let temp = tempfile::tempdir().unwrap();
    let target = temp.path();
    prepare(target);
    write_legacy_schema(target);
    git(target, &["init"]);
    git(
        target,
        &["add", ".aiplus/aieconlab/acceptance/v0.1.0/schema.yaml"],
    );

    let output = run(target, &["install", "codex", "--yes"], 1);
    let combined = format!(
        "{}\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    assert!(
        combined.contains("ERROR refusing to delete git-tracked file(s)"),
        "{combined}"
    );
    assert!(
        combined.contains(".aiplus/aieconlab/acceptance/v0.1.0/schema.yaml"),
        "{combined}"
    );
    assert!(
        combined.contains("file an issue"),
        "error should include module-manifest issue hint:\n{combined}"
    );
    assert!(
        legacy_schema_path(target).exists(),
        "tracked legacy schema must be preserved"
    );

    let status = git(target, &["status", "--short"]);
    let status = String::from_utf8_lossy(&status.stdout);
    assert!(
        !status
            .lines()
            .any(|line| line.starts_with(" D") || line.starts_with("D ")),
        "tracked deletions should not appear:\n{status}"
    );
}

#[test]
fn install_still_removes_untracked_legacy_aieconlab_schema() {
    let temp = tempfile::tempdir().unwrap();
    let target = temp.path();
    prepare(target);
    write_legacy_schema(target);
    git(target, &["init"]);

    run(target, &["install", "codex", "--yes"], 0);

    assert!(
        !legacy_schema_path(target).exists(),
        "untracked legacy schema should still be removed by residue cleanup"
    );
}
