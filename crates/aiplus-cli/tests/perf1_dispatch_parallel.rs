use serde_json::Value;
use std::fs;
use std::path::Path;
use std::process::Command;
use std::thread;
use std::time::{Duration, Instant};

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
        .env_remove("BWS_ACCESS_TOKEN");
    cmd
}

fn metrics(target: &Path) -> Vec<Value> {
    let path = target.join(".aiplus/agents/dispatch-metrics.jsonl");
    if !path.exists() {
        return Vec::new();
    }
    fs::read_to_string(path)
        .unwrap()
        .lines()
        .map(|line| serde_json::from_str::<Value>(line).expect("metric JSON"))
        .collect()
}

fn dispatch_log(target: &Path) -> Vec<Value> {
    let path = target.join(".aiplus/agents/dispatch-log.jsonl");
    if !path.exists() {
        return Vec::new();
    }
    fs::read_to_string(path)
        .unwrap()
        .lines()
        .map(|line| serde_json::from_str::<Value>(line).expect("dispatch log JSON"))
        .collect()
}

fn setup_gate_fixture(target: &Path) {
    fs::create_dir_all(target.join(".aiplus")).unwrap();
    fs::write(
        target.join(".aiplus/consultant-team.toml"),
        r#"
schema_version = "0.1"

[[members]]
id = "release_automation"
name = "Release / Automation"
default_tiers = ["LIGHT", "MEDIUM", "HEAVY"]
triggers = ["release", "publish", "deploy", "push"]

[[triggers]]
id = "release"
patterns = ["release", "publish", "deploy", "push"]
tier = "HEAVY"
members = ["release_automation"]
stop_gate = true

[owner_gates]
push = true
release = true
package_publish = true
deploy = true
"#,
    )
    .unwrap();
}

#[test]
fn t1_builder_reviewer_qa_share_one_dispatch_batch() {
    let temp = tempfile::tempdir().unwrap();
    let target = temp.path();

    let output = command(target)
        .env("AIPLUS_PERF1_SIDECARS", "reviewer,qa")
        .args([
            "agent",
            "route",
            "engineer-a",
            "Implement local fixture batching for PERF-1.",
        ])
        .output()
        .expect("run batched route");
    assert_eq!(
        output.status.code(),
        Some(0),
        "batched route failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    let metrics = metrics(target);
    assert_eq!(metrics.len(), 3, "expected primary + two sidecar metrics");
    let batch_id = metrics[0]["batchId"].as_str().unwrap().to_string();
    let mut roles = metrics
        .iter()
        .map(|metric| {
            assert_eq!(metric["batchId"].as_str(), Some(batch_id.as_str()));
            assert_eq!(metric["outcome"].as_str(), Some("success"));
            metric["role"].as_str().unwrap().to_string()
        })
        .collect::<Vec<_>>();
    roles.sort();
    assert_eq!(roles, ["engineer-a", "qa", "reviewer"]);
}

#[test]
fn t8_owner_gate_blocks_sidecar_batch_metrics() {
    let temp = tempfile::tempdir().unwrap();
    let target = temp.path();
    setup_gate_fixture(target);

    let output = command(target)
        .env("AIPLUS_PERF1_SIDECARS", "reviewer,qa")
        .args([
            "agent",
            "route",
            "engineer-a",
            "Please publish and deploy the release package.",
        ])
        .output()
        .expect("run gated route");
    assert!(
        !output.status.success(),
        "gated route should exit non-zero\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    assert!(
        metrics(target).is_empty(),
        "gate-blocked route must not start batch sidecars"
    );
    let roles = dispatch_log(target)
        .into_iter()
        .filter_map(|entry| entry["role"].as_str().map(ToString::to_string))
        .collect::<Vec<_>>();
    assert!(
        !roles
            .iter()
            .any(|role| matches!(role.as_str(), "reviewer" | "qa")),
        "gate-blocked route wrote sidecar dispatch log entries: {roles:?}"
    );
}

#[test]
fn t9_slow_qa_does_not_block_reviewer_dispatch_recording() {
    let temp = tempfile::tempdir().unwrap();
    let target = temp.path();
    let mut child = command(target)
        .env("AIPLUS_PERF1_SIDECARS", "reviewer,qa")
        .env("AIPLUS_PERF1_DELAY_QA_MS", "2000")
        .args([
            "agent",
            "route",
            "engineer-a",
            "Implement PERF-1 sidecar scheduling smoke fixture.",
        ])
        .spawn()
        .expect("spawn batched route");

    let deadline = Instant::now() + Duration::from_millis(1500);
    let mut saw_reviewer = false;
    while Instant::now() < deadline {
        saw_reviewer = metrics(target)
            .iter()
            .any(|metric| metric["role"].as_str() == Some("reviewer"));
        if saw_reviewer {
            break;
        }
        thread::sleep(Duration::from_millis(50));
    }

    assert!(
        saw_reviewer,
        "reviewer metric should be recorded while QA sidecar is still delayed"
    );
    assert!(
        child.try_wait().unwrap().is_none(),
        "child should still be waiting on delayed QA after reviewer records"
    );

    let status = child.wait().expect("wait for batched route");
    assert_eq!(status.code(), Some(0));
    assert!(
        metrics(target)
            .iter()
            .any(|metric| metric["role"].as_str() == Some("qa")),
        "QA metric should be recorded after the delayed sidecar finishes"
    );
}
