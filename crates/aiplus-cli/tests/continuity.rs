use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::thread;

fn bin() -> &'static str {
    env!("CARGO_BIN_EXE_aiplus")
}

struct FakeEnv {
    home: PathBuf,
    xdg: PathBuf,
    codex: PathBuf,
}

fn fake_env(root: &Path) -> FakeEnv {
    let env = FakeEnv {
        home: root.join("home"),
        xdg: root.join("xdg"),
        codex: root.join("codex"),
    };
    fs::create_dir_all(&env.home).unwrap();
    fs::create_dir_all(&env.xdg).unwrap();
    fs::create_dir_all(&env.codex).unwrap();
    env
}

fn run(cwd: &Path, env: &FakeEnv, args: &[&str], expected: i32) -> Output {
    let output = Command::new(bin())
        .args(args)
        .current_dir(cwd)
        .env("HOME", &env.home)
        .env("XDG_CONFIG_HOME", &env.xdg)
        .env("CODEX_HOME", &env.codex)
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

fn stderr(output: &Output) -> String {
    String::from_utf8_lossy(&output.stderr).to_string()
}

#[test]
fn memory_init_status_doctor_context_and_project_isolation() {
    let temp = tempfile::tempdir().unwrap();
    let env = fake_env(temp.path());
    let alpha = temp.path().join("projects/alpha");
    let beta = temp.path().join("projects/beta");
    let gamma = temp.path().join("projects/gamma");
    for project in [&alpha, &beta, &gamma] {
        fs::create_dir_all(project).unwrap();
        run(project, &env, &["memory", "init", "--project"], 0);
        let status = stdout(&run(project, &env, &["memory", "status"], 0));
        assert!(status.contains("MEMORY_STATUS=PASS"));
        assert!(status.contains("installed=yes"));
        let doctor = stdout(&run(project, &env, &["memory", "doctor"], 0));
        assert!(doctor.contains("MEMORY_DOCTOR_STATUS=PASS"));
    }

    run(
        &alpha,
        &env,
        &[
            "memory",
            "add",
            "--scope",
            "project",
            "--kind",
            "preference",
            "--text",
            "Alpha release summaries should be concise.",
        ],
        0,
    );
    run(
        &beta,
        &env,
        &[
            "memory",
            "add",
            "--scope",
            "project",
            "--kind",
            "preference",
            "--text",
            "Beta reviews should list findings first.",
        ],
        0,
    );
    run(
        &gamma,
        &env,
        &[
            "memory",
            "add",
            "--scope",
            "project",
            "--kind",
            "preference",
            "--text",
            "Gamma handoffs should include next action.",
        ],
        0,
    );

    let alpha_context = stdout(&run(
        &alpha,
        &env,
        &[
            "memory",
            "context",
            "--runtime",
            "codex",
            "--budget",
            "2000",
        ],
        0,
    ));
    assert!(alpha_context.contains("MEMORY_CONTEXT_STATUS=PASS"));
    assert!(alpha_context.contains("Alpha release summaries"));
    assert!(!alpha_context.contains("Beta reviews"));
    assert!(!alpha_context.contains("Gamma handoffs"));

    let beta_search = stdout(&run(&beta, &env, &["memory", "search", "reviews"], 0));
    assert!(beta_search.contains("matches=1"));
    assert!(
        !alpha.join(".aiplus/memory/project-memory.jsonl").exists()
            || beta_search.contains("match=")
    );
    assert!(!env.xdg.join("aiplus/continuity").exists());
}

#[test]
fn identity_init_status_and_context_roles() {
    let temp = tempfile::tempdir().unwrap();
    let env = fake_env(temp.path());
    let project = temp.path().join("project");
    fs::create_dir_all(&project).unwrap();

    let before = stdout(&run(&project, &env, &["identity", "status"], 0));
    assert!(before.contains("installed=no"));
    run(&project, &env, &["identity", "init", "--project"], 0);
    let status = stdout(&run(&project, &env, &["identity", "status"], 0));
    assert!(status.contains("advisor=present"));
    assert!(status.contains("ceo=present"));
    let advisor = stdout(&run(
        &project,
        &env,
        &["identity", "context", "--role", "advisor"],
        0,
    ));
    assert!(advisor.contains("role=advisor"));
    assert!(advisor.contains("identity_grants_permission=no"));
    let ceo = stdout(&run(
        &project,
        &env,
        &["identity", "context", "--role", "ceo"],
        0,
    ));
    assert!(ceo.contains("role = \"CEO\""));
}

#[test]
fn skill_candidate_propose_reject_status() {
    let temp = tempfile::tempdir().unwrap();
    let env = fake_env(temp.path());
    let project = temp.path().join("project");
    fs::create_dir_all(&project).unwrap();
    let proposed = stdout(&run(
        &project,
        &env,
        &[
            "skill-candidate",
            "propose",
            "--title",
            "Release checklist reviewer",
            "--from-memory",
            "mem_safe_example",
        ],
        0,
    ));
    assert!(proposed.contains("SKILL_CANDIDATE_PROPOSE_STATUS=PASS"));
    let id = proposed
        .lines()
        .find_map(|line| line.strip_prefix("id="))
        .unwrap()
        .to_string();
    let status = stdout(&run(&project, &env, &["skill-candidate", "status"], 0));
    assert!(status.contains("candidate_proposed=1"));
    run(&project, &env, &["skill-candidate", "reject", &id], 0);
    let status = stdout(&run(&project, &env, &["skill-candidate", "status"], 0));
    assert!(status.contains("rejected=1"));
}

#[test]
fn memory_redaction_blocks_sensitive_patterns() {
    let temp = tempfile::tempdir().unwrap();
    let env = fake_env(temp.path());
    let project = temp.path().join("project");
    fs::create_dir_all(&project).unwrap();
    for text in [
        "Authorization: Bearer abcdefghijklmnopqrstuvwxyz",
        "/Users/example/private/project",
        "provider response body contained sensitive data",
        "PENDING_OWNER_INPUT_DO_NOT_USE",
    ] {
        let blocked = run(
            &project,
            &env,
            &[
                "memory",
                "add",
                "--scope",
                "project",
                "--kind",
                "preference",
                "--text",
                text,
            ],
            1,
        );
        let err = stderr(&blocked);
        assert!(err.contains("MEMORY_REDACTION_STATUS=BLOCKED"));
        assert!(!err.contains(text));
    }
}

#[test]
fn memory_atomic_concurrency_smoke() {
    let temp = tempfile::tempdir().unwrap();
    let env = fake_env(temp.path());
    let project = temp.path().join("project");
    fs::create_dir_all(&project).unwrap();
    run(&project, &env, &["memory", "init", "--project"], 0);

    let mut handles = Vec::new();
    for index in 0..8 {
        let project = project.clone();
        let home = env.home.clone();
        let xdg = env.xdg.clone();
        let codex = env.codex.clone();
        handles.push(thread::spawn(move || {
            let text = format!("Concurrent memory record {index}");
            let output = Command::new(bin())
                .args([
                    "memory",
                    "add",
                    "--scope",
                    "project",
                    "--kind",
                    "preference",
                    "--text",
                    &text,
                ])
                .current_dir(project)
                .env("HOME", home)
                .env("XDG_CONFIG_HOME", xdg)
                .env("CODEX_HOME", codex)
                .output()
                .expect("run memory add");
            assert!(output.status.success());
        }));
    }
    for handle in handles {
        handle.join().unwrap();
    }
    let status = stdout(&run(&project, &env, &["memory", "status"], 0));
    assert!(status.contains("records_total=8"));
    let body = fs::read_to_string(project.join(".aiplus/memory/project-memory.jsonl")).unwrap();
    assert_eq!(
        body.lines().filter(|line| !line.trim().is_empty()).count(),
        8
    );
    for line in body.lines().filter(|line| !line.trim().is_empty()) {
        serde_json::from_str::<serde_json::Value>(line).unwrap();
    }
    assert!(!project.join(".aiplus/memory/project-memory.lock").exists());
}
