use std::fs;
use std::path::Path;
use std::process::{Command, Output};

fn bin() -> &'static str {
    env!("CARGO_BIN_EXE_aiplus")
}

fn run(cwd: &Path, args: &[&str]) -> Output {
    let output = Command::new(bin())
        .args(args)
        .current_dir(cwd)
        .env("HOME", cwd.join("fake-home"))
        .env("CODEX_HOME", cwd.join("fake-codex-home"))
        .env("XDG_CONFIG_HOME", cwd.join("fake-xdg"))
        .env("AIPLUS_SECRET_BROKER_DISABLE_KEYCHAIN", "1")
        .env("AIPLUS_AUTOSUMMON_INTENT_MOCK", "1")
        .env_remove("ANTHROPIC_API_KEY")
        .env_remove("OPENAI_API_KEY")
        .env_remove("BWS_ACCESS_TOKEN")
        .output()
        .expect("run aiplus");
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
fn score_only_auto_summons_experts_by_intent() {
    let temp = tempfile::tempdir().unwrap();
    let target = temp.path();
    fs::write(target.join("README.md"), "# Autosummon Intent\n").unwrap();
    init_git_repo(target);
    run(target, &["install", "codex"]);

    let cases = [
        ("实现支付接口", "[security-reviewer]"),
        ("update README onboarding guide", "[tech-writer]"),
        ("implement AI agent prompt routing", "[ai-integration]"),
        ("describe git status", "[]"),
    ];

    for (task, expected) in cases {
        let out = stdout(&run(target, &["agent", "route", "--score-only", task]));
        if expected == "[]" {
            assert!(
                !out.contains("Auto-summoned experts:"),
                "negative task should not auto-summon experts:\n{out}"
            );
        } else {
            assert!(
                out.contains(&format!("Auto-summoned experts: {expected}")),
                "task {task:?} should auto-summon {expected}:\n{out}"
            );
        }
    }
}
