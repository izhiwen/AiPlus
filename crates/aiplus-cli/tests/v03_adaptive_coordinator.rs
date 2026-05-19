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

fn git_commit_all(target: &Path, message: &str) {
    Command::new("git")
        .args(["add", "."])
        .current_dir(target)
        .output()
        .expect("git add");
    Command::new("git")
        .args(["commit", "--no-verify", "-m", message])
        .current_dir(target)
        .output()
        .expect("git commit");
}

fn dispatch_log_roles(target: &Path) -> Vec<String> {
    let path = target.join(".aiplus/agents/dispatch-log.jsonl");
    fs::read_to_string(path)
        .expect("dispatch log exists")
        .lines()
        .map(|line| serde_json::from_str::<Value>(line).expect("dispatch JSON"))
        .filter_map(|value| {
            value
                .get("role")
                .and_then(|role| role.as_str())
                .map(ToString::to_string)
        })
        .collect()
}

fn dispatch_metric_roles(target: &Path) -> Vec<String> {
    let path = target.join(".aiplus/agents/dispatch-metrics.jsonl");
    fs::read_to_string(path)
        .expect("dispatch metrics exist")
        .lines()
        .map(|line| serde_json::from_str::<Value>(line).expect("metric JSON"))
        .filter_map(|value| {
            value
                .get("role")
                .and_then(|role| role.as_str())
                .map(ToString::to_string)
        })
        .collect()
}

fn coordinator_decisions(target: &Path) -> Vec<Value> {
    let path = target.join(".aiplus/agents/dispatch-log.jsonl");
    fs::read_to_string(path)
        .expect("dispatch log exists")
        .lines()
        .map(|line| serde_json::from_str::<Value>(line).expect("dispatch JSON"))
        .filter(|value| value.get("event").and_then(Value::as_str) == Some("coordinator_decision"))
        .collect()
}

#[test]
fn d5_payment_task_staffs_heavy_team() {
    let temp = tempfile::tempdir().unwrap();
    let target = temp.path();
    fs::write(target.join("README.md"), "# Coordinator Fixture\n").unwrap();
    init_git_repo(target);
    git_commit_all(target, "initial");

    run(target, &["install", "codex"], 0);
    git_commit_all(target, "install aiplus");

    let route = stdout(&run(target, &["agent", "route", "实现支付接口"], 0));
    assert!(
        route.contains("Adaptive coordinator: complexity=5 risk=0.85 tier=HEAVY"),
        "route should score payment task as HEAVY:\n{route}"
    );
    assert!(
        route.contains("Plan step: firing consultant for HEAVY task"),
        "HEAVY tasks must fire consultant:\n{route}"
    );
    assert!(
        route.contains(
            "Staffing roles: [pm,architect,engineer-a,engineer-b,reviewer,qa,security-reviewer]"
        ),
        "HEAVY staffing mismatch:\n{route}"
    );
    assert!(
        route.contains("Forced by risk: [reviewer,qa]"),
        "risk-forced roles should be visible:\n{route}"
    );
    assert!(
        route.contains("Auto-summoned experts: [security-reviewer]"),
        "payment task should auto-summon security reviewer:\n{route}"
    );
    for role in [
        "pm",
        "architect",
        "engineer-a",
        "engineer-b",
        "reviewer",
        "qa",
        "security-reviewer",
    ] {
        assert!(
            route.contains(&format!("Routing task to {role}:")),
            "missing staffed role {role}:\n{route}"
        );
    }

    let mut roles = dispatch_log_roles(target);
    roles.sort();
    assert_eq!(
        roles,
        [
            "architect",
            "engineer-a",
            "engineer-b",
            "pm",
            "qa",
            "reviewer",
            "security-reviewer"
        ]
    );

    let mut metric_roles = dispatch_metric_roles(target);
    metric_roles.sort();
    assert_eq!(
        metric_roles,
        [
            "architect",
            "engineer-a",
            "engineer-b",
            "pm",
            "qa",
            "reviewer",
            "security-reviewer"
        ]
    );

    let doctor = stdout(&run(target, &["agent", "doctor"], 0));
    assert!(
        doctor.contains("PASS coordinator scoring config valid"),
        "agent doctor should validate coordinator scoring:\n{doctor}"
    );
    assert!(
        doctor.contains("PASS coordinator tier thresholds match DESIGN.md §9.2"),
        "agent doctor should validate coordinator thresholds:\n{doctor}"
    );

    let decisions = coordinator_decisions(target);
    assert_eq!(decisions.len(), 1, "expected one coordinator decision");
    assert_eq!(
        decisions[0].get("forced_by_risk").and_then(Value::as_array),
        Some(&vec![
            Value::String("reviewer".to_string()),
            Value::String("qa".to_string())
        ])
    );
    assert_eq!(
        decisions[0].get("ttl_expired").and_then(Value::as_bool),
        Some(false)
    );
    assert_eq!(
        decisions[0].get("auto_summoned").and_then(Value::as_array),
        Some(&vec![Value::String("security-reviewer".to_string())])
    );
}
