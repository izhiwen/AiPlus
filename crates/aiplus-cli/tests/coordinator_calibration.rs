use serde::Deserialize;
use std::fs;
use std::path::Path;
use std::process::{Command, Output};

#[derive(Debug, Deserialize)]
struct Fixture {
    task: Vec<Case>,
}

#[derive(Debug, Deserialize)]
struct Case {
    input: String,
    expected_complexity: u8,
    expected_risk_max: f32,
    expected_tier: String,
    expected_auto_summoned: Option<Vec<String>>,
    #[allow(dead_code)]
    notes: Option<String>,
}

fn bin() -> &'static str {
    env!("CARGO_BIN_EXE_aiplus")
}

fn run(cwd: &Path, args: &[&str]) -> Output {
    let output = Command::new(bin())
        .args(args)
        .current_dir(cwd)
        .env("HOME", cwd.join("fake-home"))
        .env("CODEX_HOME", cwd.join("fake-codex-home"))
        .env("XDG_CONFIG_HOME", cwd.join("fake-xdg"))
        .env("AIPLUS_SECRET_BROKER_DISABLE_KEYCHAIN", "1")
        .env("AIPLUS_AUTOSUMMON_INTENT_MOCK", "1")
        .env_remove("ANTHROPIC_API_KEY")
        .env_remove("OPENAI_API_KEY")
        .env_remove("BWS_ACCESS_TOKEN")
        .output()
        .expect("run aiplus");
    assert!(
        output.status.success(),
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

fn parse_score_line(output: &str) -> (u8, f32, String) {
    let line = output
        .lines()
        .find(|line| line.starts_with("Adaptive coordinator:"))
        .unwrap_or_else(|| panic!("missing coordinator line:\n{output}"));
    let mut complexity = None;
    let mut risk = None;
    let mut tier = None;
    for part in line.split_whitespace() {
        if let Some(value) = part.strip_prefix("complexity=") {
            complexity = Some(value.parse::<u8>().expect("complexity number"));
        } else if let Some(value) = part.strip_prefix("risk=") {
            risk = Some(value.parse::<f32>().expect("risk number"));
        } else if let Some(value) = part.strip_prefix("tier=") {
            tier = Some(value.to_string());
        }
    }
    (
        complexity.expect("complexity present"),
        risk.expect("risk present"),
        tier.expect("tier present"),
    )
}

fn parse_auto_summoned(output: &str) -> Vec<String> {
    let Some(line) = output
        .lines()
        .find(|line| line.starts_with("Auto-summoned experts: ["))
    else {
        return Vec::new();
    };
    let Some(values) = line
        .strip_prefix("Auto-summoned experts: [")
        .and_then(|value| value.strip_suffix(']'))
    else {
        return Vec::new();
    };
    if values.trim().is_empty() {
        return Vec::new();
    }
    values
        .split(',')
        .map(|value| value.trim().to_string())
        .collect()
}

fn init_git_repo(target: &Path) {
    Command::new("git")
        .args(["init"])
        .current_dir(target)
        .output()
        .expect("git init");
    Command::new("git")
        .args(["config", "user.email", "test@example.com"])
        .current_dir(target)
        .output()
        .expect("git config email");
    Command::new("git")
        .args(["config", "user.name", "Test User"])
        .current_dir(target)
        .output()
        .expect("git config name");
}

#[test]
fn coordinator_scores_match_calibration_fixture() {
    let fixture_text = include_str!("fixtures/coordinator_calibration.toml");
    let fixture: Fixture = toml::from_str(fixture_text).expect("parse fixture");
    assert!(
        fixture.task.len() >= 15,
        "fixture should cover a useful matrix"
    );

    let temp = tempfile::tempdir().unwrap();
    let target = temp.path();
    fs::write(target.join("README.md"), "# Coordinator Calibration\n").unwrap();
    init_git_repo(target);
    run(target, &["install", "codex"]);
    for case in fixture.task {
        let output = stdout(&run(
            target,
            &["agent", "route", "--score-only", case.input.as_str()],
        ));
        let (complexity, risk, tier) = parse_score_line(&output);
        assert_eq!(
            complexity, case.expected_complexity,
            "complexity drift for {:?}\n{output}",
            case.input
        );
        assert!(
            risk <= case.expected_risk_max,
            "risk {risk} exceeded max {} for {:?}\n{output}",
            case.expected_risk_max,
            case.input
        );
        assert_eq!(
            tier, case.expected_tier,
            "tier drift for {:?}\n{output}",
            case.input
        );
        if let Some(expected_auto_summoned) = case.expected_auto_summoned {
            assert_eq!(
                parse_auto_summoned(&output),
                expected_auto_summoned,
                "auto-summon drift for {:?}\n{output}",
                case.input
            );
        }
    }
}
