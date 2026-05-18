use std::fs;
use std::path::Path;
use std::process::{Command, Output};

fn bin() -> &'static str {
    env!("CARGO_BIN_EXE_aiplus")
}

fn run(cwd: &Path, args: &[&str], extra_env: &[(&str, &str)]) -> Output {
    let mut cmd = Command::new(bin());
    cmd.args(args)
        .current_dir(cwd)
        .env("HOME", cwd.join("fake-home"))
        .env("CODEX_HOME", cwd.join("fake-codex-home"))
        .env("XDG_CONFIG_HOME", cwd.join("fake-xdg"))
        .env("AIPLUS_SECRET_BROKER_DISABLE_KEYCHAIN", "1")
        .env_remove("ANTHROPIC_API_KEY")
        .env_remove("OPENAI_API_KEY")
        .env_remove("BWS_ACCESS_TOKEN");
    for (key, value) in extra_env {
        cmd.env(key, value);
    }
    let output = cmd.output().expect("run aiplus");
    assert!(
        output.status.success(),
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

fn write_active_agent(target: &Path) {
    let agents_dir = target.join(".aiplus/agents");
    fs::create_dir_all(&agents_dir).unwrap();
    fs::write(
        agents_dir.join("engineer-a.toml"),
        r#"
[agent]
role = "engineer-a"
display_name = "Engineer A"
tier = "builder"
status = "active"

[workspace]
needs_worktree = false
"#,
    )
    .unwrap();
}

#[test]
fn doctor_warns_when_bws_active_roles_have_no_runtime_provider_key() {
    let temp = tempfile::tempdir().unwrap();
    let target = temp.path();
    write_active_agent(target);

    let out = stdout(&run(
        target,
        &["agent", "doctor"],
        &[("AIPLUS_SECRET_PROVIDER", "bws")],
    ));
    assert!(
        out.contains("WARN_SECRET_BROKER_RUNTIME_AUTH"),
        "doctor should warn about BWS runtime auth cliff:\n{out}"
    );
    assert!(
        out.contains("aiplus secret-broker run --aliases anthropic,openai -- aiplus agent route"),
        "doctor warning should include copy-pasteable wrapper:\n{out}"
    );
}

#[test]
fn doctor_suppresses_bws_runtime_auth_warning_when_provider_key_present() {
    let temp = tempfile::tempdir().unwrap();
    let target = temp.path();
    write_active_agent(target);

    let out = stdout(&run(
        target,
        &["agent", "doctor"],
        &[
            ("AIPLUS_SECRET_PROVIDER", "bws"),
            ("OPENAI_API_KEY", "test-key-present"),
        ],
    ));
    assert!(
        !out.contains("WARN_SECRET_BROKER_RUNTIME_AUTH"),
        "provider env key should suppress warning:\n{out}"
    );
}
