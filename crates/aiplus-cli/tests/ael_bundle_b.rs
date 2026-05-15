use serde_json::Value;
use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

fn bin() -> &'static str {
    env!("CARGO_BIN_EXE_aiplus")
}

fn prepare(target: &Path) {
    fs::create_dir(target.join("fake-home")).unwrap();
    fs::create_dir(target.join("fake-codex-home")).unwrap();
    fs::create_dir(target.join("fake-xdg")).unwrap();
}

fn run(cwd: &Path, args: &[&str], expected: i32) -> Output {
    run_with_env(cwd, args, expected, &[])
}

fn run_with_env(cwd: &Path, args: &[&str], expected: i32, envs: &[(&str, String)]) -> Output {
    let mut command = Command::new(bin());
    command
        .args(args)
        .current_dir(cwd)
        .env("HOME", cwd.join("fake-home"))
        .env("CODEX_HOME", cwd.join("fake-codex-home"))
        .env("XDG_CONFIG_HOME", cwd.join("fake-xdg"));
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

fn prepend_path(bin_dir: &Path) -> String {
    let current = std::env::var("PATH").unwrap_or_default();
    format!("{}:{current}", bin_dir.display())
}

fn fake_runtime(bin_dir: &Path, name: &str) {
    fs::create_dir_all(bin_dir).unwrap();
    let path = bin_dir.join(name);
    fs::write(
        &path,
        r#"#!/bin/sh
printf '%s\n' "$0" > "$AIPLUS_FAKE_RUNTIME_LOG"
printf '%s\n' "$1" >> "$AIPLUS_FAKE_RUNTIME_LOG"
exit 0
"#,
    )
    .unwrap();
    let mut permissions = fs::metadata(&path).unwrap().permissions();
    permissions.set_mode(0o755);
    fs::set_permissions(&path, permissions).unwrap();
}

fn install_ael(target: &Path, runtime: &str) {
    run(target, &["install", runtime], 0);
    run(target, &["add", "aieconlab"], 0);
}

fn last_dispatch(target: &Path) -> Value {
    let log = fs::read_to_string(target.join(".aiplus/agents/dispatch-log.jsonl")).unwrap();
    let line = log.lines().last().expect("dispatch log line");
    serde_json::from_str(line).unwrap()
}

#[test]
fn talk_runtime_flag_selects_valid_runtime() {
    let temp = tempfile::tempdir().unwrap();
    let target = temp.path();
    prepare(target);
    install_ael(target, "opencode");

    let bin_dir = target.join("fake-bin");
    fake_runtime(&bin_dir, "opencode");
    let runtime_log = target.join("runtime.log");

    let out = run_with_env(
        target,
        &["agent", "talk", "--runtime", "opencode", "pi"],
        0,
        &[
            ("PATH", prepend_path(&bin_dir)),
            (
                "AIPLUS_FAKE_RUNTIME_LOG",
                runtime_log.to_string_lossy().into_owned(),
            ),
        ],
    );
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(stdout.contains("Opening runtime=opencode session as role=pi"));
    assert!(stdout.contains("talk_audit runtime=opencode role=pi"));

    let runtime_invocation = fs::read_to_string(runtime_log).unwrap();
    assert!(runtime_invocation
        .lines()
        .next()
        .unwrap()
        .ends_with("opencode"));
    assert!(runtime_invocation.contains("You are the `pi`"));
}

#[test]
fn talk_runtime_flag_rejects_invalid_runtime() {
    let temp = tempfile::tempdir().unwrap();
    let target = temp.path();
    prepare(target);
    install_ael(target, "codex");

    let out = run(
        target,
        &["agent", "talk", "--runtime", "not-a-runtime", "pi"],
        3,
    );
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(stderr.contains("Invalid runtime `not-a-runtime`"));
    assert!(stderr.contains("codex, claude-code, opencode"));
}

#[test]
fn talk_without_runtime_keeps_default_detection() {
    let temp = tempfile::tempdir().unwrap();
    let target = temp.path();
    prepare(target);
    install_ael(target, "codex");

    let bin_dir = target.join("fake-bin");
    fake_runtime(&bin_dir, "codex");
    let runtime_log = target.join("runtime.log");

    let out = run_with_env(
        target,
        &["agent", "talk", "pi"],
        0,
        &[
            ("PATH", prepend_path(&bin_dir)),
            (
                "AIPLUS_FAKE_RUNTIME_LOG",
                runtime_log.to_string_lossy().into_owned(),
            ),
        ],
    );
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(stdout.contains("Opening runtime=codex session as role=pi"));
    assert!(fs::read_to_string(runtime_log)
        .unwrap()
        .contains("You are the `pi`"));
}

#[test]
fn route_ascii_aliases_record_canonical_role_and_source() {
    let temp = tempfile::tempdir().unwrap();
    let target = temp.path();
    prepare(target);
    install_ael(target, "codex");

    let out = run(target, &["agent", "route", "CEO", "draft", "intro"], 0);
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(stdout.contains("Resolved role alias `CEO` -> `pi`"));
    assert!(stdout.contains("Routing task to pi: draft intro"));

    let dispatch = last_dispatch(target);
    assert_eq!(dispatch["role"], "pi");
    assert_eq!(dispatch["roleInput"], "CEO");

    let transcript = run(target, &["agent", "transcript"], 0);
    let transcript_stdout = String::from_utf8_lossy(&transcript.stdout);
    assert!(transcript_stdout.contains("pi (from CEO)"));
}

#[test]
fn route_chinese_aliases_record_canonical_roles_and_sources() {
    let temp = tempfile::tempdir().unwrap();
    let target = temp.path();
    prepare(target);
    install_ael(target, "codex");

    for (alias, canonical) in [
        ("顾问", "advisor"),
        ("回归", "ra-stata"),
        ("计量", "econometrician"),
    ] {
        let out = run(target, &["agent", "route", alias, "check"], 0);
        let stdout = String::from_utf8_lossy(&out.stdout);
        assert!(
            stdout.contains(&format!("Resolved role alias `{alias}` -> `{canonical}`")),
            "stdout missing alias resolution:\n{stdout}"
        );

        let dispatch = last_dispatch(target);
        assert_eq!(dispatch["role"], canonical);
        assert_eq!(dispatch["roleInput"], alias);
    }
}

#[test]
fn route_unknown_chinese_alias_fails_clearly() {
    let temp = tempfile::tempdir().unwrap();
    let target = temp.path();
    prepare(target);
    install_ael(target, "codex");

    let out = run(target, &["agent", "route", "不存在", "check"], 3);
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(stderr.contains("Unknown AiEconLab role alias `不存在`"));
    assert!(stderr.contains("Canonical role ids continue to work"));
}

#[test]
fn talk_chinese_alias_resolves_to_canonical_persona() {
    let temp = tempfile::tempdir().unwrap();
    let target = temp.path();
    prepare(target);
    install_ael(target, "codex");

    let bin_dir = target.join("fake-bin");
    fake_runtime(&bin_dir, "codex");
    let runtime_log = PathBuf::from(target).join("runtime.log");

    let out = run_with_env(
        target,
        &["agent", "talk", "主作者"],
        0,
        &[
            ("PATH", prepend_path(&bin_dir)),
            (
                "AIPLUS_FAKE_RUNTIME_LOG",
                runtime_log.to_string_lossy().into_owned(),
            ),
        ],
    );
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(stdout.contains("Resolved role alias `主作者` -> `pi`"));
    assert!(stdout.contains("Opening runtime=codex session as role=pi"));
    assert!(fs::read_to_string(runtime_log)
        .unwrap()
        .contains("You are the `pi`"));
}
