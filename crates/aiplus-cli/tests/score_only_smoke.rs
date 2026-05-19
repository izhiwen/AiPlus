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
fn score_only_prints_plan_and_logs_coordinator_decision_without_dispatch() {
    let temp = tempfile::tempdir().unwrap();
    let target = temp.path();
    fs::write(target.join("README.md"), "# Score Only\n").unwrap();
    init_git_repo(target);

    let task = "describe the current git status in one sentence";
    let out = stdout(&run(target, &["agent", "route", "--score-only", task], 0));
    assert!(
        out.contains("Adaptive coordinator: complexity=1 risk=0.15 tier=LIGHT_NO_CODE"),
        "score-only should print coordinator score:\n{out}"
    );
    assert!(
        out.contains("Plan step: consultant would be skipped for LIGHT_NO_CODE"),
        "score-only should print dry-run consultant plan:\n{out}"
    );
    assert!(
        out.contains("Would staff: []"),
        "score-only should print dry-run staffing:\n{out}"
    );

    let log_path = target.join(".aiplus/agents/dispatch-log.jsonl");
    let body = fs::read_to_string(&log_path).expect("dispatch log should exist");
    let entries: Vec<Value> = body
        .lines()
        .map(|line| serde_json::from_str(line).expect("jsonl line"))
        .collect();
    let decisions: Vec<&Value> = entries
        .iter()
        .filter(|entry| entry.get("event").and_then(Value::as_str) == Some("coordinator_decision"))
        .collect();
    assert_eq!(
        decisions.len(),
        1,
        "expected one coordinator decision:\n{body}"
    );
    let decision = decisions[0];
    assert_eq!(
        decision.get("mode").and_then(Value::as_str),
        Some("score_only")
    );
    assert_eq!(
        decision.get("dispatched").and_then(Value::as_bool),
        Some(false)
    );
    assert_eq!(
        decision.get("taskExcerpt").and_then(Value::as_str),
        Some(task)
    );
    assert!(
        entries
            .iter()
            .all(|entry| entry.get("role").and_then(Value::as_str).is_none()),
        "score-only must not append role dispatch rows:\n{body}"
    );
}

#[test]
fn light_no_code_route_logs_coordinator_decision_for_no_dispatch_path() {
    let temp = tempfile::tempdir().unwrap();
    let target = temp.path();
    fs::write(target.join("README.md"), "# Route Decision\n").unwrap();
    init_git_repo(target);

    let task = "describe the current git status in one sentence";
    let out = stdout(&run(target, &["agent", "route", task], 0));
    assert!(
        out.contains("Execute step: CEO handles directly; no worktree staffing required."),
        "LIGHT_NO_CODE route should not staff roles:\n{out}"
    );

    let log_path = target.join(".aiplus/agents/dispatch-log.jsonl");
    let body = fs::read_to_string(&log_path).expect("dispatch log should exist");
    let entries: Vec<Value> = body
        .lines()
        .map(|line| serde_json::from_str(line).expect("jsonl line"))
        .collect();
    let decisions: Vec<&Value> = entries
        .iter()
        .filter(|entry| entry.get("event").and_then(Value::as_str) == Some("coordinator_decision"))
        .collect();
    assert_eq!(
        decisions.len(),
        1,
        "expected one coordinator decision:\n{body}"
    );
    assert_eq!(
        decisions[0].get("mode").and_then(Value::as_str),
        Some("route")
    );
    assert_eq!(
        decisions[0].get("dispatched").and_then(Value::as_bool),
        Some(false)
    );
    assert!(
        entries.iter().any(|entry| {
            entry.get("role").and_then(Value::as_str) == Some("ceo")
                && entry.get("outcome").and_then(Value::as_str) == Some("success")
        }),
        "normal LIGHT_NO_CODE route should still record CEO handling:\n{body}"
    );
}

#[test]
fn score_only_prints_auto_summoned_experts_without_dispatch() {
    let temp = tempfile::tempdir().unwrap();
    let target = temp.path();
    fs::write(target.join("README.md"), "# Score Only Autosummon\n").unwrap();
    init_git_repo(target);
    run(target, &["install", "codex"], 0);

    let task = "write secure payment API docs";
    let out = stdout(&run(target, &["agent", "route", "--score-only", task], 0));
    assert!(
        out.contains("Would staff: [pm,architect,engineer-a,engineer-b,reviewer,qa,security-reviewer,tech-writer]"),
        "score-only should include autosummoned experts:\n{out}"
    );
    assert!(
        out.contains("Auto-summoned experts: [security-reviewer,tech-writer]"),
        "score-only should report autosummoned experts:\n{out}"
    );

    let log_path = target.join(".aiplus/agents/dispatch-log.jsonl");
    let body = fs::read_to_string(&log_path).expect("dispatch log should exist");
    let decision: Value = body
        .lines()
        .map(|line| serde_json::from_str(line).expect("jsonl line"))
        .find(|entry: &Value| {
            entry.get("event").and_then(Value::as_str) == Some("coordinator_decision")
        })
        .expect("coordinator decision");
    assert_eq!(
        decision.get("auto_summoned").and_then(Value::as_array),
        Some(&vec![
            Value::String("security-reviewer".to_string()),
            Value::String("tech-writer".to_string())
        ])
    );
    assert!(body.lines().all(|line| {
        let entry: Value = serde_json::from_str(line).expect("jsonl line");
        entry.get("role").and_then(Value::as_str).is_none()
    }));
}
