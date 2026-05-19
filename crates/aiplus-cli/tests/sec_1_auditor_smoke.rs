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

fn dispatch_entries(target: &Path) -> Vec<Value> {
    fs::read_to_string(target.join(".aiplus/agents/dispatch-log.jsonl"))
        .expect("dispatch log exists")
        .lines()
        .map(|line| serde_json::from_str::<Value>(line).expect("dispatch JSON"))
        .collect()
}

#[cfg(unix)]
fn install_fake_codex(target: &Path) -> String {
    let fake_bin = target.join("fake-bin");
    fs::create_dir_all(&fake_bin).unwrap();
    let codex = fake_bin.join("codex");
    fs::write(
        &codex,
        "#!/bin/sh\nprintf 'VERDICT: flag\\nREASON: ambiguous payment and auth risk\\n'\n",
    )
    .unwrap();
    {
        use std::os::unix::fs::PermissionsExt;
        let mut perms = fs::metadata(&codex).unwrap().permissions();
        perms.set_mode(0o755);
        fs::set_permissions(&codex, perms).unwrap();
    }
    format!(
        "{}:{}",
        fake_bin.display(),
        std::env::var("PATH").unwrap_or_default()
    )
}

#[cfg(unix)]
#[test]
fn auditor_provider_records_flag_verdict_event() {
    let temp = tempfile::tempdir().unwrap();
    let target = temp.path();
    let path_env = install_fake_codex(target);

    let output = command(target)
        .env("PATH", path_env)
        .args([
            "agent",
            "route",
            "--auditor-provider",
            "codex",
            "review ambiguous secure payment plan",
        ])
        .output()
        .expect("run audited route");
    assert_eq!(
        output.status.code(),
        Some(0),
        "stdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let output = stdout(&output);
    assert!(
        output.contains("Auditor verdict recorded: provider=codex verdict=flag"),
        "{output}"
    );

    let entries = dispatch_entries(target);
    let verdict = entries
        .iter()
        .find(|entry| entry.get("event").and_then(Value::as_str) == Some("auditor_verdict"))
        .expect("auditor verdict event");
    assert_eq!(
        verdict.get("auditor_provider").and_then(Value::as_str),
        Some("codex")
    );
    assert_eq!(
        verdict.get("primary_provider").and_then(Value::as_str),
        Some("local-cli")
    );
    assert_eq!(verdict.get("verdict").and_then(Value::as_str), Some("flag"));
    assert_eq!(
        verdict
            .get("auditor_runtime_status")
            .and_then(Value::as_str),
        Some("success")
    );
    assert_eq!(
        verdict.get("reasoning_summary").and_then(Value::as_str),
        Some("ambiguous payment and auth risk")
    );
    assert!(verdict.get("prev_hash").is_some());
    assert!(verdict.get("entry_hash").is_some());

    let verify = stdout(&run(target, &["agent", "audit", "verify-log"], 0));
    assert!(verify.contains("VERIFY_LOG=PASS"), "{verify}");
}

#[test]
fn auditor_provider_must_differ_from_primary_provider() {
    let temp = tempfile::tempdir().unwrap();
    let target = temp.path();

    let output = command(target)
        .env("AIPLUS_PRIMARY_PROVIDER", "codex")
        .args([
            "agent",
            "route",
            "--auditor-provider",
            "codex",
            "review ambiguous secure payment plan",
        ])
        .output()
        .expect("run same-provider auditor route");
    assert_eq!(output.status.code(), Some(3));
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("must differ from primary provider `codex`"),
        "{stderr}"
    );
}

#[test]
fn doctor_reports_auditor_provider_env_status() {
    let temp = tempfile::tempdir().unwrap();
    let target = temp.path();

    let output = command(target)
        .env("AIPLUS_AUDITOR_PROVIDER", "opencode")
        .args(["agent", "doctor"])
        .output()
        .expect("run doctor");
    assert_eq!(output.status.code(), Some(0));
    let out = String::from_utf8_lossy(&output.stdout);
    assert!(
        out.contains("INFO auditor_provider_configured=opencode"),
        "{out}"
    );
}
