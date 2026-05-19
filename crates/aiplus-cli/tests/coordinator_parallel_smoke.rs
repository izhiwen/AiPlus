use serde_json::Value;
use std::fs;
use std::path::Path;
use std::process::{Command, Output};
use std::time::{Duration, Instant};

const HEAVY_TASK: &str = "implement payment billing refund support";
const HEAVY_ROLES: [&str; 6] = [
    "architect",
    "engineer-a",
    "engineer-b",
    "pm",
    "qa",
    "reviewer",
];

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

fn metrics(target: &Path) -> Vec<Value> {
    let path = target.join(".aiplus/agents/dispatch-metrics.jsonl");
    fs::read_to_string(path)
        .expect("dispatch metrics exist")
        .lines()
        .map(|line| serde_json::from_str::<Value>(line).expect("metric JSON"))
        .collect()
}

fn metric_roles(metrics: &[Value]) -> Vec<String> {
    let mut roles = metrics
        .iter()
        .map(|metric| metric["role"].as_str().unwrap().to_string())
        .collect::<Vec<_>>();
    roles.sort();
    roles
}

fn expected_roles() -> Vec<String> {
    HEAVY_ROLES.iter().map(|role| (*role).to_string()).collect()
}

fn stdout(output: &Output) -> String {
    String::from_utf8_lossy(&output.stdout).to_string()
}

fn init_git_repo(target: &Path) {
    fs::create_dir_all(target).unwrap();
    git(target, &["init"]);
    git(target, &["config", "user.email", "test@example.invalid"]);
    git(target, &["config", "user.name", "Test User"]);
    fs::write(target.join("README.md"), "coordinator parallel fixture\n").unwrap();
    git(target, &["add", "README.md"]);
    git(target, &["commit", "-m", "init fixture / 初始化 fixture"]);
}

fn git(cwd: &Path, args: &[&str]) {
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
}

fn write_worktree_role_configs(target: &Path) {
    let agents = target.join(".aiplus/agents");
    fs::create_dir_all(&agents).unwrap();
    for role in HEAVY_ROLES {
        fs::write(
            agents.join(format!("{role}.toml")),
            format!(
                r#"
schema_version = "1.0"

[agent]
role = "{role}"
display_name = "{role}"
tier = "internal"
status = "active"

[workspace]
needs_worktree = true
worktree_branch = "agent/{role}"
worktree_path = "../repo.{role}"
"#
            ),
        )
        .unwrap();
    }
}

#[test]
fn heavy_adaptive_route_fans_out_all_staffed_roles_in_one_batch() {
    let temp = tempfile::tempdir().unwrap();
    let target = temp.path();

    let output = run(target, &["agent", "route", HEAVY_TASK], 0);
    let out = stdout(&output);
    assert!(
        out.contains("Coordinator batch coord-"),
        "adaptive route should use coordinator batch:\n{out}"
    );

    let metrics = metrics(target);
    assert_eq!(metrics.len(), 6, "expected six coordinator peer metrics");
    assert_eq!(metric_roles(&metrics), expected_roles());
    let batch_id = metrics[0]["batchId"].as_str().unwrap().to_string();
    for metric in metrics {
        assert_eq!(metric["batchId"].as_str(), Some(batch_id.as_str()));
        assert_eq!(metric["kind"].as_str(), Some("coordinator_peer"));
        assert_eq!(metric["outcome"].as_str(), Some("success"));
        assert_eq!(metric["cacheInvalidated"].as_bool(), Some(true));
    }
}

#[test]
fn coordinator_batch_collects_partial_failure_after_siblings_finish() {
    let temp = tempfile::tempdir().unwrap();
    let target = temp.path();

    let output = command(target)
        .env("AIPLUS_PERF1_FAIL_ROLE", "reviewer")
        .args(["agent", "route", HEAVY_TASK])
        .output()
        .expect("run partial failure route");
    assert!(
        !output.status.success(),
        "partial failure should return non-zero\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let out = String::from_utf8_lossy(&output.stdout);
    assert!(
        out.contains("completed with partial failure"),
        "stdout should report partial failure after join:\n{out}"
    );

    let metrics = metrics(target);
    assert_eq!(metrics.len(), 6, "all workers should record metrics");
    assert_eq!(metric_roles(&metrics), expected_roles());
    let successes = metrics
        .iter()
        .filter(|metric| metric["outcome"].as_str() == Some("success"))
        .count();
    let failures = metrics
        .iter()
        .filter(|metric| metric["outcome"].as_str() == Some("fail"))
        .count();
    assert_eq!(successes, 5);
    assert_eq!(failures, 1);
    assert!(
        metrics.iter().any(|metric| {
            metric["role"].as_str() == Some("reviewer")
                && metric["worktree"].as_str() == Some("fixture_failed")
        }),
        "reviewer failure should be visible in metrics: {metrics:?}"
    );
}

#[test]
fn coordinator_batch_is_parallel_by_wall_clock_delay_fixture() {
    let temp = tempfile::tempdir().unwrap();
    let target = temp.path();
    let delay = Duration::from_millis(900);
    let serial_estimate = delay * HEAVY_ROLES.len() as u32;

    let started = Instant::now();
    let output = command(target)
        .env("AIPLUS_PERF1_DELAY_PM_MS", delay.as_millis().to_string())
        .env(
            "AIPLUS_PERF1_DELAY_ARCHITECT_MS",
            delay.as_millis().to_string(),
        )
        .env(
            "AIPLUS_PERF1_DELAY_ENGINEER_A_MS",
            delay.as_millis().to_string(),
        )
        .env(
            "AIPLUS_PERF1_DELAY_ENGINEER_B_MS",
            delay.as_millis().to_string(),
        )
        .env(
            "AIPLUS_PERF1_DELAY_REVIEWER_MS",
            delay.as_millis().to_string(),
        )
        .env("AIPLUS_PERF1_DELAY_QA_MS", delay.as_millis().to_string())
        .args(["agent", "route", HEAVY_TASK])
        .output()
        .expect("run delayed heavy route");
    let elapsed = started.elapsed();

    assert_eq!(
        output.status.code(),
        Some(0),
        "delayed route failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        elapsed < serial_estimate / 2,
        "parallel dispatch should be at least 2x faster than serial estimate; elapsed={elapsed:?} serial_estimate={serial_estimate:?}"
    );
    assert!(
        elapsed <= delay + Duration::from_millis(1200),
        "parallel dispatch should stay close to the slowest worker; elapsed={elapsed:?} single_delay={delay:?}"
    );
}

#[test]
fn coordinator_batch_handles_six_way_worktree_pool_contention() {
    let temp = tempfile::tempdir().unwrap();
    let target = temp.path().join("repo");
    init_git_repo(&target);
    write_worktree_role_configs(&target);

    run(&target, &["agent", "route", HEAVY_TASK], 0);

    let metrics = metrics(&target);
    assert_eq!(metrics.len(), 6, "expected six coordinator peer metrics");
    assert_eq!(metric_roles(&metrics), expected_roles());
    for metric in metrics {
        assert_eq!(metric["kind"].as_str(), Some("coordinator_peer"));
        assert_eq!(metric["outcome"].as_str(), Some("success"));
        assert!(
            matches!(metric["worktree"].as_str(), Some("created" | "reused")),
            "worktree status should be created/reused: {metric:?}"
        );
    }
}
