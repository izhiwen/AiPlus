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

fn stdout(output: &Output) -> String {
    String::from_utf8_lossy(&output.stdout).to_string()
}

#[test]
fn audit_run_embedded_schema_in_fresh_project() {
    let temp = tempfile::tempdir().unwrap();
    let target = temp.path();
    fs::create_dir(target.join("fake-home")).unwrap();
    fs::create_dir(target.join("fake-codex-home")).unwrap();
    fs::create_dir(target.join("fake-xdg")).unwrap();

    // Create the parent directory so the audit lock can be created,
    // but do NOT create the acceptance schema file — the embedded schema
    // should be used when the on-disk copy is absent.
    fs::create_dir_all(target.join(".aiplus/agent-team")).unwrap();
    let output = run(
        target,
        &[
            "agent",
            "audit",
            "run",
            "--deliverable",
            "v0.1-stub-not-invitable",
            "--mode",
            "deterministic",
        ],
        0,
    );
    let out = stdout(&output);
    // Verify structured YAML output is produced (not an ENOENT crash)
    assert!(
        out.contains("schema_version:") || out.contains("AUDIT_BLOCKED:"),
        "expected structured audit output, got:\n{out}"
    );
}
