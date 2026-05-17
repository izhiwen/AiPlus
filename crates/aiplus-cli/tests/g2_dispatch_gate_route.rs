use serde::Deserialize;
use std::fs;
use std::path::Path;
use std::process::{Command, Output};

fn bin() -> &'static str {
    env!("CARGO_BIN_EXE_aiplus")
}

fn run(cwd: &Path, args: &[String], expected: i32) -> Output {
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

fn stdout(output: &Output) -> String {
    String::from_utf8_lossy(&output.stdout).to_string()
}

fn stderr(output: &Output) -> String {
    String::from_utf8_lossy(&output.stderr).to_string()
}

#[derive(Debug, Deserialize)]
struct FixtureSample {
    id: String,
    category: String,
    expected_gate: bool,
    task: String,
}

fn fixture_samples() -> Vec<FixtureSample> {
    include_str!("fixtures/g2_dispatch_gate_samples.jsonl")
        .lines()
        .enumerate()
        .map(|(idx, line)| {
            serde_json::from_str::<FixtureSample>(line)
                .unwrap_or_else(|err| panic!("fixture line {} should parse: {err}", idx + 1))
        })
        .collect()
}

fn setup_project_with_g2_gate_team(target: &Path) {
    fs::create_dir(target.join("fake-home")).unwrap();
    fs::create_dir(target.join("fake-codex-home")).unwrap();
    fs::create_dir(target.join("fake-xdg")).unwrap();
    run(target, &["install".into(), "codex".into()], 0);

    fs::write(
        target.join(".aiplus/consultant-team.toml"),
        r#"
schema_version = "0.1"

[[members]]
id = "release_automation"
name = "Release / Automation"
default_tiers = ["LIGHT", "MEDIUM", "HEAVY"]
triggers = ["release", "tag", "publish", "artifact", "upload", "deploy"]

[[members]]
id = "trust_safety"
name = "Trust / Safety"
default_tiers = ["LIGHT", "MEDIUM", "HEAVY"]
triggers = ["secret", "external account", "private data", "telemetry", "global config"]

[[triggers]]
id = "release"
patterns = ["release", "tag", "publish", "artifact", "upload", "deploy"]
tier = "HEAVY"
members = ["release_automation", "trust_safety"]
stop_gate = true

[owner_gates]
push = true
tag = true
release = true
artifact_upload = true
package_publish = true
deploy = true
global_config_edit = true
external_account_mutation = true
secret_exposure = true
private_data_upload = true
telemetry = true
send_delete_publish_or_mutate_external_content = true
"#,
    )
    .unwrap();
}

fn route_args_for_task(task: &str) -> Vec<String> {
    vec![
        "agent".into(),
        "route".into(),
        "qa".into(),
        task.to_string(),
    ]
}

fn all_owner_approvals_for_task(task: &str) -> Vec<String> {
    let mut args = vec!["agent".into(), "route".into()];
    for gate in [
        "push",
        "tag",
        "release",
        "artifact_upload",
        "package_publish",
        "deploy",
        "global_config_edit",
        "external_account_mutation",
        "secret_exposure",
        "private_data_upload",
        "telemetry",
        "send_delete_publish_or_mutate_external_content",
    ] {
        args.push("--owner-approved".into());
        args.push(gate.into());
    }
    args.push("qa".into());
    args.push(task.to_string());
    args
}

fn gate_records(target: &Path) -> Vec<serde_json::Value> {
    let team_dir = target.join(".aiplus/agent-memory/_team");
    if !team_dir.exists() {
        return Vec::new();
    }
    fs::read_dir(team_dir)
        .unwrap()
        .filter_map(|entry| entry.ok())
        .map(|entry| entry.path())
        .filter(|path| {
            path.file_name()
                .and_then(|name| name.to_str())
                .map(|name| name.starts_with("gates-") && name.ends_with(".jsonl"))
                .unwrap_or(false)
        })
        .flat_map(|path| {
            fs::read_to_string(path)
                .unwrap()
                .lines()
                .map(str::to_string)
                .collect::<Vec<_>>()
        })
        .map(|line| serde_json::from_str::<serde_json::Value>(&line).expect("gate record JSON"))
        .collect()
}

fn has_gate_record(target: &Path, task: &str, status: &str) -> bool {
    gate_records(target).into_iter().any(|record| {
        record.get("task").and_then(|v| v.as_str()) == Some(task)
            && record.get("status").and_then(|v| v.as_str()) == Some(status)
            && record
                .get("gateId")
                .and_then(|v| v.as_str())
                .map(|s| !s.is_empty())
                .unwrap_or(false)
            && record
                .get("description")
                .and_then(|v| v.as_str())
                .map(|s| !s.is_empty())
                .unwrap_or(false)
            && record
                .get("source")
                .and_then(|v| v.as_str())
                .map(|s| !s.is_empty())
                .unwrap_or(false)
    })
}

#[test]
fn g2_false_positive_fixture_samples_do_not_block_route() {
    let temp = tempfile::tempdir().unwrap();
    let target = temp.path();
    setup_project_with_g2_gate_team(target);

    for sample in fixture_samples()
        .into_iter()
        .filter(|sample| !sample.expected_gate)
    {
        let output = run(target, &route_args_for_task(&sample.task), 0);
        let combined = format!("{}{}", stdout(&output), stderr(&output));
        assert!(
            !combined.contains("Owner gate(s) fired")
                && !combined.contains("dispatch refused")
                && !combined.contains("owner_gate_pending"),
            "{} should not block route:\n{}",
            sample.id,
            combined
        );
    }

    assert!(
        gate_records(target).is_empty(),
        "false-positive fixture samples should not write gate ledgers"
    );
}

#[test]
fn g2_true_positive_fixture_samples_block_and_can_be_owner_approved() {
    let temp = tempfile::tempdir().unwrap();
    let target = temp.path();
    setup_project_with_g2_gate_team(target);

    let positives: Vec<_> = fixture_samples()
        .into_iter()
        .filter(|sample| sample.expected_gate)
        .collect();

    for sample in &positives {
        assert_eq!(sample.category, "true_positive");
        let output = run(target, &route_args_for_task(&sample.task), 3);
        let combined = format!("{}{}", stdout(&output), stderr(&output));
        assert!(
            combined.contains("Owner gate(s) fired")
                && combined.contains("[pending]")
                && combined.contains("--owner-approved")
                && combined.contains("Gate ledger:"),
            "{} should block with ledger/reason text:\n{}",
            sample.id,
            combined
        );
        assert!(
            has_gate_record(target, &sample.task, "pending"),
            "{} should write a pending gate ledger record",
            sample.id
        );
    }

    for sample in &positives {
        let output = run(target, &all_owner_approvals_for_task(&sample.task), 0);
        let combined = format!("{}{}", stdout(&output), stderr(&output));
        assert!(
            combined.contains("[approved]") && combined.contains("Dispatch recorded:"),
            "{} should route after explicit owner approval:\n{}",
            sample.id,
            combined
        );
        assert!(
            has_gate_record(target, &sample.task, "approved"),
            "{} should write an approved gate ledger record",
            sample.id
        );
    }
}
