use std::collections::hash_map::DefaultHasher;
use std::fs;
use std::hash::{Hash, Hasher};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::time::{SystemTime, UNIX_EPOCH};

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

fn run_with_env(cwd: &Path, args: &[&str], expected: i32, envs: &[(&str, &str)]) -> Output {
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

fn run_with_env_and_path(
    cwd: &Path,
    args: &[&str],
    expected: i32,
    envs: &[(&str, &str)],
    path_override: &Path,
) -> Output {
    let mut command = Command::new(bin());
    command
        .args(args)
        .current_dir(cwd)
        .env("HOME", cwd.join("fake-home"))
        .env("CODEX_HOME", cwd.join("fake-codex-home"))
        .env("XDG_CONFIG_HOME", cwd.join("fake-xdg"))
        .env("PATH", path_override);
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
    assert!(status
        .contains("modules=[agent-memory@0.5.1, auto-compact@0.4.6, auto-team-consultant@0.4.6]"));
    assert!(status.contains("type \"AiPlus 刷新\""));
    assert!(status.contains("agentMemory="));
    assert!(status.contains("memoryRecordsActive="));
    assert!(status.contains("identity=advisor="));
    assert!(status.contains("skillCandidatesTotal="));
    assert!(status.contains("approved_auto=none"));
    assert!(status.contains("profile=aiplus-work-with-zhiwen"));
    assert!(status.contains("secret_values=none"));
    assert!(status.contains("global_agent_config=untouched"));
    assert!(status.contains("STATUS=PASS"));

    let refresh = stdout(&run(target, &["refresh"], 0));
    assert!(refresh.contains("AiPlus refreshed."));
    assert!(refresh.contains("- Auto Compact: installed"));
    assert!(refresh.contains("- Auto Team Consultant: installed"));
    assert!(refresh.contains("- Compact state: present"));
    assert!(refresh.contains("- Agent Memory:"));
    assert!(refresh.contains("- Memory records:"));
    assert!(refresh.contains("- Identity: advisor="));
    assert!(refresh.contains("- Skill candidates:"));
    assert!(refresh.contains("- Profile: aiplus-work-with-zhiwen"));
    assert!(refresh.contains("- Secret values: none"));
    assert!(refresh.contains("- Global agent config: untouched"));
    assert!(refresh.contains("AIPLUS_REFRESH_STATUS=PASS"));
    assert!(!refresh.contains("已刷新 AiPlus。"));

    let refresh_zh = stdout(&run(target, &["refresh", "AiPlus 刷新"], 0));
    assert!(refresh_zh.contains("已刷新 AiPlus。"));
    assert!(refresh_zh.contains("- Auto Compact: 已安装"));
    assert!(refresh_zh.contains("- Auto Team Consultant: 已安装"));
    assert!(refresh_zh.contains("- Agent Memory:"));
    assert!(refresh_zh.contains("- Memory records:"));
    assert!(refresh_zh.contains("- Identity: advisor="));
    assert!(refresh_zh.contains("- Skill candidates:"));
    assert!(refresh_zh.contains("- Profile: aiplus-work-with-zhiwen"));
    assert!(refresh_zh.contains("- Secret values: none"));
    assert!(refresh_zh.contains("- Global agent config: untouched"));
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
        "记住这个偏好",
        "以后都这样",
        "只在这个项目用",
        "忘掉这个",
        "你记住了什么",
        "这次用了哪些记忆",
        "新开顾问",
        "新开 advisor",
        "新开 CEO",
        "把这次经验沉淀成 skill",
        "不要用我的私人记忆",
        "本次忽略我的偏好",
        "Skill Candidate is proposal",
        "Natural language triggers are not hidden",
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
    assert!(doctor.contains("PASS runtimeAdapter codex is supported"));
    assert!(doctor.contains("PASS AGENTS.md contains exactly one AiPlus managed block"));
    assert!(doctor.contains("PASS managed block points to .aiplus/AGENTS.aiplus.md"));
    assert!(doctor.contains("PASS bundled module manifests validate"));
    assert!(doctor.contains("PASS module manifest auto-compact present"));
    assert!(doctor.contains("PASS module manifest auto-team-consultant present"));
    assert!(doctor.contains("PASS module manifest agent-memory present"));
    assert!(doctor.contains("agentMemory="));
    assert!(doctor.contains("memoryRecordsActive="));
    assert!(doctor.contains("identity=advisor="));
    assert!(doctor.contains("skillCandidatesTotal="));
    assert!(doctor.contains("secret_values=none"));
    assert!(!doctor.contains("compactctl.mjs"));

    let update = stdout(&run(target, &["update"], 0));
    assert!(update.contains("UPDATE_STATUS=PASS"));
    assert!(update.contains("GLOBAL_CONFIG_UNTOUCHED"));

    let rollback_dry = stdout(&run(target, &["rollback", "--dry-run"], 0));
    assert!(rollback_dry.contains("AIPLUS_ROLLBACK"));
    assert!(rollback_dry.contains("ROLLBACK_STATUS=DRY_RUN"));

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
    let rollback_plan = fs::read_dir(&backups)
        .unwrap()
        .flat_map(|stamp| {
            let stamp = stamp.unwrap().path();
            fs::read_dir(stamp)
                .into_iter()
                .flatten()
                .map(|entry| entry.unwrap().path())
                .collect::<Vec<_>>()
        })
        .any(|path| path.file_name().unwrap() == "rollback-plan.json");
    assert!(rollback_plan);
    let rollback_dry = stdout(&run(
        target,
        &["rollback", "--id", "latest", "--dry-run"],
        0,
    ));
    assert!(rollback_dry.contains("AIPLUS_ROLLBACK"));
    assert!(rollback_dry.contains("ROLLBACK_STATUS=DRY_RUN"));
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
        if runtime == "claude-code" || runtime == "all" {
            let refresh =
                fs::read_to_string(target.join(".claude/commands/aiplus-refresh.md")).unwrap();
            let advisor =
                fs::read_to_string(target.join(".claude/agents/aiplus-advisor.md")).unwrap();
            assert!(refresh.contains("记住这个"));
            assert!(refresh.contains("这次用了哪些记忆"));
            assert!(refresh.contains("aiplus compact remind"));
            assert!(refresh.contains("HEAVY work every 30 minutes"));
            assert!(advisor.contains("Agent Memory"));
            assert!(advisor.contains("Auto Compact reminder schedule"));
            assert!(advisor.contains("把这次经验沉淀成 skill"));
        }
        if runtime == "opencode" || runtime == "all" {
            let prompt = fs::read_to_string(target.join(".opencode/prompts/aiplus.md")).unwrap();
            let config = fs::read_to_string(target.join(".opencode/opencode.json")).unwrap();
            assert!(prompt.contains("新开 advisor"));
            assert!(prompt.contains("本次忽略我的偏好"));
            assert!(prompt.contains("把这次经验沉淀成 skill"));
            assert!(prompt.contains("aiplus compact remind --event long-session"));
            assert!(prompt.contains("Do not click or call host compact"));
            let parsed: serde_json::Value = serde_json::from_str(&config).unwrap();
            assert_eq!(
                parsed.get("$schema").and_then(|value| value.as_str()),
                Some("https://opencode.ai/config.json")
            );
            assert!(parsed.get("aiplus").is_none());
        }
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
fn doctor_rejects_invalid_opencode_json_and_aiplus_top_level_key() {
    let temp = tempfile::tempdir().unwrap();
    let target = temp.path();
    setup_fake_env(target);
    run(target, &["install", "opencode"], 0);

    fs::write(
        target.join(".opencode/opencode.json"),
        r#"{"$schema": "https://opencode.ai/config.json",}"#,
    )
    .unwrap();
    let invalid = stdout(&run(target, &["doctor"], 0));
    assert!(invalid.contains("NEEDS_FIX .opencode/opencode.json parses as strict JSON"));
    assert!(invalid.contains("DOCTOR_STATUS=NEEDS_FIX"));

    fs::write(
        target.join(".opencode/opencode.json"),
        r#"{"$schema":"https://opencode.ai/config.json","aiplus":{"localOnly":true}}"#,
    )
    .unwrap();
    let unsupported_key = stdout(&run(target, &["doctor"], 0));
    assert!(unsupported_key
        .contains("NEEDS_FIX .opencode/opencode.json has no unsupported AiPlus top-level key"));
    assert!(unsupported_key.contains("DOCTOR_STATUS=NEEDS_FIX"));
}

#[test]
fn install_opencode_preserves_existing_valid_user_config() {
    let temp = tempfile::tempdir().unwrap();
    let target = temp.path();
    setup_fake_env(target);
    fs::create_dir(target.join(".opencode")).unwrap();
    fs::write(
        target.join(".opencode/opencode.json"),
        r#"{"theme":"dark","provider":"local"}"#,
    )
    .unwrap();

    let install = stdout(&run(target, &["install", "opencode"], 0));
    assert!(install.contains("INSTALL_STATUS=PASS"));

    let config = fs::read_to_string(target.join(".opencode/opencode.json")).unwrap();
    let parsed: serde_json::Value = serde_json::from_str(&config).unwrap();
    assert_eq!(
        parsed.get("theme").and_then(|value| value.as_str()),
        Some("dark")
    );
    assert_eq!(
        parsed.get("provider").and_then(|value| value.as_str()),
        Some("local")
    );
    assert!(parsed.get("aiplus").is_none());

    let doctor = stdout(&run(target, &["doctor"], 0));
    assert!(doctor.contains("DOCTOR_STATUS=PASS"), "{doctor}");
}

#[test]
fn doctor_validates_codex_managed_block_and_claude_adapter_content() {
    let codex = tempfile::tempdir().unwrap();
    setup_fake_env(codex.path());
    run(codex.path(), &["install", "codex"], 0);
    let agents = fs::read_to_string(codex.path().join("AGENTS.md")).unwrap();
    fs::write(
        codex.path().join("AGENTS.md"),
        format!("{agents}\n{agents}"),
    )
    .unwrap();
    let duplicate = stdout(&run(codex.path(), &["doctor"], 0));
    assert!(duplicate.contains("NEEDS_FIX AGENTS.md contains exactly one AiPlus managed block"));
    assert!(duplicate.contains("DOCTOR_STATUS=NEEDS_FIX"));

    let claude = tempfile::tempdir().unwrap();
    setup_fake_env(claude.path());
    run(claude.path(), &["install", "claude-code"], 0);
    fs::write(
        claude.path().join(".claude/commands/aiplus-refresh.md"),
        "# Other Command\n",
    )
    .unwrap();
    let bad_claude = stdout(&run(claude.path(), &["doctor"], 0));
    assert!(bad_claude
        .contains("NEEDS_FIX .claude/commands/aiplus-refresh.md is AiPlus refresh command"));
    assert!(bad_claude.contains("DOCTOR_STATUS=NEEDS_FIX"));
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
fn rollback_dry_run_and_restore_only_managed_files() {
    let temp = tempfile::tempdir().unwrap();
    let target = temp.path();
    setup_fake_env(target);
    run(target, &["install", "codex"], 0);

    let original_agents = fs::read_to_string(target.join("AGENTS.md")).unwrap();
    fs::write(target.join("AGENTS.md"), "damaged managed block\n").unwrap();
    let backup_dir = target.join(".aiplus/backups/manual-rollback");
    fs::create_dir_all(&backup_dir).unwrap();
    fs::write(backup_dir.join("AGENTS.md"), original_agents).unwrap();
    fs::write(
        backup_dir.join("rollback-plan.json"),
        r#"{
  "schemaVersion": "0.1.0",
  "id": "manual-rollback",
  "createdAt": "synthetic",
  "entries": [
    {
      "action": "restore",
      "originalPath": "AGENTS.md",
      "backupPath": ".aiplus/backups/manual-rollback/AGENTS.md",
      "managedFile": true
    },
    {
      "action": "restore",
      "originalPath": "user-owned.txt",
      "backupPath": ".aiplus/backups/manual-rollback/user-owned.txt",
      "managedFile": false
    }
  ]
}
"#,
    )
    .unwrap();

    let dry = stdout(&run(
        target,
        &["rollback", "--id", "manual-rollback", "--dry-run"],
        0,
    ));
    assert!(dry.contains("restore AGENTS.md"));
    assert!(dry.contains("skip original=user-owned.txt reason=not_managed_file"));
    assert!(fs::read_to_string(target.join("AGENTS.md"))
        .unwrap()
        .contains("damaged"));

    let applied = stdout(&run(
        target,
        &["rollback", "--id", "manual-rollback", "--yes"],
        0,
    ));
    assert!(applied.contains("ROLLBACK_STATUS=PASS"));
    let restored = fs::read_to_string(target.join("AGENTS.md")).unwrap();
    assert!(restored.contains("BEGIN AIPLUS MANAGED BLOCK"));
    assert!(!target.join("user-owned.txt").exists());
}

#[test]
fn rollback_rejects_symlinked_backup_source() {
    let temp = tempfile::tempdir().unwrap();
    let target = temp.path();
    setup_fake_env(target);
    run(target, &["install", "codex"], 0);

    fs::write(target.join("AGENTS.md"), "damaged managed block\n").unwrap();
    let backup_dir = target.join(".aiplus/backups/symlink-rollback");
    fs::create_dir_all(&backup_dir).unwrap();
    let outside_backup = target.join("outside-backup.txt");
    fs::write(&outside_backup, "malicious backup source\n").unwrap();
    #[cfg(unix)]
    std::os::unix::fs::symlink(&outside_backup, backup_dir.join("AGENTS.symlink")).unwrap();
    #[cfg(not(unix))]
    fs::write(
        backup_dir.join("AGENTS.symlink"),
        "not a symlink on this platform\n",
    )
    .unwrap();
    fs::write(
        backup_dir.join("rollback-plan.json"),
        r#"{
  "schemaVersion": "0.1.0",
  "id": "symlink-rollback",
  "createdAt": "synthetic",
  "entries": [
    {
      "action": "restore",
      "originalPath": "AGENTS.md",
      "backupPath": ".aiplus/backups/symlink-rollback/AGENTS.symlink",
      "managedFile": true
    }
  ]
}
"#,
    )
    .unwrap();

    let applied = stdout(&run(
        target,
        &["rollback", "--id", "symlink-rollback", "--yes"],
        0,
    ));
    #[cfg(unix)]
    {
        assert!(applied.contains("skip original=AGENTS.md reason=backup_symlink"));
        assert!(applied.contains("skipped=1"));
        assert!(fs::read_to_string(target.join("AGENTS.md"))
            .unwrap()
            .contains("damaged managed block"));
    }
}

#[test]
fn generated_rollback_plan_keeps_multiple_backup_entries() {
    let temp = tempfile::tempdir().unwrap();
    let target = temp.path();
    setup_fake_env(target);
    run(target, &["install", "codex"], 0);

    fs::write(target.join("AGENTS.md"), "changed managed block\n").unwrap();
    fs::write(
        target.join(".aiplus/AGENTS.aiplus.md"),
        "changed aiplus guidance\n",
    )
    .unwrap();
    let reinstall = stdout(&run(target, &["install", "codex"], 0));
    assert!(reinstall.contains("UPGRADE_STATUS=PASS"));

    let rollback_plan = find_latest_rollback_plan(target);
    let text = fs::read_to_string(rollback_plan).unwrap();
    assert!(text.contains("\"originalPath\": \"AGENTS.md\""));
    assert!(text.contains("\"originalPath\": \".aiplus/AGENTS.aiplus.md\""));

    let rollback = stdout(&run(target, &["rollback", "--id", "latest", "--yes"], 0));
    assert!(rollback.contains("ROLLBACK_STATUS=PASS"));
    assert!(fs::read_to_string(target.join("AGENTS.md"))
        .unwrap()
        .contains("changed managed block"));
    assert!(fs::read_to_string(target.join(".aiplus/AGENTS.aiplus.md"))
        .unwrap()
        .contains("changed aiplus guidance"));
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

    let future = tempfile::tempdir().unwrap();
    setup_fake_env(future.path());
    fs::create_dir(future.path().join(".aiplus")).unwrap();
    fs::write(
        future.path().join(".aiplus/manifest.json"),
        r#"{"installer":"aiplus","schemaVersion":"999.0.0","runtimeAdapters":["codex"],"modules":{}}"#,
    )
    .unwrap();
    let future_doctor = stdout(&run(future.path(), &["doctor"], 0));
    assert!(future_doctor.contains("PASS manifest installer is aiplus"));
    assert!(future_doctor.contains("NEEDS_FIX manifest schemaVersion supported"));
    assert!(future_doctor.contains("installed=no"));
    assert!(future_doctor.contains("DOCTOR_STATUS=NEEDS_FIX"));

    let unknown_runtime = tempfile::tempdir().unwrap();
    setup_fake_env(unknown_runtime.path());
    fs::create_dir(unknown_runtime.path().join(".aiplus")).unwrap();
    fs::write(
        unknown_runtime.path().join(".aiplus/manifest.json"),
        r#"{"installer":"aiplus","schemaVersion":"0.5.1","runtimeAdapters":["opencode","bogus"],"modules":{}}"#,
    )
    .unwrap();
    let unknown_runtime_doctor = stdout(&run(unknown_runtime.path(), &["doctor"], 0));
    assert!(unknown_runtime_doctor.contains("PASS runtimeAdapter opencode is supported"));
    assert!(unknown_runtime_doctor.contains("NEEDS_FIX runtimeAdapter bogus is supported"));
    assert!(unknown_runtime_doctor.contains("DOCTOR_STATUS=NEEDS_FIX"));
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
fn compact_remind_decisions_snooze_handoff_json_and_guidance() {
    let temp = tempfile::tempdir().unwrap();
    let target = temp.path();
    setup_fake_env(target);
    run(target, &["install", "codex"], 0);

    let template = run(target, &["compact", "remind"], 2);
    let template_out = stdout(&template);
    assert!(template_out.contains("COMPACT_REMINDER"));
    assert!(template_out.contains("REMINDER_DECISION=wait"));
    assert!(template_out.contains("REMINDER_LEVEL=safety_block"));
    assert!(template_out.contains("HANDOFF_STATE=template_only"));
    assert!(template_out.contains("SECRET_VALUES_PRINTED=no"));

    approve_compact_state(target);
    make_compact_handoff_current(target, "PASS", now_unix_millis_marker());
    let prepare_only = stdout(&run(
        target,
        &["compact", "remind", "--event", "phase-end"],
        0,
    ));
    assert!(prepare_only.contains("REMINDER_DECISION=prepare_only"));
    assert!(prepare_only.contains("REMINDER_LEVEL=soft"));
    assert!(prepare_only.contains("HANDOFF_STATE=current"));
    assert!(prepare_only.contains("RECOVERY_CONFIDENCE=medium"));
    assert!(prepare_only.contains("LAST_CHECKPOINT_AGE=missing"));
    assert!(prepare_only.contains("ESTIMATED_TOKENS_SAVED="));
    assert!(prepare_only.contains("ESTIMATED_USD_SAVED="));
    assert!(prepare_only.contains("SECRET_VALUES_PRINTED=no"));

    let prepare = stdout(&run(target, &["compact", "prepare"], 0));
    assert!(prepare.contains("CHECKPOINT_CREATED=.codex/compact/checkpoints/"));

    let ready = stdout(&run(
        target,
        &["compact", "remind", "--event", "before-review"],
        0,
    ));
    assert!(ready.contains("REMINDER_DECISION=remind_now"));
    assert!(ready.contains("REMINDER_LEVEL=ready"));
    assert!(ready.contains("RECOVERY_CONFIDENCE=high"));
    assert!(ready.contains("MANUAL_COMPACT_RECOMMENDED=yes"));
    assert!(ready.contains("HOST_COMPACT_TRIGGERED=no"));

    let json = stdout(&run(target, &["compact", "remind", "--json"], 0));
    let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();
    assert_eq!(
        parsed
            .get("reminderDecision")
            .and_then(|value| value.as_str()),
        Some("remind_now")
    );
    assert_eq!(
        parsed
            .get("secretValuesPrinted")
            .and_then(|value| value.as_str()),
        Some("no")
    );
    assert_eq!(
        parsed
            .get("hostCompactTriggered")
            .and_then(|value| value.as_bool()),
        Some(false)
    );

    let snooze_set = stdout(&run(target, &["compact", "remind", "--snooze", "30m"], 0));
    assert!(snooze_set.contains("SNOOZE_STATUS=set"));
    assert!(snooze_set.contains("REMINDER_DECISION=remind_now"));
    let snoozed = stdout(&run(
        target,
        &["compact", "remind", "--event", "long-session"],
        2,
    ));
    assert!(snoozed.contains("SNOOZE_STATUS=active"));
    assert!(snoozed.contains("REMINDER_DECISION=wait"));
    let snooze_clear = stdout(&run(target, &["compact", "remind", "--clear-snooze"], 0));
    assert!(snooze_clear.contains("SNOOZE_STATUS=cleared"));

    make_compact_handoff_current(target, "ACTIVE_WORK", now_unix_millis_marker());
    let active = stdout(&run(
        target,
        &["compact", "remind", "--event", "long-session"],
        2,
    ));
    assert!(active.contains("READINESS_STATE=NOT_RECOMMENDED_DURING_ACTIVE_WORK"));
    assert!(active.contains("REMINDER_DECISION=wait"));
    assert!(active.contains("REMINDER_LEVEL=safety_block"));

    make_compact_handoff_current(target, "PASS", "unix-1ms".to_string());
    let stale = stdout(&run(
        target,
        &["compact", "remind", "--event", "phase-end"],
        2,
    ));
    assert!(stale.contains("HANDOFF_STATE=stale"));
    assert!(stale.contains("REMINDER_DECISION=wait"));

    fs::write(target.join(".codex/compact/compact-policy.json"), "{ bad").unwrap();
    let blocked = stdout(&run(target, &["compact", "remind"], 1));
    assert!(blocked.contains("REMINDER_DECISION=blocked"));
    assert!(blocked.contains("REMINDER_LEVEL=safety_block"));

    let agents = fs::read_to_string(target.join(".aiplus/AGENTS.aiplus.md")).unwrap();
    assert!(agents.contains("aiplus compact remind --event long-session"));
    assert!(agents.contains("HEAVY task"));
    assert!(agents.contains("REMINDER_DECISION=remind_now"));
    assert!(agents.contains("aiplus compact savings"));
}

#[test]
fn compact_remind_is_offline_no_network_no_cache_write() {
    let temp = tempfile::tempdir().unwrap();
    let target = temp.path();
    setup_fake_env(target);
    run(target, &["install", "codex"], 0);
    approve_compact_state(target);
    make_compact_handoff_current(target, "PASS", now_unix_millis_marker());

    let no_network_path = make_empty_path();
    let cache_dir = target.join("fake-home/.cache/aiplus");
    fs::create_dir_all(&cache_dir).unwrap();

    let remind = stdout(&run_with_env(
        target,
        &["compact", "remind", "--event", "before-review"],
        0,
        &[
            ("PATH", no_network_path.to_str().unwrap()),
            ("HOME", target.join("fake-home").to_str().unwrap()),
        ],
    ));
    assert!(
        remind.contains("REMINDER_DECISION=prepare_only")
            || remind.contains("REMINDER_DECISION=remind_now"),
        "remind should return safe decision without network: got {remind}"
    );
    assert!(remind.contains("SECRET_VALUES_PRINTED=no"));

    let cache_after = fs::read_dir(&cache_dir)
        .unwrap()
        .filter(|e| e.as_ref().unwrap().path().is_file())
        .count();
    assert_eq!(
        cache_after, 0,
        "remind must not write user-level pricing cache"
    );
}

#[test]
fn compact_remind_reaches_remind_now_with_current_handoff_and_checkpoint() {
    let temp = tempfile::tempdir().unwrap();
    let target = temp.path();
    setup_fake_env(target);
    run(target, &["install", "codex"], 0);
    approve_compact_state(target);
    make_compact_handoff_current(target, "PASS", now_unix_millis_marker());

    let prepare = stdout(&run(target, &["compact", "prepare"], 0));
    assert!(prepare.contains("CHECKPOINT_CREATED=.codex/compact/checkpoints/"));

    let remind = stdout(&run(
        target,
        &["compact", "remind", "--event", "before-review"],
        0,
    ));
    assert!(remind.contains("REMINDER_DECISION=remind_now"));
    assert!(remind.contains("REMINDER_LEVEL=ready"));
    assert!(remind.contains("MANUAL_COMPACT_RECOMMENDED=yes"));
    assert!(remind.contains("HANDOFF_STATE=current"));
    assert!(remind.contains("RECOVERY_CONFIDENCE=high"));
    assert!(remind.contains("SECRET_VALUES_PRINTED=no"));
    assert!(remind.contains("HOST_COMPACT_TRIGGERED=no"));

    let json = stdout(&run(target, &["compact", "remind", "--json"], 0));
    let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();
    assert_eq!(
        parsed.get("reminderDecision").and_then(|v| v.as_str()),
        Some("remind_now")
    );
    assert_eq!(
        parsed.get("reminderLevel").and_then(|v| v.as_str()),
        Some("ready")
    );
    assert_eq!(
        parsed
            .get("manualCompactRecommended")
            .and_then(|v| v.as_bool()),
        Some(true)
    );
    assert_eq!(
        parsed.get("handoffState").and_then(|v| v.as_str()),
        Some("current")
    );
    assert_eq!(
        parsed.get("recoveryConfidence").and_then(|v| v.as_str()),
        Some("high")
    );
    assert_eq!(
        parsed.get("secretValuesPrinted").and_then(|v| v.as_str()),
        Some("no")
    );
    assert_eq!(
        parsed.get("hostCompactTriggered").and_then(|v| v.as_bool()),
        Some(false)
    );
}

#[test]
fn compact_watch_once_and_json_output() {
    let temp = tempfile::tempdir().unwrap();
    let target = temp.path();
    setup_fake_env(target);
    run(target, &["install", "codex"], 0);
    approve_compact_state(target);
    make_compact_handoff_current(target, "PASS", now_unix_millis_marker());

    let watch_once = stdout(&run(target, &["compact", "watch", "--once"], 0));
    assert!(watch_once.contains("COMPACT_WATCH"));
    assert!(watch_once.contains("WATCH_MODE=once"));
    assert!(watch_once.contains("WATCH_ITERATION=1"));
    assert!(watch_once.contains("HOST_COMPACT_TRIGGERED=no"));
    assert!(watch_once.contains("SECRET_VALUES_PRINTED=no"));
    assert!(watch_once.contains("RAW_TRANSCRIPT_CAPTURED=no"));

    let watch_json = stdout(&run(target, &["compact", "watch", "--once", "--json"], 0));
    let last_json = watch_json.lines().last().unwrap_or("{}");
    let parsed: serde_json::Value = serde_json::from_str(last_json).unwrap();
    assert_eq!(parsed.get("status").and_then(|v| v.as_str()), Some("PASS"));
    assert_eq!(
        parsed.get("watchMode").and_then(|v| v.as_str()),
        Some("once")
    );
    assert_eq!(parsed.get("iteration").and_then(|v| v.as_u64()), Some(1));
    assert_eq!(
        parsed.get("hostCompactTriggered").and_then(|v| v.as_bool()),
        Some(false)
    );
    assert_eq!(
        parsed.get("secretValuesPrinted").and_then(|v| v.as_bool()),
        Some(false)
    );
    assert_eq!(
        parsed
            .get("rawTranscriptCaptured")
            .and_then(|v| v.as_bool()),
        Some(false)
    );
}

#[test]
fn compact_watch_creates_reminder_state() {
    let temp = tempfile::tempdir().unwrap();
    let target = temp.path();
    setup_fake_env(target);
    run(target, &["install", "codex"], 0);
    approve_compact_state(target);
    make_compact_handoff_current(target, "PASS", now_unix_millis_marker());

    run(target, &["compact", "watch", "--once"], 0);

    let state_path = target.join(".codex/compact/reminder-state.json");
    assert!(state_path.exists(), "reminder-state.json should be created");
    let state_text = fs::read_to_string(&state_path).unwrap();
    let state: serde_json::Value = serde_json::from_str(&state_text).unwrap();
    assert_eq!(state.get("schemaVersion").and_then(|v| v.as_u64()), Some(1));
    assert_eq!(
        state.get("manualCompactOnly").and_then(|v| v.as_bool()),
        Some(true)
    );
    assert_eq!(
        state.get("hostCompactTriggered").and_then(|v| v.as_bool()),
        Some(false)
    );
    assert!(state.get("watchCount").and_then(|v| v.as_u64()).unwrap() >= 1);
}

#[test]
fn compact_prepare_creates_context_capsule() {
    let temp = tempfile::tempdir().unwrap();
    let target = temp.path();
    setup_fake_env(target);
    run(target, &["install", "codex"], 0);
    approve_compact_state(target);
    make_compact_handoff_current(target, "PASS", now_unix_millis_marker());

    let prepare = stdout(&run(target, &["compact", "prepare"], 0));
    assert!(prepare.contains("CONTEXT_CAPSULE_CREATED=.codex/compact/context-capsule.json"));

    let capsule_path = target.join(".codex/compact/context-capsule.json");
    assert!(
        capsule_path.exists(),
        "context-capsule.json should be created"
    );
    let capsule_text = fs::read_to_string(&capsule_path).unwrap();
    let capsule: serde_json::Value = serde_json::from_str(&capsule_text).unwrap();
    assert_eq!(
        capsule.get("schemaVersion").and_then(|v| v.as_u64()),
        Some(1)
    );
    assert!(capsule.get("projectId").is_some());
    assert!(!capsule
        .get("redaction")
        .and_then(|r| r.get("secretValuesPrinted"))
        .and_then(|v| v.as_bool())
        .unwrap_or(true));
    assert!(!capsule
        .get("redaction")
        .and_then(|r| r.get("rawTranscriptCaptured"))
        .and_then(|v| v.as_bool())
        .unwrap_or(true));
    assert!(capsule
        .get("checksums")
        .and_then(|c| c.as_object())
        .map(|o| !o.is_empty())
        .unwrap_or(false));
}

#[test]
fn compact_resume_reads_valid_capsule() {
    let temp = tempfile::tempdir().unwrap();
    let target = temp.path();
    setup_fake_env(target);
    run(target, &["install", "codex"], 0);
    approve_compact_state(target);
    make_compact_handoff_current(target, "PASS", now_unix_millis_marker());

    // Run prepare to create capsule
    let prepare = stdout(&run(target, &["compact", "prepare"], 0));
    assert!(prepare.contains("CONTEXT_CAPSULE_CREATED=.codex/compact/context-capsule.json"));

    // Now resume should read the capsule
    let resume = stdout(&run(target, &["compact", "resume"], 0));
    assert!(
        resume.contains("CAPSULE_LOADED=yes"),
        "resume should report CAPSULE_LOADED=yes\n{resume}"
    );
    assert!(
        resume.contains("CAPSULE_STATUS=current"),
        "resume should report CAPSULE_STATUS=current\n{resume}"
    );
    assert!(
        resume.contains("RESUME_READY"),
        "resume should report RESUME_READY\n{resume}"
    );
    assert!(
        resume.contains("read_only_recovery_guidance=yes"),
        "resume should report read_only_recovery_guidance\n{resume}"
    );
}

#[test]
fn compact_resume_falls_back_to_handoff_when_capsule_missing() {
    let temp = tempfile::tempdir().unwrap();
    let target = temp.path();
    setup_fake_env(target);
    run(target, &["install", "codex"], 0);
    approve_compact_state(target);
    make_compact_handoff_current(target, "PASS", now_unix_millis_marker());

    // Ensure capsule does not exist
    let capsule_path = target.join(".codex/compact/context-capsule.json");
    if capsule_path.exists() {
        fs::remove_file(&capsule_path).unwrap();
    }

    let resume = stdout(&run(target, &["compact", "resume"], 0));
    assert!(
        resume.contains("CAPSULE_LOADED=no"),
        "resume should report CAPSULE_LOADED=no\n{resume}"
    );
    assert!(
        resume.contains("CAPSULE_STATUS=missing"),
        "resume should report CAPSULE_STATUS=missing\n{resume}"
    );
    assert!(
        resume.contains("RESUME_READY"),
        "resume should still report RESUME_READY via handoff fallback\n{resume}"
    );
}

#[test]
fn compact_resume_rejects_malformed_capsule() {
    let temp = tempfile::tempdir().unwrap();
    let target = temp.path();
    setup_fake_env(target);
    run(target, &["install", "codex"], 0);
    approve_compact_state(target);
    make_compact_handoff_current(target, "PASS", now_unix_millis_marker());

    // Write malformed capsule
    let capsule_path = target.join(".codex/compact/context-capsule.json");
    fs::write(&capsule_path, "{ not valid json").unwrap();

    let resume = stdout(&run(target, &["compact", "resume"], 0));
    assert!(
        resume.contains("CAPSULE_LOADED=no"),
        "resume should report CAPSULE_LOADED=no\n{resume}"
    );
    assert!(
        resume.contains("CAPSULE_STATUS=malformed"),
        "resume should report CAPSULE_STATUS=malformed\n{resume}"
    );
    assert!(
        resume.contains("RESUME_READY"),
        "resume should still fall back to handoff\n{resume}"
    );
}

#[test]
fn compact_resume_rejects_checksum_mismatch() {
    let temp = tempfile::tempdir().unwrap();
    let target = temp.path();
    setup_fake_env(target);
    run(target, &["install", "codex"], 0);
    approve_compact_state(target);
    make_compact_handoff_current(target, "PASS", now_unix_millis_marker());

    // Run prepare to create a valid capsule
    run(target, &["compact", "prepare"], 0);

    // Corrupt the capsule objective so checksum no longer matches
    let capsule_path = target.join(".codex/compact/context-capsule.json");
    let capsule_text = fs::read_to_string(&capsule_path).unwrap();
    let mut capsule: serde_json::Value = serde_json::from_str(&capsule_text).unwrap();
    if let Some(obj) = capsule.get_mut("objective") {
        *obj = serde_json::json!("tampered objective");
    }
    fs::write(
        &capsule_path,
        serde_json::to_string_pretty(&capsule).unwrap(),
    )
    .unwrap();

    let resume = stdout(&run(target, &["compact", "resume"], 0));
    assert!(
        resume.contains("CAPSULE_LOADED=no"),
        "resume should report CAPSULE_LOADED=no after checksum mismatch\n{resume}"
    );
    assert!(
        resume.contains("CAPSULE_STATUS=checksum_mismatch"),
        "resume should report CAPSULE_STATUS=checksum_mismatch\n{resume}"
    );
    assert!(
        resume.contains("RESUME_READY"),
        "resume should still fall back to handoff\n{resume}"
    );
}

#[test]
fn decision_ledger_extraction_normal_table() {
    let temp = tempfile::tempdir().unwrap();
    let target = temp.path();
    setup_fake_env(target);
    run(target, &["install", "codex"], 0);
    approve_compact_state(target);
    make_compact_handoff_current(target, "PASS", now_unix_millis_marker());

    // Replace decisions section in existing decision log
    let decision_log = target.join(".codex/compact/decision-log.md");
    let log_text = fs::read_to_string(&decision_log).unwrap();
    let new_decisions = "| ID | Status | Decision | Rationale | Evidence |\n| --- | --- | --- | --- | --- |\n| DEC-001 | DECIDED | Use Rust for CLI. | Performance and safety. | EVD-001 |\n| DEC-002 | PROVISIONAL | Evaluate Go for future tools. | Team familiarity. | EVD-002 |\n\nAllowed status values: DECIDED, PROVISIONAL, REVERSED, NEEDS_VERIFICATION.";
    let updated = replace_section_body(&log_text, "Decisions", new_decisions);
    fs::write(&decision_log, updated).unwrap();

    run(target, &["compact", "prepare"], 0);
    let capsule_path = target.join(".codex/compact/context-capsule.json");
    assert!(capsule_path.exists());
    let capsule_text = fs::read_to_string(&capsule_path).unwrap();
    let capsule: serde_json::Value = serde_json::from_str(&capsule_text).unwrap();
    let decisions = capsule
        .get("decisions")
        .and_then(|d| d.as_array())
        .expect("decisions array should exist");
    assert!(
        decisions.len() >= 2,
        "expected at least 2 decisions, got {}\n{capsule_text}",
        decisions.len()
    );
    let ids: Vec<String> = decisions
        .iter()
        .filter_map(|d| d.get("id").and_then(|v| v.as_str()).map(String::from))
        .collect();
    assert!(ids.contains(&"DEC-001".to_string()));
    assert!(ids.contains(&"DEC-002".to_string()));
}

#[test]
fn decision_ledger_extraction_malformed_log() {
    let temp = tempfile::tempdir().unwrap();
    let target = temp.path();
    setup_fake_env(target);
    run(target, &["install", "codex"], 0);
    approve_compact_state(target);
    make_compact_handoff_current(target, "PASS", now_unix_millis_marker());

    // Malformed table: rows that don't start with '|' should be skipped
    let decision_log = target.join(".codex/compact/decision-log.md");
    let log_text = fs::read_to_string(&decision_log).unwrap();
    let new_decisions = "DEC-001 DECIDED Use Rust for CLI. Performance. EVD-001\n\nAllowed status values: DECIDED, PROVISIONAL, REVERSED, NEEDS_VERIFICATION.";
    let updated = replace_section_body(&log_text, "Decisions", new_decisions);
    fs::write(&decision_log, updated).unwrap();

    run(target, &["compact", "prepare"], 0);
    let capsule_path = target.join(".codex/compact/context-capsule.json");
    let capsule_text = fs::read_to_string(&capsule_path).unwrap();
    let capsule: serde_json::Value = serde_json::from_str(&capsule_text).unwrap();
    let decisions = capsule
        .get("decisions")
        .and_then(|d| d.as_array())
        .expect("decisions array should exist");
    assert_eq!(decisions.len(), 0, "non-table rows should be skipped");
}

#[test]
fn decision_ledger_extraction_skips_sensitive_entries() {
    let temp = tempfile::tempdir().unwrap();
    let target = temp.path();
    setup_fake_env(target);
    run(target, &["install", "codex"], 0);
    approve_compact_state(target);
    make_compact_handoff_current(target, "PASS", now_unix_millis_marker());

    // Replace decisions section with sensitive entries
    let decision_log = target.join(".codex/compact/decision-log.md");
    let log_text = fs::read_to_string(&decision_log).unwrap();
    let new_decisions = "| ID | Status | Decision | Rationale | Evidence |\n| --- | --- | --- | --- | --- |\n| DEC-001 | DECIDED | Use Rust for CLI. | Performance. | EVD-001 |\n| DEC-002 | DECIDED | api_key is secret123. | Security. | EVD-002 |\n| DEC-003 | DECIDED | Store raw transcript. | Logging. | EVD-003 |\n\nAllowed status values: DECIDED, PROVISIONAL, REVERSED, NEEDS_VERIFICATION.";
    let updated = replace_section_body(&log_text, "Decisions", new_decisions);
    fs::write(&decision_log, updated).unwrap();

    // Sensitive patterns block prepare (exit code 1), but capsule is still created
    let prepare = stdout(&run(target, &["compact", "prepare"], 1));
    assert!(prepare.contains("BLOCKED_BY_OWNER_GATE"));
    assert!(prepare.contains("CONTEXT_CAPSULE_CREATED"));
    let capsule_path = target.join(".codex/compact/context-capsule.json");
    let capsule_text = fs::read_to_string(&capsule_path).unwrap();
    let capsule: serde_json::Value = serde_json::from_str(&capsule_text).unwrap();
    let decisions = capsule
        .get("decisions")
        .and_then(|d| d.as_array())
        .expect("decisions array should exist");
    let ids: Vec<String> = decisions
        .iter()
        .filter_map(|d| d.get("id").and_then(|v| v.as_str()).map(String::from))
        .collect();
    assert!(
        ids.contains(&"DEC-001".to_string()),
        "DEC-001 should be present"
    );
    assert!(
        !ids.contains(&"DEC-002".to_string()),
        "DEC-002 with api_key should be skipped"
    );
    assert!(
        !ids.contains(&"DEC-003".to_string()),
        "DEC-003 with raw transcript should be skipped"
    );
}

#[test]
fn decision_ledger_extraction_empty_log() {
    let temp = tempfile::tempdir().unwrap();
    let target = temp.path();
    setup_fake_env(target);
    run(target, &["install", "codex"], 0);
    approve_compact_state(target);
    make_compact_handoff_current(target, "PASS", now_unix_millis_marker());

    // Empty decisions section
    let decision_log = target.join(".codex/compact/decision-log.md");
    let log_text = fs::read_to_string(&decision_log).unwrap();
    let updated = replace_section_body(&log_text, "Decisions", "No decisions yet.\n\nAllowed status values: DECIDED, PROVISIONAL, REVERSED, NEEDS_VERIFICATION.");
    fs::write(&decision_log, updated).unwrap();

    run(target, &["compact", "prepare"], 0);
    let capsule_path = target.join(".codex/compact/context-capsule.json");
    let capsule_text = fs::read_to_string(&capsule_path).unwrap();
    let capsule: serde_json::Value = serde_json::from_str(&capsule_text).unwrap();
    let decisions = capsule
        .get("decisions")
        .and_then(|d| d.as_array())
        .expect("decisions array should exist");
    assert_eq!(decisions.len(), 0, "empty log should yield 0 decisions");
}

#[test]
fn compact_savings_uses_cache_and_handles_unknown_model_without_price_input() {
    let temp = tempfile::tempdir().unwrap();
    let target = temp.path();
    setup_fake_env(target);
    run(target, &["install", "codex"], 0);
    approve_compact_state(target);
    seed_pricing_cache(target, &[("known-model", 1.0)]);

    let no_network_path = make_empty_path();
    let prepare = stdout(&run_with_env(
        target,
        &["compact", "prepare"],
        0,
        &[
            ("AIPLUS_MODEL", "known-model"),
            ("PATH", no_network_path.to_str().unwrap()),
        ],
    ));
    assert!(prepare.contains("PREPARE_STATUS=PASS"));
    let projected = stdout(&run_with_path(
        target,
        &["compact", "savings"],
        0,
        Some(&no_network_path),
    ));
    assert!(projected.contains("no completed compact cycle yet"));
    run_with_env(
        target,
        &["compact", "resume"],
        0,
        &[
            ("AIPLUS_MODEL", "known-model"),
            ("PATH", no_network_path.to_str().unwrap()),
        ],
    );
    let savings = stdout(&run_with_path(
        target,
        &["compact", "savings"],
        0,
        Some(&no_network_path),
    ));
    assert!(savings.contains("Compact savings estimate"));
    assert!(savings.contains("This compact:"));
    assert!(savings.contains("All time:"));
    assert!(savings.contains("Tokens saved: ~"));
    assert!(savings.contains("Token reduction: ~"));
    assert!(savings.contains("Estimated cost saved: ~$"));
    assert!(savings.contains("billing_data=no"));
    assert!(savings.contains("Estimate only, not billing data."));
    let ledger = fs::read_to_string(target.join(".codex/compact/savings-ledger.jsonl")).unwrap();
    assert!(ledger.contains(r#""pricingStatus":"matched""#));
    assert!(ledger.contains(r#""billingData":false"#));
    assert!(ledger.contains("no prompt text"));
    assert!(ledger.contains("raw checkpoint text"));

    let json = stdout(&run_with_path(
        target,
        &["compact", "savings", "--json"],
        0,
        Some(&no_network_path),
    ));
    let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();
    assert_eq!(parsed["status"], "PASS");
    assert_eq!(parsed["billingData"], false);
    assert!(parsed["allTime"]["estimatedTokensSaved"].as_u64().unwrap() > 0);
    assert_eq!(parsed["allTime"]["completedCycles"].as_u64().unwrap(), 1);
    assert_eq!(
        parsed["eventSemantics"]["candidate"],
        "checkpoint events do not count toward completed savings totals"
    );
    run_with_path(target, &["compact", "resume"], 0, Some(&no_network_path));
    let repeated_json = stdout(&run_with_path(
        target,
        &["compact", "savings", "--json"],
        0,
        Some(&no_network_path),
    ));
    let repeated: serde_json::Value = serde_json::from_str(&repeated_json).unwrap();
    assert_eq!(repeated["allTime"]["completedCycles"].as_u64().unwrap(), 1);

    let unknown = tempfile::tempdir().unwrap();
    let unknown_target = unknown.path();
    setup_fake_env(unknown_target);
    run(unknown_target, &["install", "codex"], 0);
    approve_compact_state(unknown_target);
    seed_pricing_cache(unknown_target, &[("known-model", 1.0)]);
    run_with_env(
        unknown_target,
        &["compact", "prepare"],
        0,
        &[("AIPLUS_MODEL", "gpt-new-model")],
    );
    run(unknown_target, &["compact", "resume"], 0);
    let unknown_savings = stdout(&run(unknown_target, &["compact", "savings"], 0));
    assert!(unknown_savings.contains("Estimated cost saved: unavailable"));
    assert!(unknown_savings.contains("pricing for detected model is not available"));
    assert!(unknown_savings.contains("Tokens saved: ~"));
    assert!(unknown_savings.contains("Token reduction: ~"));

    let missing_cache = tempfile::tempdir().unwrap();
    let missing_target = missing_cache.path();
    setup_fake_env(missing_target);
    run(missing_target, &["install", "codex"], 0);
    approve_compact_state(missing_target);
    run_with_env(
        missing_target,
        &["compact", "prepare"],
        0,
        &[("AIPLUS_MODEL", "gpt-new-model")],
    );
    run_with_env(
        missing_target,
        &["compact", "resume"],
        0,
        &[("AIPLUS_MODEL", "gpt-new-model")],
    );
    let missing_out = stdout(&run(missing_target, &["compact", "savings"], 0));
    assert!(missing_out.contains("Estimated cost saved: unavailable"));

    let weighted = tempfile::tempdir().unwrap();
    let weighted_target = weighted.path();
    setup_fake_env(weighted_target);
    run(weighted_target, &["install", "codex"], 0);
    let ledger_dir = weighted_target.join(".codex/compact");
    fs::write(
        ledger_dir.join("savings-ledger.jsonl"),
        format!(
            "{}\n{}\nnot json\n",
            savings_event_fixture(100, 50, 50.0, Some(0.01)),
            savings_event_fixture(900, 300, 33.3, None)
        ),
    )
    .unwrap();
    let weighted_out = stdout(&run(weighted_target, &["compact", "savings"], 0));
    assert!(weighted_out.contains("Average reduction: ~35"));
    assert!(weighted_out.contains("Unpriced compacts: 1"));
    assert!(weighted_out.contains("WARNING malformed ledger lines ignored: 1"));

    let pricing_status = stdout(&run(target, &["pricing", "status"], 0));
    assert!(pricing_status.contains("PRICING_STATUS=PASS"));
    assert!(pricing_status.contains("billing_data=no"));
    assert!(pricing_status.contains("uploads=none"));

    let remote_catalog = target.join("pricing-source.json");
    fs::write(
        &remote_catalog,
        r#"{"schemaVersion":"0.3.0","fetchedAt":null,"sourceUrl":"file-test","source":"official","models":[{"provider":"test","model":"updated-model","inputUsdPer1mTokens":3.0,"source":"official","sourceUrl":"file-test"}]}"#,
    )
    .unwrap();
    let update = stdout(&run_with_env(
        target,
        &["pricing", "update"],
        0,
        &[(
            "AIPLUS_PRICING_URL",
            &format!("file://{}", remote_catalog.display()),
        )],
    ));
    assert!(update.contains("PRICING_UPDATE_STATUS=PASS"));
    assert!(update.contains("uploads=none"));

    let failed_update = stdout(&run_with_env(
        target,
        &["pricing", "update"],
        0,
        &[("AIPLUS_PRICING_URL", "file:///does/not/exist")],
    ));
    assert!(failed_update.contains("PRICING_UPDATE_STATUS=PASS"));
}

#[test]
fn compact_source_does_not_invoke_node() {
    let manifest_dir = Path::new(env!("CARGO_MANIFEST_DIR"));
    let source = fs::read_to_string(manifest_dir.join("src/main.rs")).unwrap();
    assert!(!source.contains("Command::new(\"node\")"));
    assert!(!source.contains("failed to launch Node compact bridge"));
    assert!(!source.contains("COMPACT_RUST_NATIVE_STATUS=PARTIAL"));
}

#[test]
fn self_update_and_update_all_are_safe_in_fake_home() {
    let temp = tempfile::tempdir().unwrap();
    let target = temp.path();
    setup_fake_env(target);
    let install_dir = target.join("fake-bin");
    fs::create_dir(&install_dir).unwrap();
    let installed = install_dir.join("aiplus");
    fs::copy(bin(), &installed).unwrap();
    let release_dir = seed_release_asset(target, false);

    let before = digest(target);
    let dry = stdout(&run_with_env(
        target,
        &["self", "update", "--dry-run"],
        0,
        &[
            ("AIPLUS_SELF_UPDATE_TARGET", installed.to_str().unwrap()),
            (
                "AIPLUS_RELEASE_BASE_URL",
                &format!("file://{}", release_dir.display()),
            ),
        ],
    ));
    assert!(dry.contains("SELF_UPDATE"));
    assert!(dry.contains("SELF_UPDATE_STATUS=DRY_RUN"));
    assert_eq!(digest(target), before);

    let updated = stdout(&run_with_env(
        target,
        &["self", "update", "--yes"],
        0,
        &[
            ("AIPLUS_SELF_UPDATE_TARGET", installed.to_str().unwrap()),
            (
                "AIPLUS_RELEASE_BASE_URL",
                &format!("file://{}", release_dir.display()),
            ),
        ],
    ));
    assert!(updated.contains("checksum_status=PASS"));
    assert!(updated.contains("backup_path="));
    assert!(updated.contains("SELF_UPDATE_STATUS=PASS"));
    assert!(fs::read_dir(&install_dir).unwrap().any(|entry| entry
        .unwrap()
        .file_name()
        .to_string_lossy()
        .contains(".backup-")));

    let bad_release = seed_release_asset(target, true);
    let bad = run_with_env(
        target,
        &["self", "update", "--yes"],
        1,
        &[
            ("AIPLUS_SELF_UPDATE_TARGET", installed.to_str().unwrap()),
            (
                "AIPLUS_RELEASE_BASE_URL",
                &format!("file://{}", bad_release.display()),
            ),
        ],
    );
    assert!(stderr(&bad).contains("ERROR checksum mismatch"));

    run(target, &["install", "codex"], 0);
    let update_all = stdout(&run_with_env(
        target,
        &["update", "all"],
        0,
        &[
            ("AIPLUS_SELF_UPDATE_TARGET", installed.to_str().unwrap()),
            (
                "AIPLUS_RELEASE_BASE_URL",
                &format!("file://{}", release_dir.display()),
            ),
        ],
    ));
    assert!(update_all.contains("AIPLUS_UPDATE_ALL"));
    assert!(update_all.contains("SELF_UPDATE_STATUS=PASS"));
    assert!(update_all.contains("PROJECT_UPDATE_STATUS=PASS"));
    assert!(update_all.contains("UPDATE_ALL_STATUS=PASS"));
    assert!(target.join(".codex/compact").exists());

    let no_project = tempfile::tempdir().unwrap();
    let no_project_target = no_project.path();
    setup_fake_env(no_project_target);
    let no_project_bin = no_project_target.join("fake-bin");
    fs::create_dir(&no_project_bin).unwrap();
    let no_project_installed = no_project_bin.join("aiplus");
    fs::copy(bin(), &no_project_installed).unwrap();
    let no_project_release = seed_release_asset(no_project_target, false);
    let no_project_out = stdout(&run_with_env(
        no_project_target,
        &["update", "all"],
        0,
        &[
            (
                "AIPLUS_SELF_UPDATE_TARGET",
                no_project_installed.to_str().unwrap(),
            ),
            (
                "AIPLUS_RELEASE_BASE_URL",
                &format!("file://{}", no_project_release.display()),
            ),
        ],
    ));
    assert!(no_project_out.contains("SELF_UPDATE_STATUS=PASS"));
    assert!(no_project_out.contains("PROJECT_UPDATE_STATUS=NO_PROJECT"));
    assert!(no_project_out.contains("UPDATE_ALL_STATUS=PASS"));
}

#[test]
fn self_update_falls_back_to_sha256_when_checksums_txt_missing() {
    let temp = tempfile::tempdir().unwrap();
    let target = temp.path();
    setup_fake_env(target);
    let install_dir = target.join("fake-bin");
    fs::create_dir(&install_dir).unwrap();
    let installed = install_dir.join("aiplus");
    fs::copy(bin(), &installed).unwrap();
    let release_dir = seed_release_asset_sha256_only(target);

    let updated = stdout(&run_with_env(
        target,
        &["self", "update", "--yes"],
        0,
        &[
            ("AIPLUS_SELF_UPDATE_TARGET", installed.to_str().unwrap()),
            (
                "AIPLUS_RELEASE_BASE_URL",
                &format!("file://{}", release_dir.display()),
            ),
        ],
    ));
    assert!(
        updated.contains("checksum_source=aiplus-aarch64-apple-darwin.tar.gz.sha256"),
        "should fall back to .sha256 file\n{updated}"
    );
    assert!(updated.contains("checksum_status=PASS"));
    assert!(updated.contains("SELF_UPDATE_STATUS=PASS"));
}

#[test]
fn user_profile_and_secret_broker_are_secret_safe() {
    let temp = tempfile::tempdir().unwrap();
    let target = temp.path();
    setup_fake_env(target);
    let profile_source = target.join("profile-source");
    fs::create_dir(&profile_source).unwrap();
    fs::write(
        profile_source.join("profile.toml"),
        "name = \"example-private-profile\"\nsecret_values = \"never-store\"\n",
    )
    .unwrap();
    fs::write(
        profile_source.join("AGENTS.profile.md"),
        "# example-private-profile\n\nUse aiplus secret-broker status. Do not store secret values.\n",
    )
    .unwrap();
    fs::write(
        profile_source.join("secret-aliases.tsv"),
        expected_secret_aliases()
            .iter()
            .map(|(alias, secret_name, env_var)| format!("{alias}\t{secret_name}\t{env_var}\n"))
            .collect::<String>(),
    )
    .unwrap();

    let profile_dry = stdout(&run(
        target,
        &[
            "profile",
            "install",
            "example-private-profile",
            "--user",
            "--source",
            profile_source.to_str().unwrap(),
            "--dry-run",
        ],
        0,
    ));
    assert!(profile_dry.contains("PROFILE_INSTALL_STATUS=DRY_RUN"));
    assert!(!target
        .join("fake-xdg/aiplus/profiles/example-private-profile/profile.toml")
        .exists());

    let profile_install = stdout(&run(
        target,
        &[
            "profile",
            "install",
            "example-private-profile",
            "--user",
            "--source",
            profile_source.to_str().unwrap(),
            "--yes",
        ],
        0,
    ));
    assert!(profile_install.contains("PROFILE_INSTALL_STATUS=PASS"));
    let profile = fs::read_to_string(
        target.join("fake-xdg/aiplus/profiles/example-private-profile/AGENTS.profile.md"),
    )
    .unwrap();
    assert!(profile.contains("example-private-profile"));
    assert!(profile.contains("aiplus secret-broker status"));
    assert!(!profile.contains("BWS_ACCESS_TOKEN"));
    assert!(!profile.contains("sk-"));

    let profile_status = stdout(&run(target, &["profile", "status"], 0));
    assert!(profile_status.contains("installed=yes"));
    assert!(profile_status.contains("secret_values=none"));

    run(target, &["install", "codex"], 0);
    let link = stdout(&run(
        target,
        &["profile", "link", "example-private-profile", "--project"],
        0,
    ));
    assert!(link.contains("PROFILE_LINK_STATUS=PASS"));
    let linked = fs::read_to_string(target.join(".aiplus/PROFILE.aiplus.md")).unwrap();
    assert!(linked.contains("example-private-profile"));
    assert!(!linked.contains("sk-"));

    let broker_status = stdout(&run(target, &["secret-broker", "status"], 0));
    assert!(broker_status.contains("SECRET_BROKER_STATUS=PASS"));
    assert!(broker_status.contains("secret_values_printed=no"));

    let broker_list = stdout(&run(target, &["secret-broker", "list"], 0));
    for (alias, secret_name, env_var) in expected_secret_aliases() {
        assert!(
            broker_list.contains(&format!("{alias} -> {secret_name} -> {env_var}")),
            "missing alias mapping for {alias}"
        );
    }
    assert!(broker_list.contains("SECRET_ALIAS_STATUS=PASS"));
    assert!(!broker_list.contains("AIPLUS_MOCK_OPENAI"));

    for alias in ["openai", "kimi", "deepseek", "qwen"] {
        let resolve = stdout(&run_with_env(
            target,
            &["secret-broker", "resolve", alias],
            0,
            &[("AIPLUS_SECRET_PROVIDER", "mock")],
        ));
        assert!(resolve.contains("SECRET_RESOLVE_STATUS=PASS"));
        assert!(resolve.contains("secret_value_printed=no"));
        assert!(!resolve.contains("SECRET_ALIAS_NOT_ALLOWED"));
        assert!(
            !resolve.contains(&format!("AIPLUS_MOCK_{}", alias.to_ascii_uppercase())),
            "resolve printed a mock secret value for {alias}"
        );
    }

    let unknown_alias = run_with_env(
        target,
        &["secret-broker", "resolve", "unknown-provider"],
        1,
        &[("AIPLUS_SECRET_PROVIDER", "mock")],
    );
    assert!(stderr(&unknown_alias).contains("SECRET_ALIAS_NOT_ALLOWED unknown-provider"));

    let denied_print = run_with_env(
        target,
        &["secret-broker", "resolve", "openai", "--print"],
        1,
        &[("AIPLUS_SECRET_PROVIDER", "mock")],
    );
    assert!(stderr(&denied_print).contains("--print is disabled"));

    let run_out = stdout(&run_with_env(
        target,
        &[
            "secret-broker",
            "run",
            "--",
            "sh",
            "-c",
            "test -n \"$OPENAI_API_KEY\" && echo broker-env-ok",
        ],
        0,
        &[("AIPLUS_SECRET_PROVIDER", "mock")],
    ));
    assert!(run_out.contains("broker-env-ok"));
    assert!(!run_out.contains("AIPLUS_MOCK_OPENAI"));

    let disable = stdout(&run(
        target,
        &[
            "profile",
            "disable",
            "example-private-profile",
            "--user",
            "--yes",
        ],
        0,
    ));
    assert!(disable.contains("PROFILE_DISABLE_STATUS=PASS"));

    let uninstall = stdout(&run(
        target,
        &[
            "profile",
            "uninstall",
            "example-private-profile",
            "--user",
            "--yes",
        ],
        0,
    ));
    assert!(uninstall.contains("PROFILE_UNINSTALL_STATUS=PASS"));
    assert!(uninstall.contains("secret_aliases_removed=yes"));
    assert!(!target
        .join("fake-xdg/aiplus/profiles/example-private-profile")
        .exists());
    assert!(!target
        .join("fake-xdg/aiplus/secret-broker/profiles/example-private-profile")
        .exists());
}

#[test]
fn canonical_profile_cleanup_and_migrate_legacy_registration() {
    let temp = tempfile::tempdir().unwrap();
    let target = temp.path();
    setup_fake_env(target);
    seed_installed_profile(target, "aiplus-work-with-zhiwen", "canonical");
    seed_installed_profile(target, "work-with-zhiwen", "legacy");

    let status = stdout(&run(target, &["profile", "status"], 0));
    assert!(status.contains("profiles=[aiplus-work-with-zhiwen]"));
    assert!(status.contains("legacy_profiles=[work-with-zhiwen]"));
    assert!(status.contains("next=run aiplus profile cleanup --user --yes"));

    let dry = stdout(&run(
        target,
        &["profile", "cleanup", "--user", "--dry-run"],
        0,
    ));
    assert!(dry.contains("PROFILE_CLEANUP_STATUS=DRY_RUN"));
    assert!(target
        .join("fake-xdg/aiplus/profiles/work-with-zhiwen/profile.toml")
        .exists());

    let cleanup = stdout(&run(target, &["profile", "cleanup", "--user", "--yes"], 0));
    assert!(cleanup.contains("PROFILE_CLEANUP_STATUS=PASS"));
    assert!(cleanup.contains("profile_removed=work-with-zhiwen"));
    assert!(!target
        .join("fake-xdg/aiplus/profiles/work-with-zhiwen")
        .exists());
    assert!(target
        .join("fake-xdg/aiplus/profiles/aiplus-work-with-zhiwen/profile.toml")
        .exists());
    assert!(target.join("fake-xdg/aiplus/profile-backups").exists());

    seed_installed_profile(target, "work-with-zhiwen", "legacy-again");
    let migrate = stdout(&run(
        target,
        &[
            "profile",
            "migrate",
            "work-with-zhiwen",
            "aiplus-work-with-zhiwen",
            "--user",
            "--yes",
        ],
        0,
    ));
    assert!(migrate.contains("PROFILE_MIGRATE_STATUS=PASS"));
    assert!(!target
        .join("fake-xdg/aiplus/profiles/work-with-zhiwen")
        .exists());

    let final_status = stdout(&run(target, &["profile", "status"], 0));
    assert!(final_status.contains("profiles=[aiplus-work-with-zhiwen]"));
    assert!(!final_status.contains("legacy_profiles=[work-with-zhiwen]"));
}

#[test]
fn bws_resolver_looks_up_secret_id_without_printing_value() {
    let temp = tempfile::tempdir().unwrap();
    let target = temp.path();
    setup_fake_env(target);
    let profile_source = target.join("profile-source");
    fs::create_dir(&profile_source).unwrap();
    fs::write(
        profile_source.join("profile.toml"),
        "name = \"example-private-profile\"\n",
    )
    .unwrap();
    fs::write(profile_source.join("AGENTS.profile.md"), "# example\n").unwrap();
    fs::write(
        profile_source.join("secret-aliases.tsv"),
        "kimi\tprivate/kimi/api_key\tKIMI_API_KEY\nopenai\tprivate/openai/api_key\tOPENAI_API_KEY\n",
    )
    .unwrap();
    run(
        target,
        &[
            "profile",
            "install",
            "example-private-profile",
            "--user",
            "--source",
            profile_source.to_str().unwrap(),
            "--yes",
        ],
        0,
    );
    let fake_bin = target.join("fake-bin");
    fs::create_dir(&fake_bin).unwrap();
    write_fake_bws(&fake_bin.join("bws"), "ok");
    let path_value = format!("{}:/usr/bin:/bin", fake_bin.display());
    let resolve = stdout(&run_with_env_and_path(
        target,
        &["secret-broker", "resolve", "kimi"],
        0,
        &[
            ("BWS_ACCESS_TOKEN", "fixture-token"),
            ("AIPLUS_BWS_PROJECT_ID", "fixture-project"),
        ],
        Path::new(&path_value),
    ));
    assert!(resolve.contains("SECRET_RESOLVE_STATUS=PASS"));
    assert!(resolve.contains("provider=bws"));
    assert!(resolve.contains("token_source=env"));
    assert!(resolve.contains("secret_key=private/kimi/api_key"));
    assert!(resolve.contains("secret_id_found=yes"));
    assert!(resolve.contains("provider_family=kimi_code"));
    assert!(resolve.contains("platform=kimi_code_membership"));
    assert!(resolve.contains("base_url=https://api.kimi.com/coding/v1"));
    assert!(resolve.contains("model=kimi-for-coding"));
    assert!(resolve.contains("smoke_endpoint=https://api.kimi.com/coding/v1/models"));
    assert!(resolve.contains("secret_value_printed=no"));
    assert!(!resolve.contains("fixture-secret-value"));
    assert!(!resolve.contains("secret-kimi-id"));

    write_fake_bws(&fake_bin.join("bws"), "missing");
    let missing = run_with_env_and_path(
        target,
        &["secret-broker", "resolve", "kimi"],
        1,
        &[
            ("BWS_ACCESS_TOKEN", "fixture-token"),
            ("AIPLUS_BWS_PROJECT_ID", "fixture-project"),
        ],
        Path::new(&path_value),
    );
    assert!(stderr(&missing).contains("reason=secret_key_not_found"));

    write_fake_bws(&fake_bin.join("bws"), "invalid");
    let invalid = run_with_env_and_path(
        target,
        &["secret-broker", "resolve", "kimi"],
        1,
        &[
            ("BWS_ACCESS_TOKEN", "fixture-token"),
            ("AIPLUS_BWS_PROJECT_ID", "fixture-project"),
        ],
        Path::new(&path_value),
    );
    assert!(stderr(&invalid).contains("reason=invalid_json"));

    let token_missing = run_with_env_and_path(
        target,
        &["secret-broker", "resolve", "kimi"],
        1,
        &[("AIPLUS_BWS_PROJECT_ID", "fixture-project")],
        Path::new(&path_value),
    );
    assert!(stderr(&token_missing).contains("SECRET_BROKER_TOKEN_MISSING"));
}

#[test]
fn bws_resolver_rejects_placeholder_and_empty_values() {
    let temp = tempfile::tempdir().unwrap();
    let target = temp.path();
    setup_fake_env(target);
    let profile_source = target.join("profile-source");
    fs::create_dir(&profile_source).unwrap();
    fs::write(
        profile_source.join("profile.toml"),
        "name = \"example-private-profile\"\n",
    )
    .unwrap();
    fs::write(profile_source.join("AGENTS.profile.md"), "# example\n").unwrap();
    fs::write(
        profile_source.join("secret-aliases.tsv"),
        "openai\tprivate/openai/api_key\tOPENAI_API_KEY\nxai\tprivate/xai/api_key\tXAI_API_KEY\nempty\tprivate/empty/api_key\tEMPTY_API_KEY\nblank\tprivate/blank/api_key\tBLANK_API_KEY\n",
    )
    .unwrap();
    run(
        target,
        &[
            "profile",
            "install",
            "example-private-profile",
            "--user",
            "--source",
            profile_source.to_str().unwrap(),
            "--yes",
        ],
        0,
    );
    let fake_bin = target.join("fake-bin");
    fs::create_dir(&fake_bin).unwrap();
    write_fake_bws(&fake_bin.join("bws"), "placeholder");
    let path_value = format!("{}:/usr/bin:/bin", fake_bin.display());

    for alias in ["xai", "empty", "blank"] {
        let resolve = run_with_env_and_path(
            target,
            &["secret-broker", "resolve", alias],
            1,
            &[
                ("BWS_ACCESS_TOKEN", "fixture-token"),
                ("AIPLUS_BWS_PROJECT_ID", "fixture-project"),
            ],
            Path::new(&path_value),
        );
        let err = stderr(&resolve);
        assert!(err.contains(&format!(
            "SECRET_RESOLVE_STATUS=FAIL alias={alias} provider=bws reason=secret_placeholder_or_empty"
        )));
        assert!(!err.contains("PENDING_OWNER_INPUT_DO_NOT_USE"));
        assert!(!err.contains("fixture-secret-value"));
    }

    let requested_placeholder = run_with_env_and_path(
        target,
        &[
            "secret-broker",
            "run",
            "--aliases",
            "xai",
            "--",
            "sh",
            "-c",
            "echo should-not-run",
        ],
        1,
        &[
            ("BWS_ACCESS_TOKEN", "fixture-token"),
            ("AIPLUS_BWS_PROJECT_ID", "fixture-project"),
        ],
        Path::new(&path_value),
    );
    let err = stderr(&requested_placeholder);
    assert!(
        err.contains("SECRET_BROKER_RUN_STATUS=FAIL alias=xai reason=secret_placeholder_or_empty")
    );
    assert!(!err.contains("PENDING_OWNER_INPUT_DO_NOT_USE"));
    assert!(!err.contains("fixture-secret-value"));

    let selective_valid = stdout(&run_with_env_and_path(
        target,
        &[
            "secret-broker",
            "run",
            "--aliases",
            "openai",
            "--",
            "sh",
            "-c",
            "test -n \"$OPENAI_API_KEY\" && test -z \"${XAI_API_KEY+x}\" && echo valid-only-ok",
        ],
        0,
        &[
            ("BWS_ACCESS_TOKEN", "fixture-token"),
            ("AIPLUS_BWS_PROJECT_ID", "fixture-project"),
        ],
        Path::new(&path_value),
    ));
    assert!(selective_valid.contains("requested_aliases=[openai]"));
    assert!(selective_valid.contains("injected_env=[OPENAI_API_KEY]"));
    assert!(selective_valid.contains("valid-only-ok"));
    assert!(selective_valid.contains("SECRET_BROKER_RUN_STATUS=PASS"));
    assert!(!selective_valid.contains("PENDING_OWNER_INPUT_DO_NOT_USE"));
    assert!(!selective_valid.contains("fixture-secret-value"));

    let best_effort = stdout(&run_with_env_and_path(
        target,
        &[
            "secret-broker",
            "run",
            "--",
            "sh",
            "-c",
            "test -n \"$OPENAI_API_KEY\" && test -z \"${XAI_API_KEY+x}\" && test -z \"${EMPTY_API_KEY+x}\" && test -z \"${BLANK_API_KEY+x}\" && echo placeholders-skipped",
        ],
        0,
        &[
            ("BWS_ACCESS_TOKEN", "fixture-token"),
            ("AIPLUS_BWS_PROJECT_ID", "fixture-project"),
        ],
        Path::new(&path_value),
    ));
    assert!(best_effort.contains("requested_aliases=[]"));
    assert!(best_effort.contains("skipped_aliases=[blank,empty,xai]"));
    assert!(best_effort.contains("placeholders-skipped"));
    assert!(best_effort.contains("SECRET_BROKER_RUN_STATUS=PASS"));
    assert!(!best_effort.contains("PENDING_OWNER_INPUT_DO_NOT_USE"));
    assert!(!best_effort.contains("fixture-secret-value"));
}

#[test]
fn secret_broker_run_selective_aliases_skip_unrequested_failures() {
    let temp = tempfile::tempdir().unwrap();
    let target = temp.path();
    setup_fake_env(target);
    let profile_source = target.join("profile-source");
    fs::create_dir(&profile_source).unwrap();
    fs::write(
        profile_source.join("profile.toml"),
        "name = \"example-private-profile\"\n",
    )
    .unwrap();
    fs::write(profile_source.join("AGENTS.profile.md"), "# example\n").unwrap();
    fs::write(
        profile_source.join("secret-aliases.tsv"),
        "kimi\tprivate/kimi/api_key\tKIMI_API_KEY\nopenai\tprivate/openai/api_key\tOPENAI_API_KEY\nvoyage\tprivate/voyage/api_key\tVOYAGE_API_KEY\nkimi_platform\tprivate/kimi_platform/api_key\tKIMI_PLATFORM_API_KEY\n",
    )
    .unwrap();
    run(
        target,
        &[
            "profile",
            "install",
            "example-private-profile",
            "--user",
            "--source",
            profile_source.to_str().unwrap(),
            "--yes",
        ],
        0,
    );
    let fake_bin = target.join("fake-bin");
    fs::create_dir(&fake_bin).unwrap();
    write_fake_bws(&fake_bin.join("bws"), "ok");
    let path_value = format!("{}:/usr/bin:/bin", fake_bin.display());

    let selective = stdout(&run_with_env_and_path(
        target,
        &[
            "secret-broker",
            "run",
            "--aliases",
            "openai,kimi",
            "--",
            "sh",
            "-c",
            "test -n \"$OPENAI_API_KEY\" && test -n \"$KIMI_API_KEY\" && test -z \"${VOYAGE_API_KEY+x}\" && echo selective-env-ok",
        ],
        0,
        &[
            ("BWS_ACCESS_TOKEN", "fixture-token"),
            ("AIPLUS_BWS_PROJECT_ID", "fixture-project"),
        ],
        Path::new(&path_value),
    ));
    assert!(selective.contains("SECRET_BROKER_RUN"));
    assert!(selective.contains("requested_aliases=[openai,kimi]"));
    assert!(selective.contains("injected_env=[OPENAI_API_KEY,KIMI_API_KEY]"));
    assert!(selective.contains("skipped_aliases=[]"));
    assert!(selective.contains("selective-env-ok"));
    assert!(selective.contains("SECRET_BROKER_RUN_STATUS=PASS"));
    assert!(!selective.contains("fixture-secret-value"));
    assert!(!selective.contains("secret-openai-id"));
    assert!(!selective.contains("secret-kimi-id"));

    let repeated_flags = stdout(&run_with_env_and_path(
        target,
        &[
            "secret-broker",
            "run",
            "--alias",
            "openai",
            "--alias",
            "kimi",
            "--",
            "sh",
            "-c",
            "test -n \"$OPENAI_API_KEY\" && test -n \"$KIMI_API_KEY\" && echo repeated-alias-ok",
        ],
        0,
        &[
            ("BWS_ACCESS_TOKEN", "fixture-token"),
            ("AIPLUS_BWS_PROJECT_ID", "fixture-project"),
        ],
        Path::new(&path_value),
    ));
    assert!(repeated_flags.contains("requested_aliases=[openai,kimi]"));
    assert!(repeated_flags.contains("repeated-alias-ok"));
    assert!(!repeated_flags.contains("fixture-secret-value"));

    let best_effort = stdout(&run_with_env_and_path(
        target,
        &[
            "secret-broker",
            "run",
            "--",
            "sh",
            "-c",
            "test -n \"$OPENAI_API_KEY\" && test -n \"$KIMI_API_KEY\" && echo best-effort-ok",
        ],
        0,
        &[
            ("BWS_ACCESS_TOKEN", "fixture-token"),
            ("AIPLUS_BWS_PROJECT_ID", "fixture-project"),
        ],
        Path::new(&path_value),
    ));
    assert!(best_effort.contains("requested_aliases=[]"));
    assert!(best_effort.contains("skipped_aliases=[kimi_platform,voyage]"));
    assert!(best_effort.contains("best-effort-ok"));
    assert!(best_effort.contains("SECRET_BROKER_RUN_STATUS=PASS"));
    assert!(!best_effort.contains("fixture-secret-value"));

    let requested_missing = run_with_env_and_path(
        target,
        &[
            "secret-broker",
            "run",
            "--aliases",
            "voyage",
            "--",
            "sh",
            "-c",
            "echo should-not-run",
        ],
        1,
        &[
            ("BWS_ACCESS_TOKEN", "fixture-token"),
            ("AIPLUS_BWS_PROJECT_ID", "fixture-project"),
        ],
        Path::new(&path_value),
    );
    assert!(stderr(&requested_missing)
        .contains("SECRET_BROKER_RUN_STATUS=FAIL alias=voyage reason=secret_key_not_found"));

    let unknown_alias = run_with_env_and_path(
        target,
        &[
            "secret-broker",
            "run",
            "--aliases",
            "unknown-provider",
            "--",
            "sh",
            "-c",
            "echo should-not-run",
        ],
        1,
        &[
            ("BWS_ACCESS_TOKEN", "fixture-token"),
            ("AIPLUS_BWS_PROJECT_ID", "fixture-project"),
        ],
        Path::new(&path_value),
    );
    assert!(stderr(&unknown_alias)
        .contains("SECRET_BROKER_RUN_STATUS=FAIL alias=unknown-provider reason=unknown_alias"));
}

fn expected_secret_aliases() -> Vec<(&'static str, &'static str, &'static str)> {
    vec![
        ("openai", "private/openai/api_key", "OPENAI_API_KEY"),
        (
            "anthropic",
            "private/anthropic/api_key",
            "ANTHROPIC_API_KEY",
        ),
        ("gemini", "private/gemini/api_key", "GEMINI_API_KEY"),
        ("github", "private/github/token", "GITHUB_TOKEN"),
        (
            "cloudflare",
            "private/cloudflare/token",
            "CLOUDFLARE_API_TOKEN",
        ),
        ("kimi", "private/kimi/api_key", "KIMI_API_KEY"),
        ("deepseek", "private/deepseek/api_key", "DEEPSEEK_API_KEY"),
        ("minimax", "private/minimax/api_key", "MINIMAX_API_KEY"),
        ("qwen", "private/qwen/api_key", "QWEN_API_KEY"),
        ("glm", "private/glm/api_key", "GLM_API_KEY"),
        (
            "openrouter",
            "private/openrouter/api_key",
            "OPENROUTER_API_KEY",
        ),
        ("xai", "private/xai/api_key", "XAI_API_KEY"),
        ("groq", "private/groq/api_key", "GROQ_API_KEY"),
        ("mistral", "private/mistral/api_key", "MISTRAL_API_KEY"),
        (
            "perplexity",
            "private/perplexity/api_key",
            "PERPLEXITY_API_KEY",
        ),
        ("together", "private/together/api_key", "TOGETHER_API_KEY"),
        ("cohere", "private/cohere/api_key", "COHERE_API_KEY"),
        (
            "huggingface",
            "private/huggingface/token",
            "HUGGINGFACE_TOKEN",
        ),
        ("voyage", "private/voyage/api_key", "VOYAGE_API_KEY"),
        ("jina", "private/jina/api_key", "JINA_API_KEY"),
        (
            "replicate",
            "private/replicate/api_token",
            "REPLICATE_API_TOKEN",
        ),
        ("fal", "private/fal/api_key", "FAL_API_KEY"),
        (
            "stability",
            "private/stability/api_key",
            "STABILITY_API_KEY",
        ),
        (
            "elevenlabs",
            "private/elevenlabs/api_key",
            "ELEVENLABS_API_KEY",
        ),
        ("tavily", "private/tavily/api_key", "TAVILY_API_KEY"),
        ("exa", "private/exa/api_key", "EXA_API_KEY"),
        ("serper", "private/serper/api_key", "SERPER_API_KEY"),
        (
            "firecrawl",
            "private/firecrawl/api_key",
            "FIRECRAWL_API_KEY",
        ),
        ("brave", "private/brave/api_key", "BRAVE_API_KEY"),
        (
            "siliconflow",
            "private/siliconflow/api_key",
            "SILICONFLOW_API_KEY",
        ),
        (
            "volcengine_ark",
            "private/volcengine_ark/api_key",
            "VOLCENGINE_ARK_API_KEY",
        ),
    ]
}

fn setup_fake_env(target: &Path) {
    fs::create_dir(target.join("fake-home")).unwrap();
    fs::create_dir(target.join("fake-codex-home")).unwrap();
    fs::create_dir(target.join("fake-xdg")).unwrap();
}

fn seed_installed_profile(target: &Path, profile: &str, marker: &str) {
    let dir = target.join("fake-xdg/aiplus/profiles").join(profile);
    fs::create_dir_all(&dir).unwrap();
    fs::write(
        dir.join("profile.toml"),
        format!("name = \"{profile}\"\nmarker = \"{marker}\"\n"),
    )
    .unwrap();
    fs::write(
        dir.join("AGENTS.profile.md"),
        format!("# {profile}\n\nmarker={marker}\n"),
    )
    .unwrap();
    let alias_dir = target
        .join("fake-xdg/aiplus/secret-broker/profiles")
        .join(profile);
    fs::create_dir_all(&alias_dir).unwrap();
    fs::write(
        alias_dir.join("secret-aliases.tsv"),
        "openai\tprivate/openai/api_key\tOPENAI_API_KEY\n",
    )
    .unwrap();
}

fn write_fake_bws(path: &Path, mode: &str) {
    let mut file = fs::File::create(path).unwrap();
    writeln!(
        file,
        r#"#!/bin/sh
set -eu
mode="{mode}"
if [ "$1" = "--version" ]; then
  echo "bws fixture"
  exit 0
fi
if [ "$1" != "secret" ]; then
  exit 2
fi
if [ "$2" = "list" ]; then
  if [ "$mode" = "invalid" ]; then
    printf 'not-json'
    exit 0
  fi
  if [ "$mode" = "missing" ]; then
    printf '[{{"id":"other-id","key":"private/other/api_key","value":"fixture-secret-value"}}]'
    exit 0
  fi
  if [ "$mode" = "placeholder" ]; then
    printf '[{{"id":"secret-openai-id","key":"private/openai/api_key","value":"fixture-secret-value"}},{{"id":"secret-xai-id","key":"private/xai/api_key","value":"PENDING_OWNER_INPUT_DO_NOT_USE"}},{{"id":"secret-empty-id","key":"private/empty/api_key","value":""}},{{"id":"secret-blank-id","key":"private/blank/api_key","value":"   "}}]'
    exit 0
  fi
  printf '[{{"id":"secret-kimi-id","key":"private/kimi/api_key","value":"fixture-secret-value"}},{{"id":"secret-openai-id","key":"private/openai/api_key","value":"fixture-secret-value"}}]'
  exit 0
fi
if [ "$2" = "get" ]; then
  if [ "$3" = "secret-kimi-id" ]; then
    printf '{{"id":"secret-kimi-id","key":"private/kimi/api_key","value":"fixture-secret-value"}}'
    exit 0
  fi
  if [ "$3" = "secret-openai-id" ]; then
    printf '{{"id":"secret-openai-id","key":"private/openai/api_key","value":"fixture-secret-value"}}'
    exit 0
  fi
  if [ "$3" = "secret-xai-id" ]; then
    printf '{{"id":"secret-xai-id","key":"private/xai/api_key","value":"PENDING_OWNER_INPUT_DO_NOT_USE"}}'
    exit 0
  fi
  if [ "$3" = "secret-empty-id" ]; then
    printf '{{"id":"secret-empty-id","key":"private/empty/api_key","value":""}}'
    exit 0
  fi
  if [ "$3" = "secret-blank-id" ]; then
    printf '{{"id":"secret-blank-id","key":"private/blank/api_key","value":"   "}}'
    exit 0
  fi
  exit 1
fi
exit 2
"#
    )
    .unwrap();
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut permissions = fs::metadata(path).unwrap().permissions();
        permissions.set_mode(0o755);
        fs::set_permissions(path, permissions).unwrap();
    }
}

fn seed_release_asset(target: &Path, bad_checksum: bool) -> PathBuf {
    let release = target.join(if bad_checksum {
        "bad-release"
    } else {
        "release"
    });
    let root = release.join("root");
    fs::create_dir_all(&root).unwrap();
    fs::copy(bin(), root.join("aiplus")).unwrap();
    let archive = release.join("aiplus-aarch64-apple-darwin.tar.gz");
    let status = Command::new("tar")
        .arg("-czf")
        .arg(&archive)
        .arg("-C")
        .arg(&root)
        .arg("aiplus")
        .status()
        .unwrap();
    assert!(status.success());
    let output = Command::new("shasum")
        .args(["-a", "256"])
        .arg(&archive)
        .output()
        .unwrap();
    assert!(output.status.success());
    let mut line = String::from_utf8(output.stdout).unwrap();
    if bad_checksum {
        line = format!(
            "0000000000000000000000000000000000000000000000000000000000000000  {}\n",
            archive.file_name().unwrap().to_string_lossy()
        );
    }
    fs::write(release.join("checksums.txt"), line).unwrap();
    release
}

fn seed_release_asset_sha256_only(target: &Path) -> PathBuf {
    let release = target.join("release-sha256-only");
    let root = release.join("root");
    fs::create_dir_all(&root).unwrap();
    fs::copy(bin(), root.join("aiplus")).unwrap();
    let archive = release.join("aiplus-aarch64-apple-darwin.tar.gz");
    let status = Command::new("tar")
        .arg("-czf")
        .arg(&archive)
        .arg("-C")
        .arg(&root)
        .arg("aiplus")
        .status()
        .unwrap();
    assert!(status.success());
    let output = Command::new("shasum")
        .args(["-a", "256"])
        .arg(&archive)
        .output()
        .unwrap();
    assert!(output.status.success());
    let line = String::from_utf8(output.stdout).unwrap();
    // Write just the checksum (first word) to .sha256 file
    let checksum = line.split_whitespace().next().unwrap();
    fs::write(
        release.join(format!(
            "{}.sha256",
            archive.file_name().unwrap().to_string_lossy()
        )),
        checksum,
    )
    .unwrap();
    release
}

fn make_empty_path() -> PathBuf {
    tempfile::tempdir().unwrap().keep()
}

fn approve_compact_state(target: &Path) {
    for file in [
        "current-handoff.md",
        "decision-log.md",
        "agent-state-ledger.md",
        "evidence-ledger.md",
    ] {
        let path = target.join(".codex/compact").join(file);
        let next = fs::read_to_string(&path)
            .unwrap()
            .replace("UNKNOWN_PENDING", "APPROVED");
        fs::write(path, next).unwrap();
    }
    let mut policy = fs::read_to_string(target.join(".codex/compact/compact-policy.json")).unwrap();
    policy = policy.replace(
        "\"status\": \"UNKNOWN_PENDING\"",
        "\"status\": \"APPROVED\"",
    );
    fs::write(target.join(".codex/compact/compact-policy.json"), policy).unwrap();
}

fn make_compact_handoff_current(target: &Path, phase: &str, last_updated: String) {
    let path = target.join(".codex/compact/current-handoff.md");
    let mut handoff = fs::read_to_string(&path).unwrap();
    handoff = handoff.replace(
        "Synthetic template. Replace placeholders before use.\n\n",
        "",
    );
    handoff = handoff.replace("<REPO_ROOT>", "target");
    handoff = replace_section_body(&handoff, "Last Updated", &last_updated);
    handoff = replace_section_body(
        &handoff,
        "Current Goal",
        "Deliver Auto Compact reminder engine.",
    );
    handoff = replace_section_body(&handoff, "Current Phase", phase);
    handoff = replace_section_body(
        &handoff,
        "Next 3 Actions",
        "1. Run targeted reminder tests.\n2. Run cross-runtime dogfood.\n3. Hand off for Rust Lead review.",
    );
    fs::write(path, handoff).unwrap();
}

fn replace_section_body(text: &str, heading: &str, body: &str) -> String {
    let marker = format!("## {heading}");
    let Some(start) = text.find(&marker) else {
        return text.to_string();
    };
    let body_start = start + marker.len();
    let rest = &text[body_start..];
    let next = rest.find("\n## ").map(|offset| body_start + offset);
    let mut out = String::new();
    out.push_str(&text[..body_start]);
    out.push_str("\n\n");
    out.push_str(body);
    out.push('\n');
    if let Some(next) = next {
        out.push_str(&text[next..]);
    }
    out
}

fn now_unix_millis_marker() -> String {
    let millis = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis())
        .unwrap_or(0);
    format!("unix-{millis}ms")
}

fn seed_pricing_cache(target: &Path, models: &[(&str, f64)]) {
    let cache = target.join("fake-home/.cache/aiplus");
    fs::create_dir_all(&cache).unwrap();
    let models_json: Vec<_> = models
        .iter()
        .map(|(model, price)| {
            serde_json::json!({
                "provider": "test",
                "model": model,
                "inputUsdPer1mTokens": price,
                "source": "official",
                "sourceUrl": "file-test"
            })
        })
        .collect();
    fs::write(
        cache.join("pricing-cache.json"),
        serde_json::json!({
            "schemaVersion": "0.3.0",
            "fetchedAt": "test-cache",
            "sourceUrl": "file-test",
            "source": "cached",
            "models": models_json
        })
        .to_string(),
    )
    .unwrap();
}

fn find_latest_rollback_plan(target: &Path) -> PathBuf {
    let mut plans = Vec::new();
    for entry in fs::read_dir(target.join(".aiplus/backups")).unwrap() {
        let path = entry.unwrap().path().join("rollback-plan.json");
        if path.exists() {
            plans.push(path);
        }
    }
    plans.sort();
    plans.pop().unwrap()
}

fn savings_event_fixture(baseline: u64, saved: u64, reduction: f64, cost: Option<f64>) -> String {
    serde_json::json!({
        "schemaVersion": "0.3.0",
        "timestamp": "test",
        "event": "resume",
        "eventScope": "completed",
        "checkpointId": format!("fixture-{baseline}"),
        "checkpointLevel": "standard",
        "readinessState": "READY_TO_COMPACT",
        "compactPressure": "HIGH",
        "sessionRole": "Unknown",
        "workflowLevel": "Unknown",
        "estimatedInputTokensBefore": baseline,
        "estimatedHandoffTokensAfter": baseline - saved,
        "estimatedResumeTokens": 0,
        "estimatedTokensSaved": saved,
        "estimatedTokenReductionPercent": reduction,
        "estimatedCostSavedUsd": cost,
        "pricingModel": if cost.is_some() { "known-model" } else { "unavailable" },
        "pricingStatus": if cost.is_some() { "matched" } else { "unavailable" },
        "pricingSource": if cost.is_some() { "cached" } else { "unavailable" },
        "pricingFetchedAt": "test-cache",
        "pricingAgeDays": 0,
        "inputPriceUsdPer1mTokens": if cost.is_some() { Some(1.0) } else { None },
        "modelDetected": "known-model",
        "modelDetectionConfidence": "medium",
        "costEstimateAvailable": cost.is_some(),
        "costEstimateReason": if cost.is_some() { "matched cached public pricing" } else { "pricing for detected model is not available" },
        "billingData": false,
        "method": "local_estimate_v1",
        "confidence": "low",
        "notes": []
    })
    .to_string()
}

#[test]
fn velocity_cli_human_time_bias_end_to_end() {
    let temp = tempfile::tempdir().unwrap();
    let target = temp.path();
    fs::create_dir(target.join("fake-home")).unwrap();
    fs::create_dir(target.join("fake-codex-home")).unwrap();
    fs::create_dir(target.join("fake-xdg")).unwrap();

    let init = run(target, &["velocity", "init"], 0);
    let init_out = stdout(&init);
    assert!(init_out.contains("VELOCITY_INIT_STATUS=PASS"));
    assert!(init_out.contains(".aiplus/velocity"));

    let velocity_dir = target.join(".aiplus/velocity");
    assert!(velocity_dir.is_dir(), "velocity dir missing");
    for f in [
        "config.json",
        "estimates.jsonl",
        "runs.jsonl",
        "rare-cases.jsonl",
        "anchor-signals.jsonl",
        "multipliers.json",
        "aggregates.json",
        "rotation-state.json",
    ] {
        assert!(velocity_dir.join(f).exists(), "missing velocity file: {f}");
    }

    let config_text = fs::read_to_string(velocity_dir.join("config.json")).unwrap();
    let config: serde_json::Value = serde_json::from_str(&config_text).unwrap();
    assert_eq!(config["maxRecords"], 200);
    assert_eq!(config["rareCaseMaxRecords"], 20);
    assert_eq!(config["rawContentAllowed"], false);
    assert_eq!(config["memoryIntegration"], "disabled");

    let est = run(
        target,
        &[
            "velocity",
            "estimate",
            "--task-type",
            "bug_fix",
            "--human-estimate",
            "5h",
            "--task-id",
            "task_e2e_1",
            "--model",
            "claude-opus",
            "--workflow",
            "MEDIUM",
        ],
        0,
    );
    let est_out = stdout(&est);
    assert!(est_out.contains("VELOCITY_ESTIMATE_STATUS=PASS"));
    assert!(est_out.contains("HUMAN_ANCHOR_DETECTED=yes"));
    assert!(est_out.contains("HUMAN_ESTIMATE_MINUTES=300"));
    assert!(est_out.contains("STOP_WHEN_DONE=yes"));

    let estimates_jsonl = fs::read_to_string(velocity_dir.join("estimates.jsonl")).unwrap();
    assert!(estimates_jsonl.contains("\"taskId\":\"task_e2e_1\""));
    assert!(estimates_jsonl.contains("\"humanEstimateMinutes\":300"));

    let comp = run(
        target,
        &[
            "velocity",
            "complete",
            "--task-id",
            "task_e2e_1",
            "--actual",
            "20m",
            "--outcome",
            "pass",
            "--task-type",
            "bug_fix",
            "--model",
            "claude-opus",
            "--workflow",
            "MEDIUM",
        ],
        0,
    );
    let comp_out = stdout(&comp);
    assert!(comp_out.contains("VELOCITY_COMPLETE_STATUS=PASS"));
    assert!(comp_out.contains("ACTUAL_ACTIVE_MINUTES=20"));
    assert!(comp_out.contains("OVERESTIMATE_RATIO=15.0"));
    assert!(comp_out.contains("HUMAN_TIME_BIAS=detected"));
    assert!(comp_out.contains("RETENTION_STATUS=applied"));

    let runs_jsonl = fs::read_to_string(velocity_dir.join("runs.jsonl")).unwrap();
    assert!(runs_jsonl.contains("\"taskId\":\"task_e2e_1\""));
    assert!(runs_jsonl.contains("\"actualActiveMinutes\":20"));
    let rare_jsonl = fs::read_to_string(velocity_dir.join("rare-cases.jsonl")).unwrap();
    assert!(
        rare_jsonl.contains("\"taskId\":\"task_e2e_1\""),
        "expected rare case entry, rare-cases.jsonl: {rare_jsonl}"
    );

    let bias = run(target, &["velocity", "bias", "--task", "task_e2e_1"], 0);
    let bias_out = stdout(&bias);
    assert!(bias_out.contains("VELOCITY_BIAS_STATUS=PASS"));
    assert!(bias_out.contains("OVERESTIMATE_RATIO=15.0"));
    assert!(bias_out.contains("HUMAN_TIME_BIAS_FOUND=yes"));

    let rep = run(target, &["velocity", "report"], 0);
    let rep_out = stdout(&rep);
    assert!(rep_out.contains("VELOCITY_REPORT_STATUS=PASS"));
    assert!(rep_out.contains("CALIBRATION_WINDOW=latest_200"));
    assert!(rep_out.contains("TOTAL_ESTIMATES=1"));
    assert!(rep_out.contains("TOTAL_RUNS=1"));

    let doc = run(target, &["velocity", "doctor"], 0);
    let doc_out = stdout(&doc);
    assert!(doc_out.contains("VELOCITY_DOCTOR_STATUS=PASS"));
    assert!(doc_out.contains("sqlite_found=no"));
    assert!(doc_out.contains("raw_content_found=no"));
    assert!(doc_out.contains("secret_values=none"));
    assert!(doc_out.contains("global_agent_config_edits=none"));

    assert!(!velocity_dir.join("velocity.sqlite").exists());
    assert!(!velocity_dir.join("velocity.db").exists());
    assert!(!velocity_dir.join("velocity.sqlite3").exists());

    let scan_files = [
        "config.json",
        "estimates.jsonl",
        "runs.jsonl",
        "rare-cases.jsonl",
        "anchor-signals.jsonl",
        "multipliers.json",
        "aggregates.json",
        "rotation-state.json",
    ];
    let forbidden = [
        "raw transcript",
        "raw prompt",
        "provider payload",
        "Authorization:",
        "Bearer ",
        "BEGIN PRIVATE KEY",
    ];
    for f in scan_files {
        let p = velocity_dir.join(f);
        if !p.exists() {
            continue;
        }
        let text = fs::read_to_string(&p).unwrap();
        for needle in forbidden {
            assert!(
                !text.contains(needle),
                "{f} unexpectedly contains forbidden marker: {needle}"
            );
        }
        assert!(
            !text.to_lowercase().contains("telemetry"),
            "{f} unexpectedly contains telemetry field"
        );
    }

    for dir in ["fake-home", "fake-xdg", "fake-codex-home"] {
        let entries: Vec<_> = fs::read_dir(target.join(dir))
            .unwrap()
            .map(|e| e.unwrap().path())
            .collect();
        assert!(
            entries.is_empty(),
            "{dir} unexpectedly received files: {entries:?}"
        );
    }
}
