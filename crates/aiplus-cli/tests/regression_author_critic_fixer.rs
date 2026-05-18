use serde_json::Value;
use std::collections::BTreeSet;
use std::fs;
use std::path::Path;
use std::process::{Command, Output};

fn bin() -> &'static str {
    env!("CARGO_BIN_EXE_aiplus")
}

fn run(cwd: &Path, args: &[&str], expected: i32) -> Output {
    let output = Command::new(bin())
        .args(args)
        .current_dir(cwd)
        .env("HOME", cwd.join("fake-home"))
        .env("CODEX_HOME", cwd.join("fake-codex-home"))
        .env("XDG_CONFIG_HOME", cwd.join("fake-xdg"))
        .env("AIPLUS_SECRET_BROKER_DISABLE_KEYCHAIN", "1")
        .env_remove("BWS_ACCESS_TOKEN")
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

fn combined(output: &Output) -> String {
    format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    )
}

#[test]
fn author_critic_fixer_routes_ael_pi_through_independent_referee() {
    let temp = tempfile::tempdir().unwrap();
    let target = temp.path();
    run(target, &["install", "codex"], 0);
    run(target, &["add", "aieconlab"], 0);

    let output = run(
        target,
        &[
            "agent",
            "route",
            "--workflow",
            "author-critic-fixer",
            "pi",
            "draft",
            "section",
            "X",
        ],
        0,
    );
    let text = combined(&output);
    assert!(
        text.contains("Author/Critic/Fixer workflow"),
        "workflow output should announce the workflow:\n{text}"
    );
    assert!(
        text.contains("Phase 1/3 author")
            && text.contains("Phase 2/3 critic")
            && text.contains("Phase 3/3 fixer"),
        "workflow output should list all three phases:\n{text}"
    );
    assert!(
        text.contains("critic=referee") && text.contains("v2 draft dispatched to pi"),
        "AEL workflow should use referee as independent critic and dispatch v2:\n{text}"
    );

    let workflow_log = target.join(".aiplus/agents/workflow-log.jsonl");
    let body = fs::read_to_string(&workflow_log).expect("workflow log exists");
    let rows: Vec<Value> = body
        .lines()
        .map(|line| serde_json::from_str(line).expect("workflow log line parses"))
        .collect();
    assert_eq!(
        rows.len(),
        3,
        "expected exactly three workflow phases:\n{body}"
    );

    let phases: Vec<_> = rows
        .iter()
        .map(|row| row.get("phase").and_then(|v| v.as_str()).unwrap_or(""))
        .collect();
    assert_eq!(phases, vec!["author", "critic", "fixer"]);
    assert_eq!(
        rows[1].get("role").and_then(|v| v.as_str()),
        Some("referee"),
        "critic phase must dispatch the independent AEL referee:\n{body}"
    );

    let agent_ids: BTreeSet<_> = rows
        .iter()
        .map(|row| row.get("agent_id").and_then(|v| v.as_str()).unwrap_or(""))
        .collect();
    assert!(
        agent_ids.len() >= 2,
        "audit log must show at least two distinct agent_id values:\n{body}"
    );
    assert_ne!(
        rows[0].get("agent_id"),
        rows[1].get("agent_id"),
        "critic agent_id must differ from author agent_id:\n{body}"
    );

    let dispatch_log =
        fs::read_to_string(target.join(".aiplus/agents/dispatch-log.jsonl")).unwrap();
    let roles: Vec<_> = dispatch_log
        .lines()
        .filter_map(|line| serde_json::from_str::<Value>(line).ok())
        .map(|row| {
            row.get("role")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string()
        })
        .collect();
    assert!(
        roles.ends_with(&["pi".to_string(), "referee".to_string(), "pi".to_string()]),
        "dispatch log should record author/referee/fixer roles, got {roles:?}\n{dispatch_log}"
    );
}

#[test]
fn author_critic_fixer_rejects_referee_as_author_in_ael() {
    let temp = tempfile::tempdir().unwrap();
    let target = temp.path();
    run(target, &["install", "codex"], 0);
    run(target, &["add", "aieconlab"], 0);

    let output = run(
        target,
        &[
            "agent",
            "route",
            "--workflow",
            "author-critic-fixer",
            "referee",
            "draft",
            "critique",
        ],
        3,
    );
    let text = combined(&output);
    assert!(
        text.contains("requires an independent critic"),
        "workflow should refuse self-critique by the configured critic:\n{text}"
    );
}
