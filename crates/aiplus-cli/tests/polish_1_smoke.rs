use serde_json::Value;
use std::fs;
use std::path::Path;
use std::process::{Command, Output};

fn bin() -> &'static str {
    env!("CARGO_BIN_EXE_aiplus")
}

fn run(cwd: &Path, args: &[&str], expected: i32) -> Output {
    let output = Command::new(bin())
        .args(args)
        .current_dir(cwd)
        .env("HOME", cwd.join("fake-home"))
        .env("CODEX_HOME", cwd.join("fake-codex-home"))
        .env("XDG_CONFIG_HOME", cwd.join("fake-xdg"))
        .env("AIPLUS_SECRET_BROKER_DISABLE_KEYCHAIN", "1")
        .env_remove("ANTHROPIC_API_KEY")
        .env_remove("OPENAI_API_KEY")
        .env_remove("BWS_ACCESS_TOKEN")
        .output()
        .expect("run aiplus");
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

fn init_git_repo(target: &Path) {
    Command::new("git")
        .args(["init"])
        .current_dir(target)
        .output()
        .expect("git init");
    Command::new("git")
        .args(["config", "user.email", "test@example.com"])
        .current_dir(target)
        .output()
        .expect("git config email");
    Command::new("git")
        .args(["config", "user.name", "Test User"])
        .current_dir(target)
        .output()
        .expect("git config name");
}

#[test]
fn dispatch_log_rows_include_schema_version_0_4_0() {
    let temp = tempfile::tempdir().unwrap();
    let target = temp.path();
    fs::write(target.join("README.md"), "# Polish 1\n").unwrap();
    init_git_repo(target);

    run(
        target,
        &["agent", "route", "--score-only", "describe git status"],
        0,
    );

    let body = fs::read_to_string(target.join(".aiplus/agents/dispatch-log.jsonl")).unwrap();
    let entries: Vec<Value> = body
        .lines()
        .map(|line| serde_json::from_str(line).expect("jsonl line"))
        .collect();
    assert!(!entries.is_empty(), "dispatch log should have entries");
    assert!(
        entries
            .iter()
            .all(|entry| entry.get("schemaVersion").and_then(Value::as_str) == Some("0.4.0")),
        "all new dispatch-log rows should carry schemaVersion=0.4.0:\n{body}"
    );
}

#[test]
fn agent_doctor_quiet_suppresses_info_and_keeps_status() {
    let temp = tempfile::tempdir().unwrap();
    let target = temp.path();

    let out = stdout(&run(target, &["agent", "doctor", "--quiet"], 0));

    assert!(
        out.contains("WARNING: .aiplus/agents/ does not exist"),
        "quiet should keep warnings:\n{out}"
    );
    assert!(
        out.contains("DOCTOR_STATUS=WARN"),
        "quiet should keep final doctor status:\n{out}"
    );
    assert!(
        !out.contains("INFO"),
        "quiet should suppress INFO lines:\n{out}"
    );
    assert!(
        !out.contains("Running agent team doctor"),
        "quiet should suppress banner:\n{out}"
    );
}
