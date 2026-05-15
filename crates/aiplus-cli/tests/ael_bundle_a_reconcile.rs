use std::fs;
use std::path::Path;
use std::process::{Command, Output};

fn bin() -> &'static str {
    env!("CARGO_BIN_EXE_aiplus")
}

fn run(cwd: &Path, args: &[&str], expected: i32) -> Output {
    let mut command = Command::new(bin());
    command
        .args(args)
        .current_dir(cwd)
        .env("HOME", cwd.join("fake-home"))
        .env("CODEX_HOME", cwd.join("fake-codex-home"))
        .env("XDG_CONFIG_HOME", cwd.join("fake-xdg"));
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

fn prepare(target: &Path) {
    fs::create_dir(target.join("fake-home")).unwrap();
    fs::create_dir(target.join("fake-codex-home")).unwrap();
    fs::create_dir(target.join("fake-xdg")).unwrap();
}

fn remove_managed_block(text: &str, begin: &str, end: &str) -> String {
    let Some(start) = text.find(begin) else {
        return text.to_string();
    };
    let Some(end_start) = text[start..].find(end).map(|idx| start + idx) else {
        return text.to_string();
    };
    let end_idx = end_start + end.len();
    format!("{}{}", &text[..start], &text[end_idx..])
}

fn assert_ael_claude_repaired(target: &Path) {
    assert!(target.join(".claude/agents/aieconlab-pi.md").exists());
    assert!(target
        .join(".claude/agents/aieconlab-ra-python.md")
        .exists());
    assert!(target.join(".claude/commands/aiel-route.md").exists());
    let claude = fs::read_to_string(target.join("CLAUDE.md")).unwrap();
    assert_eq!(
        claude
            .matches("<!-- BEGIN AIECONLAB MANAGED BLOCK -->")
            .count(),
        1,
        "expected exactly one AEL block:\n{claude}"
    );
}

#[test]
fn add_existing_aieconlab_re_materializes_missing_claude_adapter() {
    let temp = tempfile::tempdir().unwrap();
    let target = temp.path();
    prepare(target);
    fs::write(target.join("CLAUDE.md"), "## User Notes\nKeep this.\n").unwrap();

    run(target, &["install", "claude-code"], 0);
    run(target, &["add", "aieconlab"], 0);

    fs::remove_file(target.join(".claude/agents/aieconlab-pi.md")).unwrap();
    fs::remove_file(target.join(".claude/commands/aiel-route.md")).unwrap();
    let claude = fs::read_to_string(target.join("CLAUDE.md")).unwrap();
    let claude = remove_managed_block(
        &claude,
        "<!-- BEGIN AIECONLAB MANAGED BLOCK -->",
        "<!-- END AIECONLAB MANAGED BLOCK -->",
    );
    fs::write(target.join("CLAUDE.md"), claude).unwrap();

    let broken = stdout(&run(target, &["doctor"], 0));
    assert!(broken.contains("DOCTOR_STATUS=NEEDS_FIX"), "{broken}");

    run(target, &["add", "aieconlab"], 0);
    assert_ael_claude_repaired(target);
    let claude = fs::read_to_string(target.join("CLAUDE.md")).unwrap();
    assert!(
        claude.contains("## User Notes\nKeep this."),
        "user content should be preserved:\n{claude}"
    );

    let doctor = stdout(&run(target, &["doctor"], 0));
    assert!(doctor.contains("DOCTOR_STATUS=PASS"), "{doctor}");
}

#[test]
fn install_new_runtime_reconciles_existing_aieconlab_module() {
    let temp = tempfile::tempdir().unwrap();
    let target = temp.path();
    prepare(target);
    fs::write(target.join("CLAUDE.md"), "## Claude Notes\nPreserve me.\n").unwrap();

    run(target, &["install", "codex"], 0);
    run(target, &["add", "aieconlab"], 0);
    assert!(
        !target.join(".claude/agents/aieconlab-pi.md").exists(),
        "codex-only AEL install should not write Claude artifacts yet"
    );

    run(target, &["install", "claude-code"], 0);

    assert_ael_claude_repaired(target);
    let claude = fs::read_to_string(target.join("CLAUDE.md")).unwrap();
    assert!(
        claude.contains("## Claude Notes\nPreserve me."),
        "user content should survive runtime install:\n{claude}"
    );
    let doctor = stdout(&run(target, &["doctor"], 0));
    assert!(doctor.contains("DOCTOR_STATUS=PASS"), "{doctor}");
}

#[test]
fn doctor_fix_repairs_missing_aieconlab_adapter_and_is_idempotent() {
    let temp = tempfile::tempdir().unwrap();
    let target = temp.path();
    prepare(target);

    run(target, &["install", "claude-code"], 0);
    run(target, &["add", "aieconlab"], 0);
    fs::remove_file(target.join(".claude/agents/aieconlab-ra-python.md")).unwrap();
    fs::remove_file(target.join(".claude/commands/aiel-status.md")).unwrap();

    let broken = stdout(&run(target, &["doctor"], 0));
    assert!(broken.contains("DOCTOR_STATUS=NEEDS_FIX"), "{broken}");

    let fixed = stdout(&run(target, &["doctor", "--fix"], 0));
    assert!(fixed.contains("fixAttempted=yes"), "{fixed}");
    assert!(
        fixed.contains("fixReconciledModules=[agent-team,aieconlab]"),
        "{fixed}"
    );
    assert!(fixed.contains("fixRemainingUnsupported=[]"), "{fixed}");
    assert!(fixed.contains("DOCTOR_STATUS=PASS"), "{fixed}");
    assert_ael_claude_repaired(target);

    let healthy = stdout(&run(target, &["doctor", "--fix"], 0));
    assert!(healthy.contains("fixAttempted=yes"), "{healthy}");
    assert!(healthy.contains("fixRemainingUnsupported=[]"), "{healthy}");
    assert!(healthy.contains("DOCTOR_STATUS=PASS"), "{healthy}");
    assert_ael_claude_repaired(target);
}
