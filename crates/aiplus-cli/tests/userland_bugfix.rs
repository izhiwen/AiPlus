use serde_json::Value;
use std::fs;
use std::path::Path;
use std::process::{Command, Output};

fn bin() -> &'static str {
    env!("CARGO_BIN_EXE_aiplus")
}

fn run(cwd: &Path, args: &[&str], expected: i32) -> Output {
    run_with_env(cwd, args, expected, &[])
}

fn run_with_env(cwd: &Path, args: &[&str], expected: i32, envs: &[(&str, &str)]) -> Output {
    let mut command = Command::new(bin());
    command
        .args(args)
        .current_dir(cwd)
        .env("HOME", cwd.join("fake-home"))
        .env("CODEX_HOME", cwd.join("fake-codex-home"))
        .env("CLAUDE_CONFIG_DIR", cwd.join("fake-claude-home"))
        .env("XDG_CONFIG_HOME", cwd.join("fake-xdg"))
        .env("AIPLUS_SECRET_BROKER_DISABLE_KEYCHAIN", "1")
        .env_remove("ANTHROPIC_API_KEY")
        .env_remove("OPENAI_API_KEY")
        .env_remove("BWS_ACCESS_TOKEN");
    for (key, value) in envs {
        command.env(key, value);
    }
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
fn mcp_register_accepts_claude_code_alias() {
    let temp = tempfile::tempdir().unwrap();
    let target = temp.path();

    let out = stdout(&run(
        target,
        &["mcp-register", "--runtime", "claude-code", "--dry-run"],
        0,
    ));

    assert!(
        out.contains("MCP_REGISTER_CLAUDE=WOULD_WRITE"),
        "claude-code should be accepted as claude alias:\n{out}"
    );
    assert!(
        out.contains("MCP_REGISTER_STATUS=DRY_RUN_OK"),
        "dry-run should complete:\n{out}"
    );
}

#[test]
fn mcp_register_codex_honors_codex_home_and_config_dir_override() {
    let temp = tempfile::tempdir().unwrap();
    let target = temp.path();
    let codex_home = target.join("isolated-codex");
    let override_dir = target.join("explicit-codex");
    fs::create_dir_all(&codex_home).unwrap();
    fs::create_dir_all(&override_dir).unwrap();

    let codex_home_str = codex_home.to_string_lossy().to_string();
    let out = stdout(&run_with_env(
        target,
        &["mcp-register", "--runtime", "codex", "--dry-run"],
        0,
        &[("CODEX_HOME", &codex_home_str)],
    ));
    assert!(
        out.contains(&format!(
            "MCP_REGISTER_CODEX=WOULD_WRITE path={}",
            codex_home.join("config.toml").display()
        )),
        "codex registration should honor CODEX_HOME:\n{out}"
    );

    let out = stdout(&run_with_env(
        target,
        &[
            "mcp-register",
            "--runtime",
            "codex",
            "--config-dir",
            override_dir.to_str().unwrap(),
            "--dry-run",
        ],
        0,
        &[("CODEX_HOME", &codex_home_str)],
    ));
    assert!(
        out.contains(&format!(
            "MCP_REGISTER_CODEX=WOULD_WRITE path={}",
            override_dir.join("config.toml").display()
        )),
        "--config-dir should override CODEX_HOME:\n{out}"
    );
}

#[test]
fn mcp_register_claude_honors_claude_config_dir() {
    let temp = tempfile::tempdir().unwrap();
    let target = temp.path();
    let claude_dir = target.join("isolated-claude");
    fs::create_dir_all(&claude_dir).unwrap();
    let claude_dir_str = claude_dir.to_string_lossy().to_string();

    let out = stdout(&run_with_env(
        target,
        &[
            "mcp-register",
            "--runtime",
            "claude-code",
            "--scope",
            "global",
            "--dry-run",
        ],
        0,
        &[("CLAUDE_CONFIG_DIR", &claude_dir_str)],
    ));

    assert!(
        out.contains(&format!(
            "MCP_REGISTER_CLAUDE=WOULD_WRITE path={}",
            claude_dir.join(".mcp.json").display()
        )),
        "claude-code global registration should honor CLAUDE_CONFIG_DIR:\n{out}"
    );
}

#[test]
fn top_level_doctor_quiet_suppresses_info_and_keeps_status() {
    let temp = tempfile::tempdir().unwrap();
    let target = temp.path();

    let help = stdout(&run(target, &["doctor", "--help"], 0));
    assert!(
        help.contains("--quiet"),
        "doctor help should list --quiet:\n{help}"
    );

    let out = stdout(&run(target, &["doctor", "--quiet"], 0));
    assert!(
        out.contains("DOCTOR_STATUS="),
        "quiet doctor should keep final status:\n{out}"
    );
    assert!(
        out.lines().all(|line| !line.starts_with("INFO ")),
        "quiet doctor should suppress INFO lines:\n{out}"
    );
}

#[test]
fn score_only_autosummon_uses_openai_key_and_reports_missing_key_warning() {
    let temp = tempfile::tempdir().unwrap();
    let target = temp.path();
    fs::write(target.join("README.md"), "# Userland autosummon\n").unwrap();
    init_git_repo(target);
    run(target, &["install", "codex"], 0);

    let no_key_out = stdout(&run(
        target,
        &["agent", "route", "--score-only", "实现支付接口"],
        0,
    ));
    assert!(
        no_key_out.contains("Autosummon intent warning: intent_classifier skipped"),
        "missing provider key should be visible, not silently swallowed:\n{no_key_out}"
    );

    let response_path = target.join("openai-yes.json");
    fs::write(
        &response_path,
        r#"{"choices":[{"message":{"content":"YES"}}]}"#,
    )
    .unwrap();
    let url = format!("file://{}", response_path.display());
    let out = stdout(&run_with_env(
        target,
        &["agent", "route", "--score-only", "实现支付接口"],
        0,
        &[
            ("OPENAI_API_KEY", "test-openai-key"),
            ("AIPLUS_AUTOSUMMON_INTENT_URL", &url),
        ],
    ));

    assert!(
        out.contains("Auto-summoned experts: [security-reviewer"),
        "OpenAI-backed classifier should auto-summon security reviewer:\n{out}"
    );

    let body = fs::read_to_string(target.join(".aiplus/agents/dispatch-log.jsonl")).unwrap();
    let latest_decision: Value = body
        .lines()
        .rev()
        .map(|line| serde_json::from_str(line).expect("jsonl line"))
        .find(|entry: &Value| {
            entry.get("event").and_then(Value::as_str) == Some("coordinator_decision")
        })
        .expect("coordinator decision");
    assert_eq!(
        latest_decision
            .get("intent_classifier_status")
            .and_then(Value::as_str),
        Some("ok")
    );
    let auto_summoned = latest_decision
        .get("auto_summoned")
        .and_then(Value::as_array)
        .expect("auto_summoned array");
    assert!(
        auto_summoned
            .iter()
            .any(|role| role.as_str() == Some("security-reviewer")),
        "dispatch log should include security-reviewer auto-summon:\n{body}"
    );
}
