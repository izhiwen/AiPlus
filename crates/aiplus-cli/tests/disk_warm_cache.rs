use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

fn bin() -> &'static str {
    env!("CARGO_BIN_EXE_aiplus")
}

fn command(cwd: &Path, cache_root: &Path) -> Command {
    let mut cmd = Command::new(bin());
    cmd.current_dir(cwd)
        .env("HOME", cwd.join("fake-home"))
        .env("CODEX_HOME", cwd.join("fake-codex-home"))
        .env("XDG_CONFIG_HOME", cwd.join("fake-xdg"))
        .env("AIPLUS_AGENT_CACHE_ROOT", cache_root)
        .env("AIPLUS_SECRET_BROKER_DISABLE_KEYCHAIN", "1")
        .env_remove("BWS_ACCESS_TOKEN");
    cmd
}

fn run(cwd: &Path, cache_root: &Path, args: &[&str], expected: i32) -> Output {
    let output = command(cwd, cache_root)
        .args(args)
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

fn prepare_project(root: &Path) {
    fs::create_dir_all(root.join(".aiplus/agents/personas")).unwrap();
    fs::create_dir_all(root.join(".aiplus/agent-memory/engineer-a")).unwrap();
    fs::write(
        root.join(".aiplus/agents/engineer-a.toml"),
        r#"
schema_version = "1.0"

[agent]
role = "engineer-a"
display_name = "Engineer A"
tier = "internal"
status = "active"
warm_bench_ttl_seconds = 1800

[persona]
system_prompt_file = "personas/engineer-a.md"

[workspace]
needs_worktree = false
"#,
    )
    .unwrap();
    fs::write(
        root.join(".aiplus/agents/personas/engineer-a.md"),
        "You are Engineer A for tests.\n",
    )
    .unwrap();
}

fn project_cache_dir(cache_root: &Path, project: &Path) -> PathBuf {
    cache_root.join(project.file_name().unwrap())
}

fn set_enforce_ttl(project: &Path, enabled: bool) {
    fs::write(
        project.join(".aiplus/agent-team.toml"),
        format!("[cache]\nenable_disk = true\nenforce_ttl = {enabled}\n"),
    )
    .unwrap();
}

fn set_last_used_at_ms(cache_root: &Path, project: &Path, role: &str, value: u128) {
    let path = project_cache_dir(cache_root, project).join("_cache_meta.json");
    let mut meta: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(&path).unwrap()).unwrap();
    meta["roles"][role]["lastUsedAtMs"] = serde_json::json!(value);
    fs::write(path, serde_json::to_string_pretty(&meta).unwrap()).unwrap();
}

#[test]
fn enabled_disk_cache_reports_disk_warm_on_second_route() {
    let temp = tempfile::tempdir().unwrap();
    let project = temp.path().join("repo-ac1");
    let cache_root = temp.path().join("cache-root");
    prepare_project(&project);

    run(
        &project,
        &cache_root,
        &["agent", "cache", "--enable-disk"],
        0,
    );
    run(
        &project,
        &cache_root,
        &["agent", "route", "engineer-a", "first cache warm"],
        0,
    );
    let second = run(
        &project,
        &cache_root,
        &["agent", "route", "engineer-a", "second cache warm"],
        0,
    );
    assert!(stdout(&second).contains("cache_source=disk_warm"));

    let status = run(&project, &cache_root, &["agent", "status", "--verbose"], 0);
    assert!(stdout(&status).contains("role=engineer-a cache_source=disk_warm"));
}

#[test]
fn ttl_enforcement_cold_starts_expired_cache_when_enabled() {
    let temp = tempfile::tempdir().unwrap();
    let project = temp.path().join("repo-ttl-expired");
    let cache_root = temp.path().join("cache-root");
    prepare_project(&project);

    run(
        &project,
        &cache_root,
        &["agent", "cache", "--enable-disk"],
        0,
    );
    set_enforce_ttl(&project, true);
    run(
        &project,
        &cache_root,
        &["agent", "route", "engineer-a", "write ttl cache"],
        0,
    );
    set_last_used_at_ms(&cache_root, &project, "engineer-a", 0);

    let routed = run(
        &project,
        &cache_root,
        &["agent", "route", "engineer-a", "expired ttl"],
        0,
    );
    assert!(stdout(&routed).contains("cache_source=cold_start"));

    let doctor = run(&project, &cache_root, &["agent", "doctor"], 0);
    assert!(stdout(&doctor).contains("enforce_ttl=true"));
    assert!(stdout(&doctor).contains("INFO cache_age role=engineer-a"));
}

#[test]
fn ttl_enforcement_keeps_fresh_cache_warm_when_enabled() {
    let temp = tempfile::tempdir().unwrap();
    let project = temp.path().join("repo-ttl-fresh");
    let cache_root = temp.path().join("cache-root");
    prepare_project(&project);

    run(
        &project,
        &cache_root,
        &["agent", "cache", "--enable-disk"],
        0,
    );
    set_enforce_ttl(&project, true);
    run(
        &project,
        &cache_root,
        &["agent", "route", "engineer-a", "write fresh ttl cache"],
        0,
    );

    let routed = run(
        &project,
        &cache_root,
        &["agent", "route", "engineer-a", "fresh ttl"],
        0,
    );
    assert!(stdout(&routed).contains("cache_source=disk_warm"));
}

