use serde_json::Value;
use std::fs;
use std::path::Path;
use std::process::Command;

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

fn setup_git_project_with_engineer_worktree_config(root: &Path) {
    fs::create_dir_all(root).unwrap();
    git(root, &["init"]);
    git(root, &["config", "user.email", "test@example.invalid"]);
    git(root, &["config", "user.name", "Test User"]);
    fs::write(root.join("README.md"), "fixture\n").unwrap();
    git(root, &["add", "README.md"]);
    git(root, &["commit", "-m", "init fixture / 初始化 fixture"]);

    fs::create_dir_all(root.join(".aiplus/agents")).unwrap();
    fs::write(
        root.join(".aiplus/agents/engineer-a.toml"),
        r#"
schema_version = "1.0"

[agent]
role = "engineer-a"
display_name = "Engineer A"
tier = "internal"
status = "active"

[workspace]
needs_worktree = true
worktree_branch = "agent/engineer-a"
worktree_path = "../repo.engineer-a"
"#,
    )
    .unwrap();
}

#[test]
fn worktree_pool_creates_then_reuses_role_worktree() {
    let temp = tempfile::tempdir().unwrap();
    let target = temp.path().join("repo");
    setup_git_project_with_engineer_worktree_config(&target);

    let first = command(&target)
        .env("AIPLUS_PERF1_SIDECARS", "reviewer")
        .args([
            "agent",
            "route",
            "engineer-a",
            "Implement PERF-1 worktree pool fixture.",
        ])
        .output()
        .expect("run first batched route");
    assert_eq!(
        first.status.code(),
        Some(0),
        "first route failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&first.stdout),
        String::from_utf8_lossy(&first.stderr)
    );

    let second = command(&target)
        .env("AIPLUS_PERF1_SIDECARS", "reviewer")
        .args([
            "agent",
            "route",
            "engineer-a",
            "Implement PERF-1 worktree pool fixture again.",
        ])
        .output()
        .expect("run second batched route");
    assert_eq!(
        second.status.code(),
        Some(0),
        "second route failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&second.stdout),
        String::from_utf8_lossy(&second.stderr)
    );

    let primary_worktree_statuses = metrics(&target)
        .into_iter()
        .filter(|metric| {
            metric["role"].as_str() == Some("engineer-a")
                && metric["kind"].as_str() == Some("primary")
        })
        .map(|metric| {
            (
                metric["worktree"].as_str().unwrap().to_string(),
                metric["cacheInvalidated"].as_bool(),
            )
        })
        .collect::<Vec<_>>();

    assert_eq!(
        primary_worktree_statuses,
        vec![
            ("created".to_string(), Some(true)),
            ("reused".to_string(), Some(true)),
        ]
    );
}

#[test]
fn sidecar_cache_reuse_is_visible_in_metrics() {
    let temp = tempfile::tempdir().unwrap();
    let target = temp.path();

    let output = command(target)
        .env("AIPLUS_PERF1_SIDECARS", "reviewer,qa")
        .args([
            "agent",
            "route",
            "engineer-a",
            "Verify PERF-1 cache reuse metrics.",
        ])
        .output()
        .expect("run batched route");
    assert_eq!(output.status.code(), Some(0));

    let sidecar_cache_flags = metrics(target)
        .into_iter()
        .filter(|metric| metric["kind"].as_str() == Some("sidecar"))
        .map(|metric| metric["cacheInvalidated"].as_bool())
        .collect::<Vec<_>>();

    assert_eq!(sidecar_cache_flags, vec![Some(false), Some(false)]);
}
