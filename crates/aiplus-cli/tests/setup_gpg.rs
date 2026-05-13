use std::fs;
use std::path::Path;
use std::process::{Command, Output};

fn bin() -> &'static str {
    env!("CARGO_BIN_EXE_aiplus")
}

fn run(cwd: &Path, args: &[&str], expected: i32, envs: &[(&str, &str)]) -> Output {
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

fn stderr(output: &Output) -> String {
    String::from_utf8_lossy(&output.stderr).to_string()
}

fn stdout(output: &Output) -> String {
    String::from_utf8_lossy(&output.stdout).to_string()
}

#[cfg(unix)]
#[test]
fn setup_gpg_sentinel_absent_exits_3() {
    let temp = tempfile::tempdir().unwrap();
    let target = temp.path();
    fs::create_dir(target.join("fake-home")).unwrap();
    fs::create_dir(target.join("fake-codex-home")).unwrap();
    fs::create_dir(target.join("fake-xdg")).unwrap();

    let output = run(target, &["agent", "audit", "setup-gpg"], 3, &[]);
    let err = stderr(&output);
    assert!(err.contains("First-run GPG setup requires Owner authorization"));
    assert!(err.contains("cat > .aiplus/agent-team/.owner-setup-authorized"));
    assert!(err.contains("Then re-run: aiplus agent audit setup-gpg"));
}

#[cfg(unix)]
#[test]
fn setup_gpg_sentinel_malformed_exits_3() {
    let temp = tempfile::tempdir().unwrap();
    let target = temp.path();
    fs::create_dir(target.join("fake-home")).unwrap();
    fs::create_dir(target.join("fake-codex-home")).unwrap();
    fs::create_dir(target.join("fake-xdg")).unwrap();
    fs::create_dir_all(target.join(".aiplus/agent-team")).unwrap();
    fs::write(
        target.join(".aiplus/agent-team/.owner-setup-authorized"),
        "not yaml at all",
    )
    .unwrap();

    let output = run(target, &["agent", "audit", "setup-gpg"], 3, &[]);
    let err = stderr(&output);
    assert!(err.contains("First-run GPG setup requires Owner authorization"));
}

#[cfg(unix)]
#[test]
fn setup_gpg_end_to_end_with_fake_gpg() {
    let temp = tempfile::tempdir().unwrap();
    let target = temp.path();
    fs::create_dir(target.join("fake-home")).unwrap();
    fs::create_dir(target.join("fake-codex-home")).unwrap();
    fs::create_dir(target.join("fake-xdg")).unwrap();
    fs::create_dir_all(target.join(".aiplus/agent-team")).unwrap();

    // Write sentinel
    fs::write(
        target.join(".aiplus/agent-team/.owner-setup-authorized"),
        "name: Test Owner\nemail: owner@example.com\n",
    )
    .unwrap();

    // Create fake gpg binary
    let fake_bin = target.join("fake-bin");
    fs::create_dir(&fake_bin).unwrap();
    let fake_gpg = fake_bin.join("gpg");
    fs::write(
        &fake_gpg,
        r#"#!/bin/sh
if [ "$1" = "--version" ]; then
    echo "gpg (GnuPG) 2.4.0"
    exit 0
fi
if [ "$1" = "--list-secret-keys" ]; then
    echo "sec:u:255:22:1234567890ABCDEF:1715424000::::::::::"
    echo "fpr:::::::::A1B2C3D4E5F678901234567890ABCDEF12345678:"
    echo "uid:u::::1715424000::1234567890ABCDEF::Test Owner <owner@example.com>:::::::::::"
    exit 0
fi
# For --gen-key, just succeed
exit 0
"#,
    )
    .unwrap();
    {
        use std::os::unix::fs::PermissionsExt;
        let mut perms = fs::metadata(&fake_gpg).unwrap().permissions();
        perms.set_mode(0o755);
        fs::set_permissions(&fake_gpg, perms).unwrap();
    }

    let path_env = format!(
        "{}:{}",
        fake_bin.to_str().unwrap(),
        std::env::var("PATH").unwrap_or_default()
    );

    let output = run(
        target,
        &["agent", "audit", "setup-gpg"],
        0,
        &[
            ("PATH", &path_env),
            ("AIPLUS_TEST_GPG_PASSPHRASE", "testpass123"),
        ],
    );
    let out = stdout(&output);
    assert!(
        out.contains("MANIFEST_SIGNING=gpg_ephemeral_dev"),
        "got: {}",
        out
    );
    assert!(out.contains("SETUP_GPG_STATUS=PASS"), "got: {}", out);

    // Sentinel should be deleted
    assert!(!target
        .join(".aiplus/agent-team/.owner-setup-authorized")
        .exists());

    // Fingerprint should be written
    let fingerprint_path = target.join(".aiplus/agent-team/owner-key-fingerprint");
    assert!(fingerprint_path.exists());
    let fingerprint = fs::read_to_string(&fingerprint_path).unwrap();
    assert_eq!(
        fingerprint.trim(),
        "A1B2C3D4E5F678901234567890ABCDEF12345678"
    );
}
