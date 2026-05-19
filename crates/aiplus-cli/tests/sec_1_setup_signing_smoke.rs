use std::fs;
use std::path::Path;
use std::process::{Command, Output};

fn bin() -> &'static str {
    env!("CARGO_BIN_EXE_aiplus")
}

fn run(cwd: &Path, args: &[&str], expected: i32, envs: &[(&str, &str)]) -> Output {
    let home = cwd.join("fake-home");
    let config_home = cwd.join("fake-xdg");
    let git_config = cwd.join("fake-gitconfig");
    fs::create_dir_all(&home).unwrap();
    fs::create_dir_all(&config_home).unwrap();

    let mut command = Command::new(bin());
    command
        .args(args)
        .current_dir(cwd)
        .env("HOME", &home)
        .env("XDG_CONFIG_HOME", &config_home)
        .env("GIT_CONFIG_GLOBAL", &git_config);
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

fn stdout(output: &Output) -> String {
    String::from_utf8_lossy(&output.stdout).to_string()
}

fn stderr(output: &Output) -> String {
    String::from_utf8_lossy(&output.stderr).to_string()
}

#[test]
fn setup_signing_dry_run_does_not_write_home_or_git_config() {
    let temp = tempfile::tempdir().unwrap();
    let target = temp.path();

    let output = run(
        target,
        &["identity", "setup-signing", "--dry-run"],
        0,
        &[("AIPLUS_SETUP_SIGNING_FORCE_PLATFORM", "macos")],
    );
    let out = stdout(&output);
    assert!(out.contains("SETUP_SIGNING_STATUS=DRY_RUN"), "got: {out}");
    assert!(out.contains("planned_ssh_keygen=ssh-keygen -t ecdsa-sk"));
    assert!(!target.join("fake-home/.ssh").exists());
    assert!(!target.join("fake-gitconfig").exists());
}

#[test]
fn setup_signing_non_macos_gracefully_reports_unsupported() {
    let temp = tempfile::tempdir().unwrap();
    let target = temp.path();

    let output = run(
        target,
        &["identity", "setup-signing"],
        3,
        &[("AIPLUS_SETUP_SIGNING_FORCE_PLATFORM", "linux")],
    );
    let err = stderr(&output);
    assert!(
        err.contains("SETUP_SIGNING_STATUS=UNSUPPORTED platform=linux"),
        "got: {err}"
    );
}

#[test]
fn setup_signing_fake_keygen_writes_isolated_git_config() {
    let temp = tempfile::tempdir().unwrap();
    let target = temp.path();

    let output = run(
        target,
        &["identity", "setup-signing"],
        0,
        &[
            ("AIPLUS_SETUP_SIGNING_FORCE_PLATFORM", "macos"),
            ("AIPLUS_SETUP_SIGNING_FAKE_KEYGEN", "1"),
        ],
    );
    let out = stdout(&output);
    assert!(out.contains("SETUP_SIGNING_STATUS=PASS"), "got: {out}");
    assert!(out.contains("signing_key=generated_fake"), "got: {out}");

    let home = target.join("fake-home");
    let key_path = home.join(".ssh/id_ecdsa_sk_aiplus");
    let pub_path = home.join(".ssh/id_ecdsa_sk_aiplus.pub");
    let allowed = home.join(".ssh/aiplus_allowed_signers");
    assert!(key_path.exists());
    assert!(pub_path.exists());
    assert!(allowed.exists());
    assert!(fs::read_to_string(&allowed)
        .unwrap()
        .contains("aiplus-owner@example.invalid sk-ecdsa-sha2-nistp256@openssh.com"));

    let git_config = fs::read_to_string(target.join("fake-gitconfig")).unwrap();
    assert!(git_config.contains("format = ssh"), "got: {git_config}");
    assert!(git_config.contains("gpgsign = true"), "got: {git_config}");
    assert!(
        git_config.contains("id_ecdsa_sk_aiplus.pub"),
        "got: {git_config}"
    );
    assert!(
        git_config.contains("aiplus_allowed_signers"),
        "got: {git_config}"
    );

    let doctor = run(
        target,
        &["agent", "doctor"],
        0,
        &[("AIPLUS_SETUP_SIGNING_FORCE_PLATFORM", "macos")],
    );
    let doctor_out = stdout(&doctor);
    assert!(
        doctor_out.contains("INFO commit_signing=secure_enclave"),
        "got: {doctor_out}"
    );
}

#[test]
fn setup_signing_refuses_to_clobber_existing_signing_config() {
    let temp = tempfile::tempdir().unwrap();
    let target = temp.path();
    fs::create_dir_all(target.join("fake-home")).unwrap();
    fs::write(target.join("fake-gitconfig"), "[gpg]\n\tformat = openpgp\n").unwrap();

    let output = run(
        target,
        &["identity", "setup-signing"],
        3,
        &[
            ("AIPLUS_SETUP_SIGNING_FORCE_PLATFORM", "macos"),
            ("AIPLUS_SETUP_SIGNING_FAKE_KEYGEN", "1"),
        ],
    );
    let err = stderr(&output);
    assert!(
        err.contains(
            "SETUP_SIGNING_STATUS=REFUSED reason=existing_git_signing_config key=gpg.format"
        ),
        "got: {err}"
    );
    assert!(!target.join("fake-home/.ssh/id_ecdsa_sk_aiplus").exists());
}
