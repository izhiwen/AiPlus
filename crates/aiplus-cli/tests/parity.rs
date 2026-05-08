use std::collections::hash_map::DefaultHasher;
use std::fs;
use std::hash::{Hash, Hasher};
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

fn bin() -> &'static str {
    env!("CARGO_BIN_EXE_aiplus")
}

fn run(cwd: &Path, args: &[&str], expected: i32) -> Output {
    run_with_path(cwd, args, expected, None)
}

fn run_with_path(cwd: &Path, args: &[&str], expected: i32, path_override: Option<&Path>) -> Output {
    let mut command = Command::new(bin());
    command
        .args(args)
        .current_dir(cwd)
        .env("HOME", cwd.join("fake-home"))
        .env("CODEX_HOME", cwd.join("fake-codex-home"))
        .env("XDG_CONFIG_HOME", cwd.join("fake-xdg"));
    if let Some(path) = path_override {
        command.env("PATH", path);
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

fn digest(dir: &Path) -> String {
    let mut rows = Vec::new();
    walk(dir, dir, &mut rows);
    rows.sort();
    rows.join("\n")
}

fn walk(root: &Path, dir: &Path, rows: &mut Vec<String>) {
    if !dir.exists() {
        return;
    }
    for entry in fs::read_dir(dir).unwrap() {
        let entry = entry.unwrap();
        let path = entry.path();
        if path
            .file_name()
            .unwrap()
            .to_string_lossy()
            .starts_with("fake-")
        {
            continue;
        }
        if path.is_dir() {
            walk(root, &path, rows);
        } else if path.is_file() {
            let mut hasher = DefaultHasher::new();
            fs::read(&path).unwrap().hash(&mut hasher);
            rows.push(format!(
                "{}:{:x}",
                path.strip_prefix(root).unwrap().display(),
                hasher.finish()
            ));
        }
    }
}

#[test]
fn install_status_doctor_update_add_uninstall_codex() {
    let temp = tempfile::tempdir().unwrap();
    let target = temp.path();
    fs::create_dir(target.join("fake-home")).unwrap();
    fs::create_dir(target.join("fake-codex-home")).unwrap();
    fs::create_dir(target.join("fake-xdg")).unwrap();

    let dry_before = digest(target);
    let dry = run(target, &["install", "codex", "--dry-run"], 0);
    let dry_out = stdout(&dry);
    assert!(dry_out.contains("AiPlus install plan for Codex in this project."));
    assert!(dry_out.contains("No files were changed."));
    assert!(dry_out.contains("INSTALL_DRY_RUN=PASS"));
    assert_eq!(digest(target), dry_before);

    let install = run(target, &["install", "codex"], 0);
    let install_out = stdout(&install);
    assert!(install_out.contains("AiPlus installed for Codex in this project."));
    assert!(install_out.contains("AiPlus 刷新"));
    assert!(install_out.contains("aiplus refresh"));
    assert!(install_out.contains("AIPLUS_REFRESH_PROMPT=刷新"));
    assert!(install_out.contains("INSTALL_STATUS=PASS"));
    assert!(target.join(".aiplus/manifest.json").exists());
    assert!(target.join(".codex/compact").exists());
    assert!(target.join("AGENTS.md").exists());
    assert!(!target
        .join(".aiplus/modules/aiplus-auto-compact/core/scripts/compactctl.mjs")
        .exists());
    assert!(!target
        .join(".aiplus/modules/aiplus-auto-compact/package.json")
        .exists());
    assert!(!target
        .join(".aiplus/modules/aiplus-auto-compact/tests/compactctl.acceptance.mjs")
        .exists());

    let status = stdout(&run(target, &["status"], 0));
    assert!(status.contains("runtimeAdapters=[codex]"));
    assert!(status.contains("modules=[auto-compact@0.2.1, auto-team-consultant@0.2.1]"));
    assert!(status.contains("type \"AiPlus 刷新\""));
    assert!(status.contains("STATUS=PASS"));

    let refresh = stdout(&run(target, &["refresh"], 0));
    assert!(refresh.contains("AiPlus refreshed."));
    assert!(refresh.contains("- Auto Compact: installed"));
    assert!(refresh.contains("- Auto Team Consultant: installed"));
    assert!(refresh.contains("- Compact state: present"));
    assert!(refresh.contains("AIPLUS_REFRESH_STATUS=PASS"));
    assert!(!refresh.contains("已刷新 AiPlus。"));

    let refresh_zh = stdout(&run(target, &["refresh", "AiPlus 刷新"], 0));
    assert!(refresh_zh.contains("已刷新 AiPlus。"));
    assert!(refresh_zh.contains("- Auto Compact: 已安装"));
    assert!(refresh_zh.contains("- Auto Team Consultant: 已安装"));
    assert!(refresh_zh.contains("AIPLUS_REFRESH_STATUS=PASS"));

    let installed_agents = fs::read_to_string(target.join(".aiplus/AGENTS.aiplus.md")).unwrap();
    for phrase in [
        "AiPlus 刷新",
        "刷新 AiPlus",
        "aiplus refresh",
        "aiplus status",
        "AiPlus status",
        "继续 AiPlus",
        "resume AiPlus",
        "project-specific refresh",
        "Never bury AiPlus status",
        "AiPlus CLI not found",
        "fix PATH",
        "prepare compact",
        "save progress",
        "aiplus compact prepare",
        "aiplus compact resume",
    ] {
        assert!(installed_agents.contains(phrase), "missing {phrase}");
    }
    assert!(!installed_agents.contains("node .aiplus"));
    assert!(!installed_agents.contains("node <REPO_ROOT>"));
    assert!(!installed_agents.contains("node <PROJECT_ROOT>"));
    assert!(!installed_agents.contains("compactctl.mjs validate"));
    assert!(!installed_agents.contains("compactctl.mjs checkpoint"));
    assert!(!installed_agents.contains("compactctl.mjs resume"));

    let doctor = stdout(&run(target, &["doctor"], 0));
    assert!(doctor.contains("runtimeAdapters=[codex]"));
    assert!(doctor.contains("DOCTOR_STATUS=PASS"));
    assert!(doctor.contains("globalConfig=untouched"));
    assert!(!doctor.contains("compactctl.mjs"));

    let update = stdout(&run(target, &["update"], 0));
    assert!(update.contains("UPDATE_STATUS=PASS"));
    assert!(update.contains("GLOBAL_CONFIG_UNTOUCHED"));

    let add_dry = stdout(&run(
        target,
        &["add", "auto-team-consultant", "--dry-run"],
        0,
    ));
    assert!(add_dry.contains("AiPlus module add plan: auto-team-consultant"));
    assert!(add_dry.contains("No files were changed."));
    assert!(add_dry.contains("ADD_DRY_RUN=PASS"));

    let unknown = run(target, &["update", "auto-router"], 1);
    assert!(stderr(&unknown).contains("MODULE_NOT_AVAILABLE auto-router"));

    let uninstall = stdout(&run(target, &["uninstall", "--dry-run"], 0));
    assert!(uninstall.contains("DRY_RUN_ONLY=YES"));
    assert!(uninstall.contains("NO_FILES_REMOVED=YES"));
    assert!(uninstall.contains("UNINSTALL_DRY_RUN=PASS"));
}

#[test]
fn install_safely_upgrades_existing_aiplus_and_preserves_compact_state() {
    let temp = tempfile::tempdir().unwrap();
    let target = temp.path();
    setup_fake_env(target);

    run(target, &["install", "codex"], 0);
    let managed_schema =
        target.join(".aiplus/modules/aiplus-auto-compact/core/schemas/compact-policy.schema.json");
    fs::write(&managed_schema, b"{\"old\":\"managed file\"}\n").unwrap();
    let checkpoint = target.join(".codex/compact/checkpoints/keep-me.json");
    fs::write(&checkpoint, b"{\"checkpoint\":\"preserve\"}\n").unwrap();
    let user_note = target.join(".aiplus/user-note.txt");
    fs::write(&user_note, b"do not delete\n").unwrap();
    fs::write(
        target.join(".aiplus/AGENTS.aiplus.md"),
        b"old guidance: node .aiplus/modules/aiplus-auto-compact/core/scripts/compactctl.mjs validate\n",
    )
    .unwrap();
    for file in [
        "decision-log.md",
        "agent-state-ledger.md",
        "evidence-ledger.md",
        "compact-policy.json",
    ] {
        let path = target.join(".codex/compact").join(file);
        let next = fs::read_to_string(&path)
            .unwrap()
            .replace("UNKNOWN_PENDING", "APPROVED");
        fs::write(path, next).unwrap();
    }
    fs::write(
        target.join(".codex/compact/current-handoff.md"),
        r#"# Compact Handoff

## Protocol Version

0.1.0

## Last Updated

synthetic-old

## Current Goal

Preserve this old user-authored goal.

## Current Phase

IN_PROGRESS

## Completed Work

- Kept old handoff content.

## Open Blockers

- None.

## Owner Gates

- APPROVED: Synthetic migration test gate.

## Next 3 Actions

1. Continue after migration.

## Do Not Do

- Do not lose user-authored content.

## Recovery Order

1. Resume from this old handoff.
"#,
    )
    .unwrap();

    let upgrade = stdout(&run(target, &["install", "codex"], 0));
    assert!(upgrade.contains("AiPlus upgraded for Codex in this project."));
    assert!(upgrade.contains("UPGRADE_STATUS=PASS"));
    assert!(upgrade.contains("COMPACT_HANDOFF_MIGRATION=APPLIED"));
    assert!(upgrade.contains(".codex/compact/ state was preserved."));
    assert!(checkpoint.exists());
    assert_eq!(fs::read_to_string(&user_note).unwrap(), "do not delete\n");
    assert!(fs::read_to_string(&managed_schema)
        .unwrap()
        .contains("\"$schema\""));
    let agents = fs::read_to_string(target.join(".aiplus/AGENTS.aiplus.md")).unwrap();
    assert!(agents.contains("AiPlus 刷新"));
    assert!(agents.contains("project-specific refresh"));
    assert!(agents.contains("Default English response shape"));
    assert!(agents.contains("AiPlus CLI not found"));
    assert!(agents.contains("prepare compact"));
    assert!(agents.contains("aiplus compact prepare"));
    assert!(!agents.contains("node .aiplus"));
    assert!(!agents.contains("compactctl.mjs validate"));

    let backups = target.join(".aiplus/backups");
    assert!(backups.exists());
    let handoff = fs::read_to_string(target.join(".codex/compact/current-handoff.md")).unwrap();
    assert!(handoff.contains("Preserve this old user-authored goal."));
    assert!(handoff.contains("## Session Role"));
    assert!(handoff.contains("Unknown"));
    assert!(handoff.contains("## Workflow Level"));
    assert!(handoff.contains("## Output Contract"));
    assert!(handoff.contains("AiPlus v0.2.1 compact handoff migration"));
    let validate = stdout(&run(target, &["compact", "validate"], 0));
    assert!(validate.contains("VALIDATION_PASS"));
    let resume = stdout(&run(target, &["compact", "resume"], 0));
    assert!(resume.contains("RESUME_READY"));
    let backup_hit = fs::read_dir(&backups)
        .unwrap()
        .flat_map(|stamp| {
            let stamp = stamp.unwrap().path();
            fs::read_dir(stamp.join(".aiplus/modules/aiplus-auto-compact/core/schemas"))
                .into_iter()
                .flatten()
                .map(|entry| entry.unwrap().path())
                .collect::<Vec<_>>()
        })
        .any(|path| {
            path.file_name().unwrap() == "compact-policy.schema.json"
                && fs::read_to_string(path).unwrap().contains("old")
        });
    assert!(backup_hit);

    let doctor = stdout(&run(target, &["doctor"], 0));
    assert!(doctor.contains("DOCTOR_STATUS=PASS"));
}

#[test]
fn runtime_doctor_modes_and_uninstall_unknown_empty_dir() {
    for runtime in ["claude-code", "opencode", "all"] {
        let temp = tempfile::tempdir().unwrap();
        let target = temp.path();
        fs::create_dir(target.join("fake-home")).unwrap();
        fs::create_dir(target.join("fake-codex-home")).unwrap();
        fs::create_dir(target.join("fake-xdg")).unwrap();
        let out = stdout(&run(target, &["install", runtime], 0));
        assert!(out.contains("INSTALL_STATUS=PASS"));
        let doctor = stdout(&run(target, &["doctor"], 0));
        assert!(doctor.contains("DOCTOR_STATUS=PASS"), "{runtime}: {doctor}");
    }

    let temp = tempfile::tempdir().unwrap();
    let target = temp.path();
    fs::create_dir(target.join("fake-home")).unwrap();
    fs::create_dir(target.join("fake-codex-home")).unwrap();
    fs::create_dir(target.join("fake-xdg")).unwrap();
    run(target, &["install", "codex"], 0);
    fs::create_dir(target.join(".aiplus/user-empty-dir")).unwrap();
    let blocked = run(target, &["uninstall", "--yes"], 1);
    assert!(stderr(&blocked).contains("unknown entries"));
    assert!(target.join(".aiplus/user-empty-dir").exists());
    let forced = stdout(&run(target, &["uninstall", "--yes", "--force"], 0));
    assert!(forced.contains("UNINSTALL_STATUS=PASS"));
    assert!(!target.join(".aiplus").exists());
    assert!(target.join(".codex/compact").exists());
}

#[test]
fn runtime_flags_compact_native_and_dangling_symlink_safety() {
    let temp = tempfile::tempdir().unwrap();
    let target = temp.path();
    fs::create_dir(target.join("fake-home")).unwrap();
    fs::create_dir(target.join("fake-codex-home")).unwrap();
    fs::create_dir(target.join("fake-xdg")).unwrap();

    let by_flag = stdout(&run(target, &["install", "--runtime", "codex"], 0));
    assert!(by_flag.contains("INSTALL_STATUS=PASS"));
    let validate = stdout(&run(target, &["compact", "validate"], 0));
    assert!(validate.contains("VALIDATION_PASS"));
    assert!(validate.contains("COMPACT_RUST_NATIVE_STATUS=PASS"));

    let all_temp = tempfile::tempdir().unwrap();
    let all_target = all_temp.path();
    fs::create_dir(all_target.join("fake-home")).unwrap();
    fs::create_dir(all_target.join("fake-codex-home")).unwrap();
    fs::create_dir(all_target.join("fake-xdg")).unwrap();
    let all = stdout(&run(all_target, &["install", "--all-runtimes"], 0));
    assert!(all.contains("AiPlus installed for Claude Code, Codex, OpenCode in this project."));
    assert!(stdout(&run(all_target, &["doctor"], 0)).contains("DOCTOR_STATUS=PASS"));

    let symlink_temp = tempfile::tempdir().unwrap();
    let symlink_target = symlink_temp.path();
    fs::create_dir(symlink_target.join("fake-home")).unwrap();
    fs::create_dir(symlink_target.join("fake-codex-home")).unwrap();
    fs::create_dir(symlink_target.join("fake-xdg")).unwrap();
    let outside = symlink_temp.path().join("outside-agents.md");
    #[cfg(unix)]
    std::os::unix::fs::symlink(&outside, symlink_target.join("AGENTS.md")).unwrap();
    #[cfg(unix)]
    {
        let blocked = run(symlink_target, &["install", "codex"], 1);
        assert!(stderr(&blocked).contains("ERROR refusing to write through symlink: AGENTS.md"));
        assert!(!outside.exists());

        let compact_temp = tempfile::tempdir().unwrap();
        let compact_target = compact_temp.path();
        fs::create_dir(compact_target.join("fake-home")).unwrap();
        fs::create_dir(compact_target.join("fake-codex-home")).unwrap();
        fs::create_dir(compact_target.join("fake-xdg")).unwrap();
        fs::create_dir_all(compact_target.join(".codex/compact")).unwrap();
        let outside_compact = compact_temp.path().join("outside-handoff.md");
        std::os::unix::fs::symlink(
            &outside_compact,
            compact_target.join(".codex/compact/current-handoff.md"),
        )
        .unwrap();
        let blocked_compact = run(compact_target, &["compact", "init"], 1);
        assert!(stderr(&blocked_compact).contains(
            "ERROR refusing to write through symlink: .codex/compact/current-handoff.md"
        ));
        assert!(!outside_compact.exists());
    }
}

#[test]
fn doctor_manifest_diagnostics_are_row_accurate() {
    let missing = tempfile::tempdir().unwrap();
    setup_fake_env(missing.path());
    let missing_doctor = stdout(&run(missing.path(), &["doctor"], 0));
    assert!(missing_doctor.contains("status=NEEDS_FIX"));
    assert!(missing_doctor.contains("installed=no"));
    assert!(missing_doctor.contains("NEEDS_FIX .aiplus/manifest.json exists"));
    assert!(missing_doctor.contains("NEEDS_FIX manifest parses"));
    assert!(missing_doctor.contains("NEEDS_FIX manifest installer is aiplus"));
    assert!(missing_doctor.contains("DOCTOR_STATUS=NEEDS_FIX"));

    let malformed = tempfile::tempdir().unwrap();
    setup_fake_env(malformed.path());
    fs::create_dir(malformed.path().join(".aiplus")).unwrap();
    fs::write(malformed.path().join(".aiplus/manifest.json"), "{ not json").unwrap();
    let malformed_doctor = stdout(&run(malformed.path(), &["doctor"], 0));
    assert!(malformed_doctor.contains("PASS .aiplus/manifest.json exists"));
    assert!(malformed_doctor.contains("NEEDS_FIX manifest parses"));
    assert!(malformed_doctor.contains("NEEDS_FIX manifest installer is aiplus"));
    assert!(malformed_doctor.contains("DOCTOR_STATUS=NEEDS_FIX"));

    let wrong = tempfile::tempdir().unwrap();
    setup_fake_env(wrong.path());
    fs::create_dir(wrong.path().join(".aiplus")).unwrap();
    fs::write(
        wrong.path().join(".aiplus/manifest.json"),
        r#"{"installer":"other","schemaVersion":"0.1.3","runtimeAdapters":["codex"],"modules":{}}"#,
    )
    .unwrap();
    let wrong_doctor = stdout(&run(wrong.path(), &["doctor"], 0));
    assert!(wrong_doctor.contains("PASS .aiplus/manifest.json exists"));
    assert!(wrong_doctor.contains("PASS manifest parses"));
    assert!(wrong_doctor.contains("NEEDS_FIX manifest installer is aiplus"));
    assert!(wrong_doctor.contains("installed=no"));
    assert!(wrong_doctor.contains("DOCTOR_STATUS=NEEDS_FIX"));
}

#[test]
fn compact_native_validate_checkpoint_resume_and_no_node_path() {
    let temp = tempfile::tempdir().unwrap();
    let target = temp.path();
    setup_fake_env(target);
    run(target, &["install", "codex"], 0);
    assert!(target.join(".codex/compact/current-handoff.md").exists());
    assert!(target.join(".codex/compact/compact-policy.json").exists());

    let no_node_path = make_empty_path();
    let init = stdout(&run_with_path(
        target,
        &["compact", "init"],
        0,
        Some(&no_node_path),
    ));
    assert!(init.contains("INIT_PASS"));
    assert!(init.contains("COMPACT_RUST_NATIVE_STATUS=PASS"));

    let validate = stdout(&run_with_path(
        target,
        &["compact", "validate"],
        0,
        Some(&no_node_path),
    ));
    assert!(validate.contains("VALIDATION_PASS"));
    assert!(validate.contains("COMPACT_RUST_NATIVE_STATUS=PASS"));

    let checkpoint = run_with_path(target, &["compact", "checkpoint"], 2, Some(&no_node_path));
    let checkpoint_out = stdout(&checkpoint);
    assert!(checkpoint_out.contains("UNKNOWN_NEEDS_REVIEW"));
    assert!(checkpoint_out.contains("READINESS_STATE=UNKNOWN_NEEDS_REVIEW"));
    assert!(checkpoint_out.contains("CHECKPOINT_LEVEL=standard"));
    assert!(checkpoint_out.contains("CHECKPOINT_CREATED=.codex/compact/checkpoints/"));
    assert!(checkpoint_out.contains("COMPACT_RUST_NATIVE_STATUS=PASS"));
    let checkpoint_count = fs::read_dir(target.join(".codex/compact/checkpoints"))
        .unwrap()
        .filter(|entry| {
            entry
                .as_ref()
                .unwrap()
                .path()
                .extension()
                .is_some_and(|ext| ext == "json")
        })
        .count();
    assert!(checkpoint_count >= 1);

    let resume = stdout(&run_with_path(
        target,
        &["compact", "resume"],
        0,
        Some(&no_node_path),
    ));
    assert!(resume.contains("RESUME_READY"));
    assert!(resume.contains("latest_checkpoint=.codex/compact/checkpoints/"));
    assert!(resume.contains("session_role=Unknown"));
    assert!(resume.contains("workflow_level=Unknown"));
    assert!(resume.contains("read_only_recovery_guidance=yes"));
    assert!(resume.contains("current_goal="));
    assert!(resume.contains("COMPACT_RUST_NATIVE_STATUS=PASS"));

    let approved = fs::read_to_string(target.join(".codex/compact/current-handoff.md"))
        .unwrap()
        .replace("UNKNOWN_PENDING", "APPROVED");
    fs::write(target.join(".codex/compact/current-handoff.md"), approved).unwrap();
    for file in [
        "decision-log.md",
        "agent-state-ledger.md",
        "evidence-ledger.md",
    ] {
        let next = fs::read_to_string(target.join(".codex/compact").join(file))
            .unwrap()
            .replace("UNKNOWN_PENDING", "APPROVED");
        fs::write(target.join(".codex/compact").join(file), next).unwrap();
    }
    let mut policy = fs::read_to_string(target.join(".codex/compact/compact-policy.json")).unwrap();
    policy = policy.replace(
        "\"status\": \"UNKNOWN_PENDING\"",
        "\"status\": \"APPROVED\"",
    );
    fs::write(target.join(".codex/compact/compact-policy.json"), policy).unwrap();
    let safe_checkpoint = stdout(&run_with_path(
        target,
        &["compact", "checkpoint"],
        0,
        Some(&no_node_path),
    ));
    assert!(safe_checkpoint.contains("SAFE_TO_COMPACT"));
    assert!(safe_checkpoint.contains("READINESS_STATE=READY_TO_COMPACT"));
    assert!(safe_checkpoint.contains("CHECKPOINT_CREATED=.codex/compact/checkpoints/"));

    for level in ["light", "standard", "full"] {
        let out = stdout(&run_with_path(
            target,
            &["compact", "checkpoint", "--level", level],
            0,
            Some(&no_node_path),
        ));
        assert!(out.contains(&format!("CHECKPOINT_LEVEL={level}")));
        assert!(out.contains("READINESS_STATE=READY_TO_COMPACT"));
    }

    let prepare = stdout(&run_with_path(
        target,
        &["compact", "prepare"],
        0,
        Some(&no_node_path),
    ));
    assert!(prepare.contains("COMPACT_PREPARE"));
    assert!(prepare.contains("READINESS_STATE=READY_TO_COMPACT"));
    assert!(prepare.contains("Ready to compact."));
    assert!(prepare.contains("PREPARE_STATUS=PASS"));

    let score = stdout(&run_with_path(
        target,
        &["compact", "score"],
        0,
        Some(&no_node_path),
    ));
    assert!(score.contains("COMPACT_SCORE"));
    assert!(score.contains("COMPACT_PRESSURE=HIGH"));

    fs::write(target.join(".codex/compact/compact-policy.json"), "{ bad").unwrap();
    let bad_policy = run_with_path(target, &["compact", "validate"], 1, Some(&no_node_path));
    assert!(stderr(&bad_policy).contains("compact-policy.json is invalid JSON"));
    assert!(stderr(&bad_policy).contains("VALIDATION_FAIL"));
    let before_blocked_count = fs::read_dir(target.join(".codex/compact/checkpoints"))
        .unwrap()
        .filter(|entry| {
            entry
                .as_ref()
                .unwrap()
                .path()
                .extension()
                .is_some_and(|ext| ext == "json")
        })
        .count();
    let blocked_checkpoint = run_with_path(
        target,
        &["compact", "checkpoint", "--level", "full"],
        1,
        Some(&no_node_path),
    );
    let blocked_out = stdout(&blocked_checkpoint);
    assert!(blocked_out.contains("BLOCKED_DO_NOT_COMPACT"));
    assert!(blocked_out.contains("READINESS_STATE=BLOCKED_BY_OWNER_GATE"));
    assert!(blocked_out.contains("CHECKPOINT_CREATED=none"));
    assert!(blocked_out.contains("checkpoint=none"));
    let after_blocked_count = fs::read_dir(target.join(".codex/compact/checkpoints"))
        .unwrap()
        .filter(|entry| {
            entry
                .as_ref()
                .unwrap()
                .path()
                .extension()
                .is_some_and(|ext| ext == "json")
        })
        .count();
    assert_eq!(before_blocked_count, after_blocked_count);

    run(target, &["compact", "init", "--force"], 0);
    fs::remove_file(target.join(".codex/compact/evidence-ledger.md")).unwrap();
    let missing = run_with_path(target, &["compact", "validate"], 1, Some(&no_node_path));
    assert!(stderr(&missing).contains("evidence-ledger.md is missing"));

    run(target, &["compact", "init", "--force"], 0);
    fs::write(
        target.join(".codex/compact/evidence-ledger.md"),
        "Authorization: Bearer abcdefghijklmnopqrstuvwxyz\n",
    )
    .unwrap();
    let sensitive = run_with_path(target, &["compact", "validate"], 1, Some(&no_node_path));
    assert!(stderr(&sensitive).contains("sensitive pattern detected (authorization header)"));
}

#[test]
fn compact_source_does_not_invoke_node() {
    let manifest_dir = Path::new(env!("CARGO_MANIFEST_DIR"));
    let source = fs::read_to_string(manifest_dir.join("src/main.rs")).unwrap();
    assert!(!source.contains("Command::new(\"node\")"));
    assert!(!source.contains("failed to launch Node compact bridge"));
    assert!(!source.contains("COMPACT_RUST_NATIVE_STATUS=PARTIAL"));
}

fn setup_fake_env(target: &Path) {
    fs::create_dir(target.join("fake-home")).unwrap();
    fs::create_dir(target.join("fake-codex-home")).unwrap();
    fs::create_dir(target.join("fake-xdg")).unwrap();
}

fn make_empty_path() -> PathBuf {
    tempfile::tempdir().unwrap().keep()
}