#[test]
fn stale_ttl_is_ignored_when_enforcement_disabled() {
    let temp = tempfile::tempdir().unwrap();
    let project = temp.path().join("repo-ttl-disabled");
    let cache_root = temp.path().join("cache-root");
    prepare_project(&project);

    run(
        &project,
        &cache_root,
        &["agent", "cache", "--enable-disk"],
        0,
    );
    run(
        &project,
        &cache_root,
        &["agent", "route", "engineer-a", "write ignored ttl cache"],
        0,
    );
    set_last_used_at_ms(&cache_root, &project, "engineer-a", 0);

    let routed = run(
        &project,
        &cache_root,
        &["agent", "route", "engineer-a", "ignored stale ttl"],
        0,
    );
    assert!(stdout(&routed).contains("cache_source=disk_warm"));
}

#[test]
fn disabled_disk_cache_creates_no_cache_directory() {
    let temp = tempfile::tempdir().unwrap();
    let project = temp.path().join("repo-ac2");
    let cache_root = temp.path().join("cache-root");
    prepare_project(&project);

    run(
        &project,
        &cache_root,
        &["agent", "route", "engineer-a", "cache disabled"],
        0,
    );
    assert!(
        !cache_root.exists(),
        "disabled disk cache must not create cache root"
    );
}

#[test]
fn corrupt_cbor_cold_starts_and_doctor_warns() {
    let temp = tempfile::tempdir().unwrap();
    let project = temp.path().join("repo-ac3");
    let cache_root = temp.path().join("cache-root");
    prepare_project(&project);

    run(
        &project,
        &cache_root,
        &["agent", "cache", "--enable-disk"],
        0,
    );
    run(
        &project,
        &cache_root,
        &["agent", "route", "engineer-a", "write cache"],
        0,
    );
    let cbor = project_cache_dir(&cache_root, &project).join("engineer-a.cbor");
    fs::write(&cbor, b"not valid cbor").unwrap();

    let routed = run(
        &project,
        &cache_root,
        &["agent", "route", "engineer-a", "corrupt cache recovery"],
        0,
    );
    assert!(stdout(&routed).contains("cache_source=cold_start"));

    let doctor = run(&project, &cache_root, &["agent", "doctor"], 0);
    assert!(stdout(&doctor).contains("disk_cache_warning"));
    assert!(stdout(&doctor).contains("checksum_mismatch") || stdout(&doctor).contains("corrupt"));
}

#[test]
fn cache_snapshot_redacts_secret_like_memory() {
    let temp = tempfile::tempdir().unwrap();
    let project = temp.path().join("repo-ac4");
    let cache_root = temp.path().join("cache-root");
    prepare_project(&project);
    fs::write(
        project.join(".aiplus/agent-memory/engineer-a/seed.md"),
        "safe line\napi_key=secret123\npassword: hunter2\n",
    )
    .unwrap();

    run(
        &project,
        &cache_root,
        &["agent", "cache", "--enable-disk"],
        0,
    );
    run(
        &project,
        &cache_root,
        &["agent", "route", "engineer-a", "redaction cache write"],
        0,
    );
    let cbor = project_cache_dir(&cache_root, &project).join("engineer-a.cbor");
    let bytes = fs::read(cbor).unwrap();
    let strings_view = String::from_utf8_lossy(&bytes);
    assert!(!strings_view.contains("secret123"));
    assert!(!strings_view.contains("hunter2"));
    assert!(strings_view.contains("REDACTED_BY_AIPLUS_CACHE"));
}

#[test]
fn clear_removes_only_current_project_cache() {
    let temp = tempfile::tempdir().unwrap();
    let project_a = temp.path().join("repo-ac5-a");
    let project_b = temp.path().join("repo-ac5-b");
    let cache_root = temp.path().join("cache-root");
    prepare_project(&project_a);
    prepare_project(&project_b);

    for project in [&project_a, &project_b] {
        run(
            project,
            &cache_root,
            &["agent", "cache", "--enable-disk"],
            0,
        );
        run(
            project,
            &cache_root,
            &["agent", "route", "engineer-a", "write cache"],
            0,
        );
    }
    assert!(project_cache_dir(&cache_root, &project_a).exists());
    assert!(project_cache_dir(&cache_root, &project_b).exists());

    run(&project_a, &cache_root, &["agent", "cache", "--clear"], 0);
    assert!(!project_cache_dir(&cache_root, &project_a).exists());
    assert!(project_cache_dir(&cache_root, &project_b).exists());
}

#[test]
fn doctor_warns_when_cache_root_is_under_sync_folder() {
    let temp = tempfile::tempdir().unwrap();
    let project = temp.path().join("repo-sync-warning");
    let cache_root = temp
        .path()
        .join("Dropbox")
        .join(".cache")
        .join("aiplus-agent-team");
    prepare_project(&project);

    run(
        &project,
        &cache_root,
        &["agent", "cache", "--enable-disk"],
        0,
    );
    let doctor = run(&project, &cache_root, &["agent", "doctor"], 0);
    assert!(stdout(&doctor).contains("sync folder"));
}
