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

fn setup_fake_env(target: &Path) {
    fs::create_dir(target.join("fake-home")).unwrap();
    fs::create_dir(target.join("fake-codex-home")).unwrap();
    fs::create_dir(target.join("fake-xdg")).unwrap();
}

#[test]
fn agent_status_doctor_list_no_project_name_placeholder() {
    let temp = tempfile::tempdir().unwrap();
    let target = temp.path();
    setup_fake_env(target);

    // Create .aiplus/agents/ with a config that uses {project_name} template
    let agents_dir = target.join(".aiplus/agents");
    fs::create_dir_all(&agents_dir).unwrap();
    fs::write(
        agents_dir.join("engineer-a.toml"),
        r#"schema_version = "1.0"

[agent]
role = "engineer-a"
display_name = "Engineer A"
status = "active"

[workspace]
needs_worktree = true
worktree_path = "../{project_name}.engineer-a"
"#,
    )
    .unwrap();

    let status = stdout(&run(target, &["agent", "status"], 0));
    assert!(
        !status.contains("{project_name}"),
        "agent status should not contain {{project_name}} placeholder:\n{status}"
    );
    // Verify it was substituted with actual project name
    assert!(
        status.contains(".engineer-a"),
        "agent status should contain resolved worktree path:\n{status}"
    );

    let doctor = stdout(&run(target, &["agent", "doctor"], 0));
    assert!(
        !doctor.contains("{project_name}"),
        "agent doctor should not contain {{project_name}} placeholder:\n{doctor}"
    );
    assert!(
        doctor.contains(".engineer-a"),
        "agent doctor should contain resolved worktree path:\n{doctor}"
    );

    let list = stdout(&run(target, &["agent", "list"], 0));
    assert!(
        !list.contains("{project_name}"),
        "agent list should not contain {{project_name}} placeholder:\n{list}"
    );
}
