use serde_json::Value;
use std::fs;
use std::path::Path;
use std::process::{Command, Output};

fn bin() -> &'static str {
    env!("CARGO_BIN_EXE_aiplus")
}

fn command(cwd: &Path) -> Command {
    let mut cmd = Command::new(bin());
    cmd.current_dir(cwd)
        .env("HOME", cwd.join("fake-home"))
        .env("CODEX_HOME", cwd.join("fake-codex-home"))
        .env("XDG_CONFIG_HOME", cwd.join("fake-xdg"))
        .env("AIPLUS_SECRET_BROKER_DISABLE_KEYCHAIN", "1")
        .env_remove("ANTHROPIC_API_KEY")
        .env_remove("OPENAI_API_KEY")
        .env_remove("BWS_ACCESS_TOKEN");
    cmd
}

fn run(cwd: &Path, args: &[&str], expected: i32) -> Output {
    let output = command(cwd).args(args).output().expect("run aiplus");
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

fn dispatch_log(target: &Path) -> std::path::PathBuf {
    target.join(".aiplus/agents/dispatch-log.jsonl")
}

#[test]
fn verify_log_passes_on_fresh_chained_dispatch_log() {
    let temp = tempfile::tempdir().unwrap();
    let target = temp.path();

    run(
        target,
        &["agent", "route", "--score-only", "describe git status"],
        0,
    );
    run(
        target,
        &["agent", "route", "--score-only", "summarize README"],
        0,
    );

    let body = fs::read_to_string(dispatch_log(target)).unwrap();
    let entries = body
        .lines()
        .map(|line| serde_json::from_str::<Value>(line).unwrap())
        .collect::<Vec<_>>();
    assert_eq!(
        entries[0].get("genesis").and_then(Value::as_bool),
        Some(true)
    );
    assert!(entries[0].get("entry_hash").is_some());
    assert!(entries[1].get("prev_hash").is_some());
    assert!(entries[1].get("entry_hash").is_some());

    let verify = stdout(&run(target, &["agent", "audit", "verify-log"], 0));
    assert!(verify.contains("VERIFY_LOG=PASS"), "{verify}");
}

#[test]
fn verify_log_fails_with_line_number_after_tamper() {
    let temp = tempfile::tempdir().unwrap();
    let target = temp.path();

    run(
        target,
        &["agent", "route", "--score-only", "describe git status"],
        0,
    );
    run(
        target,
        &["agent", "route", "--score-only", "summarize README"],
        0,
    );

    let path = dispatch_log(target);
    let mut lines = fs::read_to_string(&path)
        .unwrap()
        .lines()
        .map(ToString::to_string)
        .collect::<Vec<_>>();
    let mut second: Value = serde_json::from_str(&lines[1]).unwrap();
    second["taskExcerpt"] = Value::String("tampered task".to_string());
    lines[1] = serde_json::to_string(&second).unwrap();
    fs::write(&path, format!("{}\n", lines.join("\n"))).unwrap();

    let verify = stdout(&run(target, &["agent", "audit", "verify-log"], 3));
    assert!(verify.contains("VERIFY_LOG=FAIL line=2"), "{verify}");
}

#[test]
fn verify_log_ignores_legacy_rows_before_genesis() {
    let temp = tempfile::tempdir().unwrap();
    let target = temp.path();
    let path = dispatch_log(target);
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(
        &path,
        "{\"event\":\"legacy_dispatch\",\"task\":\"old unchained row\"}\n",
    )
    .unwrap();

    run(
        target,
        &["agent", "route", "--score-only", "describe git status"],
        0,
    );
    let body = fs::read_to_string(&path).unwrap();
    let entries = body
        .lines()
        .map(|line| serde_json::from_str::<Value>(line).unwrap())
        .collect::<Vec<_>>();
    assert!(entries[0].get("entry_hash").is_none());
    assert_eq!(
        entries[1].get("genesis").and_then(Value::as_bool),
        Some(true)
    );

    let verify = stdout(&run(target, &["agent", "audit", "verify-log"], 0));
    assert!(verify.contains("VERIFY_LOG=PASS"), "{verify}");
}

#[test]
fn doctor_reports_dispatch_log_chain_status() {
    let temp = tempfile::tempdir().unwrap();
    let target = temp.path();
    run(
        target,
        &["agent", "route", "--score-only", "describe git status"],
        0,
    );

    let doctor = stdout(&run(target, &["agent", "doctor"], 0));
    assert!(doctor.contains("INFO dispatch_log_chain=valid"), "{doctor}");
}
