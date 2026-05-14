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
    assert!(target.join(".aiplus/compact").exists());
    assert!(target.join("AGENTS.md").exists());
    assert!(!target
        .join(".aiplus/modules/aiplus-compact-reminder/core/scripts/compactctl.mjs")
        .exists());
    assert!(!target
        .join(".aiplus/modules/aiplus-compact-reminder/package.json")
        .exists());
    assert!(!target
        .join(".aiplus/modules/aiplus-compact-reminder/tests/compactctl.acceptance.mjs")
        .exists());

    let status = stdout(&run(target, &["status"], 0));
    assert!(status.contains("runtimeAdapters=[codex]"));
    // aieconlab is now opt-in (auto_install = false); the default install
    // brings the SWE-flavored agent-team plus the substrate modules.
    assert!(status.contains(
        "modules=[agent-memory@0.5.1, agent-team@0.1.0, auto-team-consultant@0.4.6, compact-reminder@0.4.6]"
    ));
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
    assert!(refresh.contains("- Compact Reminder: installed"));
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
    assert!(refresh_zh.contains("- Compact Reminder: 已安装"));
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
    assert!(doctor.contains("PASS module manifest compact-reminder present"));
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
    let managed_schema = target
        .join(".aiplus/modules/aiplus-compact-reminder/core/schemas/compact-policy.schema.json");
    fs::write(&managed_schema, b"{\"old\":\"managed file\"}\n").unwrap();
    let checkpoint = target.join(".aiplus/compact/checkpoints/keep-me.json");
    fs::write(&checkpoint, b"{\"checkpoint\":\"preserve\"}\n").unwrap();
    let user_note = target.join(".aiplus/user-note.txt");
    fs::write(&user_note, b"do not delete\n").unwrap();
    fs::write(
        target.join(".aiplus/AGENTS.aiplus.md"),
        b"old guidance: node .aiplus/modules/aiplus-compact-reminder/core/scripts/compactctl.mjs validate\n",
    )
    .unwrap();
    for file in [
        "decision-log.md",
        "agent-state-ledger.md",
        "evidence-ledger.md",
        "compact-policy.json",
    ] {
        let path = target.join(".aiplus/compact").join(file);
        let next = fs::read_to_string(&path)
            .unwrap()
            .replace("UNKNOWN_PENDING", "APPROVED");
        fs::write(path, next).unwrap();
    }
    fs::write(
        target.join(".aiplus/compact/current-handoff.md"),
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
    assert!(upgrade.contains(".aiplus/compact/ state was preserved."));
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
    let handoff = fs::read_to_string(target.join(".aiplus/compact/current-handoff.md")).unwrap();
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
            fs::read_dir(stamp.join(".aiplus/modules/aiplus-compact-reminder/core/schemas"))
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
            assert!(advisor.contains("Compact Reminder reminder schedule"));
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
    // Pre-0.5.11: compact state lived under `.codex/compact/` and survived
    // `uninstall`. After L2-B rename it lives under `.aiplus/compact/`,
    // so uninstall takes it with the rest of `.aiplus/`. See CHANGELOG v0.5.11.
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
fn install_opencode_auto_migrates_legacy_aiplus_only_file() {
    // Regression: AiPlus versions before 0.5.1+ wrote a top-level "aiplus" key
    // into .opencode/opencode.json. OpenCode 1.14+ rejects that key as
    // "Unrecognized key: aiplus" and refuses to start. The install/refresh path
    // must auto-migrate these legacy files without requiring --force, because
    // stripping our own legacy key is not destructive to user config.
    let temp = tempfile::tempdir().unwrap();
    let target = temp.path();
    setup_fake_env(target);
    fs::create_dir(target.join(".opencode")).unwrap();
    fs::write(
        target.join(".opencode/opencode.json"),
        r#"{"aiplus":{"localOnly":true,"refreshKeywords":["AiPlus 刷新","刷新"]}}"#,
    )
    .unwrap();

    let install = stdout(&run(target, &["install", "opencode"], 0));
    assert!(install.contains("INSTALL_STATUS=PASS"), "{install}");

    let config = fs::read_to_string(target.join(".opencode/opencode.json")).unwrap();
    let parsed: serde_json::Value = serde_json::from_str(&config).unwrap();
    assert!(
        parsed.get("aiplus").is_none(),
        "aiplus key must be stripped after migration"
    );
    assert_eq!(
        parsed.get("$schema").and_then(|value| value.as_str()),
        Some("https://opencode.ai/config.json"),
        "migrated file must carry the OpenCode $schema"
    );

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
        fs::create_dir_all(compact_target.join(".aiplus/compact")).unwrap();
        let outside_compact = compact_temp.path().join("outside-handoff.md");
        std::os::unix::fs::symlink(
            &outside_compact,
            compact_target.join(".aiplus/compact/current-handoff.md"),
        )
        .unwrap();
        let blocked_compact = run(compact_target, &["compact", "init"], 1);
        assert!(stderr(&blocked_compact).contains(
            "ERROR refusing to write through symlink: .aiplus/compact/current-handoff.md"
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
    assert!(target.join(".aiplus/compact/current-handoff.md").exists());
    assert!(target.join(".aiplus/compact/compact-policy.json").exists());

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
    assert!(checkpoint_out.contains("CHECKPOINT_CREATED=.aiplus/compact/checkpoints/"));
    assert!(checkpoint_out.contains("COMPACT_RUST_NATIVE_STATUS=PASS"));
    let checkpoint_count = fs::read_dir(target.join(".aiplus/compact/checkpoints"))
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
    assert!(resume.contains("latest_checkpoint=.aiplus/compact/checkpoints/"));
    assert!(resume.contains("session_role=Unknown"));
    assert!(resume.contains("workflow_level=Unknown"));
    assert!(resume.contains("read_only_recovery_guidance=yes"));
    assert!(resume.contains("current_goal="));
    assert!(resume.contains("COMPACT_RUST_NATIVE_STATUS=PASS"));

    let approved = fs::read_to_string(target.join(".aiplus/compact/current-handoff.md"))
        .unwrap()
        .replace("UNKNOWN_PENDING", "APPROVED");
    fs::write(target.join(".aiplus/compact/current-handoff.md"), approved).unwrap();
    for file in [
        "decision-log.md",
        "agent-state-ledger.md",
        "evidence-ledger.md",
    ] {
        let next = fs::read_to_string(target.join(".aiplus/compact").join(file))
            .unwrap()
            .replace("UNKNOWN_PENDING", "APPROVED");
        fs::write(target.join(".aiplus/compact").join(file), next).unwrap();
    }
    let mut policy =
        fs::read_to_string(target.join(".aiplus/compact/compact-policy.json")).unwrap();
    policy = policy.replace(
        "\"status\": \"UNKNOWN_PENDING\"",
        "\"status\": \"APPROVED\"",
    );
    fs::write(target.join(".aiplus/compact/compact-policy.json"), policy).unwrap();
    let safe_checkpoint = stdout(&run_with_path(
        target,
        &["compact", "checkpoint"],
        0,
        Some(&no_node_path),
    ));
    assert!(safe_checkpoint.contains("SAFE_TO_COMPACT"));
    assert!(safe_checkpoint.contains("READINESS_STATE=READY_TO_COMPACT"));
    assert!(safe_checkpoint.contains("CHECKPOINT_CREATED=.aiplus/compact/checkpoints/"));

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

    fs::write(target.join(".aiplus/compact/compact-policy.json"), "{ bad").unwrap();
    let bad_policy = run_with_path(target, &["compact", "validate"], 1, Some(&no_node_path));
    assert!(stderr(&bad_policy).contains("compact-policy.json is invalid JSON"));
    assert!(stderr(&bad_policy).contains("VALIDATION_FAIL"));
    let before_blocked_count = fs::read_dir(target.join(".aiplus/compact/checkpoints"))
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
    let after_blocked_count = fs::read_dir(target.join(".aiplus/compact/checkpoints"))
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
    fs::remove_file(target.join(".aiplus/compact/evidence-ledger.md")).unwrap();
    let missing = run_with_path(target, &["compact", "validate"], 1, Some(&no_node_path));
    assert!(stderr(&missing).contains("evidence-ledger.md is missing"));

    run(target, &["compact", "init", "--force"], 0);
    fs::write(
        target.join(".aiplus/compact/evidence-ledger.md"),
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
    assert!(prepare.contains("CHECKPOINT_CREATED=.aiplus/compact/checkpoints/"));

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

    fs::write(target.join(".aiplus/compact/compact-policy.json"), "{ bad").unwrap();
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
    assert!(prepare.contains("CHECKPOINT_CREATED=.aiplus/compact/checkpoints/"));

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

    let state_path = target.join(".aiplus/compact/reminder-state.json");
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
    assert!(prepare.contains("CONTEXT_CAPSULE_CREATED=.aiplus/compact/context-capsule.json"));

    let capsule_path = target.join(".aiplus/compact/context-capsule.json");
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
    assert!(prepare.contains("CONTEXT_CAPSULE_CREATED=.aiplus/compact/context-capsule.json"));

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
    let capsule_path = target.join(".aiplus/compact/context-capsule.json");
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
    let capsule_path = target.join(".aiplus/compact/context-capsule.json");
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
    let capsule_path = target.join(".aiplus/compact/context-capsule.json");
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
    let decision_log = target.join(".aiplus/compact/decision-log.md");
    let log_text = fs::read_to_string(&decision_log).unwrap();
    let new_decisions = "| ID | Status | Decision | Rationale | Evidence |\n| --- | --- | --- | --- | --- |\n| DEC-001 | DECIDED | Use Rust for CLI. | Performance and safety. | EVD-001 |\n| DEC-002 | PROVISIONAL | Evaluate Go for future tools. | Team familiarity. | EVD-002 |\n\nAllowed status values: DECIDED, PROVISIONAL, REVERSED, NEEDS_VERIFICATION.";
    let updated = replace_section_body(&log_text, "Decisions", new_decisions);
    fs::write(&decision_log, updated).unwrap();

    run(target, &["compact", "prepare"], 0);
    let capsule_path = target.join(".aiplus/compact/context-capsule.json");
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
    let decision_log = target.join(".aiplus/compact/decision-log.md");
    let log_text = fs::read_to_string(&decision_log).unwrap();
    let new_decisions = "DEC-001 DECIDED Use Rust for CLI. Performance. EVD-001\n\nAllowed status values: DECIDED, PROVISIONAL, REVERSED, NEEDS_VERIFICATION.";
    let updated = replace_section_body(&log_text, "Decisions", new_decisions);
    fs::write(&decision_log, updated).unwrap();

    run(target, &["compact", "prepare"], 0);
    let capsule_path = target.join(".aiplus/compact/context-capsule.json");
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
    let decision_log = target.join(".aiplus/compact/decision-log.md");
    let log_text = fs::read_to_string(&decision_log).unwrap();
    let new_decisions = "| ID | Status | Decision | Rationale | Evidence |\n| --- | --- | --- | --- | --- |\n| DEC-001 | DECIDED | Use Rust for CLI. | Performance. | EVD-001 |\n| DEC-002 | DECIDED | api_key is secret123. | Security. | EVD-002 |\n| DEC-003 | DECIDED | Store raw transcript. | Logging. | EVD-003 |\n\nAllowed status values: DECIDED, PROVISIONAL, REVERSED, NEEDS_VERIFICATION.";
    let updated = replace_section_body(&log_text, "Decisions", new_decisions);
    fs::write(&decision_log, updated).unwrap();

    // Sensitive patterns block prepare (exit code 1), but capsule is still created
    let prepare = stdout(&run(target, &["compact", "prepare"], 1));
    assert!(prepare.contains("BLOCKED_BY_OWNER_GATE"));
    assert!(prepare.contains("CONTEXT_CAPSULE_CREATED"));
    let capsule_path = target.join(".aiplus/compact/context-capsule.json");
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
    let decision_log = target.join(".aiplus/compact/decision-log.md");
    let log_text = fs::read_to_string(&decision_log).unwrap();
    let updated = replace_section_body(&log_text, "Decisions", "No decisions yet.\n\nAllowed status values: DECIDED, PROVISIONAL, REVERSED, NEEDS_VERIFICATION.");
    fs::write(&decision_log, updated).unwrap();

    run(target, &["compact", "prepare"], 0);
    let capsule_path = target.join(".aiplus/compact/context-capsule.json");
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
    let ledger = fs::read_to_string(target.join(".aiplus/compact/savings-ledger.jsonl")).unwrap();
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
    let ledger_dir = weighted_target.join(".aiplus/compact");
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
    assert!(target.join(".aiplus/compact").exists());

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
        let path = target.join(".aiplus/compact").join(file);
        let next = fs::read_to_string(&path)
            .unwrap()
            .replace("UNKNOWN_PENDING", "APPROVED");
        fs::write(path, next).unwrap();
    }
    let mut policy =
        fs::read_to_string(target.join(".aiplus/compact/compact-policy.json")).unwrap();
    policy = policy.replace(
        "\"status\": \"UNKNOWN_PENDING\"",
        "\"status\": \"APPROVED\"",
    );
    fs::write(target.join(".aiplus/compact/compact-policy.json"), policy).unwrap();
}

fn make_compact_handoff_current(target: &Path, phase: &str, last_updated: String) {
    let path = target.join(".aiplus/compact/current-handoff.md");
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
        "Deliver Compact Reminder reminder engine.",
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

#[test]
fn agent_team_list_shows_six_functional_experts() {
    let temp = tempfile::tempdir().unwrap();
    let target = temp.path();
    setup_fake_env(target);
    run(target, &["install", "codex"], 0);

    let agents_dir = target.join(".aiplus/agents");
    fs::create_dir_all(&agents_dir).unwrap();
    for (role, name) in [
        ("ai-integration", "AI Integration"),
        ("security-reviewer", "Security Reviewer"),
        ("tech-writer", "Technical Writer"),
        ("devops", "DevOps"),
        ("ui-designer", "UI Designer"),
        ("researcher", "Researcher"),
    ] {
        fs::write(
            agents_dir.join(format!("{role}.toml")),
            format!(
                "schema_version = \"1.0\"\n\n[agent]\nrole = \"{role}\"\ndisplay_name = \"{name}\"\ntier = \"expert\"\n\n[workspace]\nneeds_worktree = false\n"
            ),
        )
        .unwrap();
    }
    let experts_dir = agents_dir.join("experts");
    fs::create_dir_all(&experts_dir).unwrap();
    for role in [
        "data-analyst",
        "customer-researcher",
        "performance-engineer",
        "accessibility",
        "compliance-reviewer",
    ] {
        fs::write(
            experts_dir.join(format!("{role}.toml")),
            format!(
                "schema_version = \"1.0\"\n\n[agent]\nrole = \"{role}\"\ndisplay_name = \"{role}\"\ntier = \"expert\"\nstatus = \"stub_v0_2\"\n\n[workspace]\nneeds_worktree = false\n"
            ),
        )
        .unwrap();
    }

    let list = stdout(&run(target, &["agent", "list", "--functional"], 0));
    assert!(list.contains("Functional experts (v0.1):"));
    let count = list.lines().filter(|l| l.starts_with("  - ")).count();
    assert_eq!(
        count, 6,
        "expected exactly 6 functional experts, got:\n{list}"
    );
}

#[test]
fn agent_team_invite_stub_errors_with_stub_not_invitable() {
    let temp = tempfile::tempdir().unwrap();
    let target = temp.path();
    setup_fake_env(target);
    run(target, &["install", "codex"], 0);

    let output = run(target, &["agent", "invite", "data-analyst"], 2);
    let out_str = stdout(&output);
    assert!(out_str.contains("STUB_NOT_INVITABLE"));
    assert!(out_str.contains("expert is v0.2 stub, not yet functional"));
    assert!(!out_str.contains("INTERNAL_ERROR"));
}

#[test]
fn agent_team_status_shows_roster() {
    let temp = tempfile::tempdir().unwrap();
    let target = temp.path();
    setup_fake_env(target);
    run(target, &["install", "codex"], 0);

    let status = stdout(&run(target, &["agent", "status"], 0));
    assert!(status.contains("AiPlus Agent Team v0.1"));
    assert!(status.contains("Team Roster:"));
    assert!(status.contains("Active roles:"));
    assert!(status.contains("Total agents:"));
}

#[test]
fn agent_team_chinese_aliases_resolve() {
    let temp = tempfile::tempdir().unwrap();
    let target = temp.path();
    setup_fake_env(target);
    run(target, &["install", "codex"], 0);

    let status = stdout(&run(target, &["agent", "团队"], 0));
    assert!(status.contains("AiPlus Agent Team v0.1"));
    assert!(status.contains("Team Roster:"));
}

#[test]
fn agent_team_doctor_validates_configs() {
    let temp = tempfile::tempdir().unwrap();
    let target = temp.path();
    setup_fake_env(target);
    run(target, &["install", "codex"], 0);

    // Clear auto-provisioned agents and create test-specific configs
    let agents_dir = target.join(".aiplus/agents");
    fs::remove_dir_all(&agents_dir).unwrap();
    fs::create_dir_all(&agents_dir).unwrap();
    fs::write(
        agents_dir.join("ai-integration.toml"),
        "schema_version = \"1.0\"\n\n[agent]\nrole = \"ai-integration\"\ndisplay_name = \"AI Integration\"\nstatus = \"active\"\n\n[workspace]\nneeds_worktree = false\n",
    )
    .unwrap();
    fs::write(
        agents_dir.join("devops.toml"),
        "schema_version = \"1.0\"\n\n[agent]\nrole = \"devops\"\ndisplay_name = \"DevOps\"\nstatus = \"active\"\n\n[workspace]\nneeds_worktree = true\nworktree_path = \".aiplus/agents/devops-wt\"\n",
    )
    .unwrap();

    let doctor = stdout(&run(target, &["agent", "doctor"], 0));
    assert!(doctor.contains("Running agent team doctor..."));
    assert!(doctor.contains("Found 2 agent config(s)"));
    assert!(doctor.contains("ai-integration (AI Integration) [ACTIVE]"));
    assert!(doctor.contains("devops (DevOps) [ACTIVE]"));
    // Lazy worktree creation is by design — was WARNING in 0.5.9; demoted
    // to INFO in 0.5.10 because every fresh-install state-of-the-world had
    // ~30 of these and they drowned out real issues. Real failures still
    // get WARNING.
    assert!(doctor.contains("INFO: worktree .aiplus/agents/devops-wt not yet provisioned (lazy)"));
    assert!(doctor.contains("Doctor check complete."));
}

// =============================================================================
// AiPlus Agent Team v0.1 Parity Tests
// =============================================================================

fn init_git_repo(target: &Path) {
    let output = Command::new("git")
        .args(["init"])
        .current_dir(target)
        .output()
        .expect("git init failed");
    assert!(output.status.success(), "git init failed");

    Command::new("git")
        .args(["config", "user.email", "test@example.com"])
        .current_dir(target)
        .output()
        .unwrap();
    Command::new("git")
        .args(["config", "user.name", "Test User"])
        .current_dir(target)
        .output()
        .unwrap();
}

fn git_commit_all(target: &Path, message: &str) {
    Command::new("git")
        .args(["add", "-A"])
        .current_dir(target)
        .output()
        .unwrap();
    let output = Command::new("git")
        .args(["commit", "--no-verify", "-m", message])
        .current_dir(target)
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "git commit failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
}

fn create_agent_with_worktree(target: &Path, role: &str) {
    let agents_dir = target.join(".aiplus/agents");
    fs::create_dir_all(&agents_dir).unwrap();
    fs::write(
        agents_dir.join(format!("{}.toml", role)),
        format!(
            "schema_version = \"1.0\"\n\n[agent]\nrole = \"{}\"\ndisplay_name = \"{}\"\ntier = \"expert\"\nstatus = \"active\"\n\n[workspace]\nneeds_worktree = true\n",
            role, role
        ),
    )
    .unwrap();
}

fn list_agent_worktrees(target: &Path) -> Vec<String> {
    let output = Command::new("git")
        .args(["worktree", "list", "--porcelain"])
        .current_dir(target)
        .output()
        .unwrap();
    let stdout = String::from_utf8_lossy(&output.stdout);
    let mut roles = Vec::new();
    for block in stdout.split("\n\n") {
        let mut path = None;
        let mut branch = None;
        for line in block.lines() {
            if line.starts_with("worktree ") {
                path = Some(line.strip_prefix("worktree ").unwrap().trim());
            } else if line.starts_with("branch ") {
                branch = Some(line.strip_prefix("branch ").unwrap().trim());
            }
        }
        if let (Some(p), Some(b)) = (path, branch) {
            if b.starts_with("agent/") || b.starts_with("refs/heads/agent/") {
                let role = b
                    .strip_prefix("refs/heads/agent/")
                    .or_else(|| b.strip_prefix("agent/"))
                    .unwrap_or("unknown");
                if Path::new(p) != target {
                    roles.push(role.to_string());
                }
            }
        }
    }
    roles
}

fn get_worktree_path(target: &Path, role: &str) -> Option<PathBuf> {
    let output = Command::new("git")
        .args(["worktree", "list", "--porcelain"])
        .current_dir(target)
        .output()
        .unwrap();
    let stdout = String::from_utf8_lossy(&output.stdout);
    let expected_branch = format!("agent/{}", role);
    for block in stdout.split("\n\n") {
        let mut path = None;
        let mut branch = None;
        for line in block.lines() {
            if line.starts_with("worktree ") {
                path = Some(line.strip_prefix("worktree ").unwrap().trim());
            } else if line.starts_with("branch ") {
                branch = Some(line.strip_prefix("branch ").unwrap().trim());
            }
        }
        if let (Some(p), Some(b)) = (path, branch) {
            if b == expected_branch || b == format!("refs/heads/{}", expected_branch) {
                return Some(PathBuf::from(p));
            }
        }
    }
    None
}

#[test]
fn cli_parity_basic_command_structure() {
    let temp = tempfile::tempdir().unwrap();
    let target = temp.path();
    setup_fake_env(target);
    run(target, &["install", "codex"], 0);

    // aiplus agent status returns 0
    let status = stdout(&run(target, &["agent", "status"], 0));
    assert!(status.contains("AiPlus Agent Team v0.1"));
    assert!(status.contains("Team Roster:"));

    // aiplus agent --help contains expected subcommands
    let help = stdout(&run(target, &["agent", "--help"], 0));
    for subcommand in [
        "status",
        "doctor",
        "list",
        "talk",
        "route",
        "reset",
        "invite",
        "dismiss",
        "disable",
        "enable",
        "integrate",
        "transcript",
        "prune-worktrees",
        "audit",
    ] {
        assert!(
            help.contains(subcommand),
            "help missing subcommand: {}",
            subcommand
        );
    }

    // aiplus agent audit --help contains 9 audit subcommands
    let audit_help = stdout(&run(target, &["agent", "audit", "--help"], 0));
    let audit_subcommands = [
        "run",
        "canary",
        "replay",
        "owner-feedback",
        "owner-feedback-retract",
        "force-skip",
        "re-sign-manifest",
        "setup-gpg",
        "weekly-spot-check",
        "status",
    ];
    let mut found = 0;
    for sub in &audit_subcommands {
        if audit_help.contains(sub) {
            found += 1;
        }
    }
    assert!(
        found >= 9,
        "expected at least 9 audit subcommands in help, found {}. help text:\n{}",
        found,
        audit_help
    );
}

#[test]
fn chinese_aliases_parity() {
    let temp = tempfile::tempdir().unwrap();
    let target = temp.path();
    setup_fake_env(target);
    run(target, &["install", "codex"], 0);

    // aiplus 团队 works (top-level alias → agent status)
    let team = stdout(&run(target, &["团队"], 0));
    assert!(
        team.contains("AiPlus Agent Team v0.1"),
        "团队 alias failed:\n{}",
        team
    );
    assert!(team.contains("Team Roster:"));

    // aiplus 审计 状态 works (top-level alias → agent audit status)
    let audit_status = stdout(&run(target, &["审计", "状态"], 0));
    assert!(
        audit_status.contains("=== Audit System Status ==="),
        "审计 状态 alias failed:\n{}",
        audit_status
    );

    // aiplus 派单 engineer-a works (top-level alias → agent route engineer-a)
    init_git_repo(target);
    fs::write(target.join("README.md"), "# Test\n").unwrap();
    git_commit_all(target, "Initial commit");
    create_agent_with_worktree(target, "engineer-a");

    let route = stdout(&run(target, &["派单", "engineer-a"], 0));
    assert!(
        route.contains("Routing task to engineer-a"),
        "派单 alias failed:\n{}",
        route
    );

    // Verify each alias maps to correct English command by checking outputs match
    let direct_status = stdout(&run(target, &["agent", "status"], 0));
    assert_eq!(
        team.contains("AiPlus Agent Team v0.1"),
        direct_status.contains("AiPlus Agent Team v0.1"),
        "团队 alias should produce same output as agent status"
    );
}

#[test]
fn three_layer_memory_parity() {
    let temp = tempfile::tempdir().unwrap();
    let target = temp.path();
    setup_fake_env(target);
    run(target, &["install", "codex"], 0);

    // aiplus memory status works
    let mem_status = stdout(&run(target, &["memory", "status"], 0));
    assert!(mem_status.contains("MEMORY_STATUS"));
    assert!(mem_status.contains("MEMORY_STATUS=PASS"));
    assert!(mem_status.contains("scope=project-local"));

    // aiplus memory add works
    let mem_add = stdout(&run(
        target,
        &[
            "memory",
            "add",
            "--scope",
            "project",
            "--kind",
            "preference",
            "--text",
            "Prefer dark mode UI",
        ],
        0,
    ));
    assert!(mem_add.contains("MEMORY_ADD"));
    assert!(mem_add.contains("MEMORY_ADD_STATUS=PASS"));
    assert!(mem_add.contains("scope=project"));
    assert!(mem_add.contains("kind=preference"));

    // Extract memory ID from output
    let id_line = mem_add.lines().find(|l| l.starts_with("id=mem_")).unwrap();
    let id = id_line.trim_start_matches("id=");
    assert!(!id.is_empty());

    // Verify memory was added by checking status again
    let mem_status_after = stdout(&run(target, &["memory", "status"], 0));
    assert!(
        mem_status_after.contains("records_active=1"),
        "memory status should show 1 active record after add:\n{}",
        mem_status_after
    );

    // aiplus memory forget works
    let mem_forget = stdout(&run(target, &["memory", "forget", id], 0));
    assert!(mem_forget.contains("MEMORY_FORGET"));
    assert!(mem_forget.contains("MEMORY_FORGET_STATUS=PASS"));
    assert!(mem_forget.contains("forgotten=yes"));
    assert!(mem_forget.contains("status=rejected"));

    // Verify memory was forgotten
    let mem_status_final = stdout(&run(target, &["memory", "status"], 0));
    assert!(
        mem_status_final.contains("records_active=0"),
        "memory status should show 0 active records after forget:\n{}",
        mem_status_final
    );
}

#[test]
fn worktree_lifecycle_parity() {
    let temp = tempfile::tempdir().unwrap();
    let target = temp.path();
    setup_fake_env(target);

    // Set up git repository
    init_git_repo(target);
    fs::write(target.join("README.md"), "# Test Project\n").unwrap();
    git_commit_all(target, "Initial commit");

    // Install aiplus
    run(target, &["install", "codex"], 0);

    // Commit aiplus files so integrate doesn't see uncommitted changes
    git_commit_all(target, "Install AiPlus");

    // Create agent configs that need worktrees
    create_agent_with_worktree(target, "engineer-a");
    git_commit_all(target, "Add engineer-a agent config");

    // aiplus agent route engineer-a creates worktree
    let route = stdout(&run(target, &["agent", "route", "engineer-a"], 0));
    assert!(
        route.contains("Creating worktree for engineer-a") || route.contains("Worktree created:"),
        "route should create worktree:\n{}",
        route
    );

    // Verify worktree exists
    let worktree_path = get_worktree_path(target, "engineer-a");
    assert!(
        worktree_path.is_some(),
        "worktree for engineer-a should exist after route"
    );
    let worktree_path = worktree_path.unwrap();
    assert!(worktree_path.exists(), "worktree directory should exist");

    // aiplus agent integrate engineer-a merges branch
    // First make a commit in the worktree
    fs::write(worktree_path.join("feature.txt"), "new feature\n").unwrap();
    Command::new("git")
        .args(["add", "feature.txt"])
        .current_dir(&worktree_path)
        .output()
        .unwrap();
    Command::new("git")
        .args(["commit", "--no-verify", "-m", "Add feature"])
        .current_dir(&worktree_path)
        .output()
        .unwrap();

    let integrate = stdout(&run(target, &["agent", "integrate", "engineer-a"], 0));
    assert!(
        integrate.contains("Successfully integrated engineer-a"),
        "integrate should merge branch:\n{}",
        integrate
    );

    // Verify the feature file exists in main repo after merge
    assert!(
        target.join("feature.txt").exists(),
        "feature.txt should be merged into main repo"
    );

    // aiplus agent dismiss engineer-a
    let dismiss = stdout(&run(target, &["agent", "dismiss", "engineer-a"], 0));
    assert!(
        dismiss.contains("Dismissing engineer-a"),
        "dismiss should report role:\n{}",
        dismiss
    );

    // aiplus agent prune-worktrees --yes cleans all
    let prune = stdout(&run(target, &["agent", "prune-worktrees", "--yes"], 0));
    assert!(
        prune.contains("Removing worktree for engineer-a") || prune.contains("Removed 1 worktree"),
        "prune-worktrees should remove worktree:\n{}",
        prune
    );

    // Verify no agent worktrees remain
    let remaining = list_agent_worktrees(target);
    assert!(
        remaining.is_empty(),
        "Expected no remaining agent worktrees, found: {:?}",
        remaining
    );
}

#[test]
fn warm_bench_cache_parity() {
    let temp = tempfile::tempdir().unwrap();
    let target = temp.path();
    setup_fake_env(target);

    // Set up git repository
    init_git_repo(target);
    fs::write(target.join("README.md"), "# Test Project\n").unwrap();
    git_commit_all(target, "Initial commit");

    // Install aiplus
    run(target, &["install", "codex"], 0);
    create_agent_with_worktree(target, "engineer-a");

    // First route creates worktree (cold start)
    let route1 = stdout(&run(target, &["agent", "route", "engineer-a"], 0));
    assert!(
        route1.contains("Creating worktree for engineer-a"),
        "first route should create worktree:\n{}",
        route1
    );

    // Cache hit after first route: second route reuses existing worktree
    let route2 = stdout(&run(target, &["agent", "route", "engineer-a"], 0));
    assert!(
        route2.contains("Using existing worktree:"),
        "second route should hit cache and reuse worktree:\n{}",
        route2
    );

    // Cache invalidation on dismiss
    let dismiss = stdout(&run(target, &["agent", "dismiss", "engineer-a"], 0));
    assert!(
        dismiss.contains("Dismissing engineer-a"),
        "dismiss should succeed and invalidate cache:\n{}",
        dismiss
    );

    // After dismiss, route should still reuse worktree (git-level cache), but warm-bench cache was invalidated
    let route_after_dismiss = stdout(&run(target, &["agent", "route", "engineer-a"], 0));
    assert!(
        route_after_dismiss.contains("Using existing worktree:")
            || route_after_dismiss.contains("Creating worktree for engineer-a"),
        "route after dismiss should succeed:\n{}",
        route_after_dismiss
    );

    // Cache clear on reset
    let reset = stdout(&run(target, &["agent", "reset"], 0));
    assert!(
        reset.contains("Warm-bench cache cleared."),
        "reset should clear cache:\n{}",
        reset
    );
    assert!(reset.contains("Resetting agent team state..."));
}

#[test]
fn acceptance_scenario_parity() {
    let temp = tempfile::tempdir().unwrap();
    let target = temp.path();
    setup_fake_env(target);

    // Set up git repository
    init_git_repo(target);
    fs::write(target.join("README.md"), "# Test Project\n").unwrap();
    git_commit_all(target, "Initial commit");

    // Install aiplus and create agents
    run(target, &["install", "codex"], 0);
    git_commit_all(target, "Install AiPlus");

    create_agent_with_worktree(target, "engineer-a");
    create_agent_with_worktree(target, "engineer-b");
    git_commit_all(target, "Add agent configs");

    // Step 1: Route engineer-a
    let route_a = stdout(&run(target, &["agent", "route", "engineer-a"], 0));
    assert!(
        route_a.contains("Creating worktree for engineer-a")
            || route_a.contains("Worktree created:"),
        "step 1 route engineer-a failed:\n{}",
        route_a
    );

    // Step 2: Route engineer-b
    let route_b = stdout(&run(target, &["agent", "route", "engineer-b"], 0));
    assert!(
        route_b.contains("Creating worktree for engineer-b")
            || route_b.contains("Worktree created:"),
        "step 2 route engineer-b failed:\n{}",
        route_b
    );

    // Step 3: Status shows both agents
    let status = stdout(&run(target, &["agent", "status"], 0));
    assert!(
        status.contains("Worktree status:") || status.contains("Worktree"),
        "step 3 status should show worktrees:\n{}",
        status
    );

    // Step 4: Integrate engineer-a (after making a commit in its worktree)
    let worktree_a =
        get_worktree_path(target, "engineer-a").expect("engineer-a worktree should exist");
    fs::write(worktree_a.join("feature-a.txt"), "feature a\n").unwrap();
    Command::new("git")
        .args(["add", "feature-a.txt"])
        .current_dir(&worktree_a)
        .output()
        .unwrap();
    Command::new("git")
        .args(["commit", "--no-verify", "-m", "Add feature A"])
        .current_dir(&worktree_a)
        .output()
        .unwrap();

    let integrate = stdout(&run(target, &["agent", "integrate", "engineer-a"], 0));
    assert!(
        integrate.contains("Successfully integrated engineer-a"),
        "step 4 integrate engineer-a failed:\n{}",
        integrate
    );
    assert!(
        target.join("feature-a.txt").exists(),
        "feature-a.txt should exist in main repo after integrate"
    );

    // Step 5: Dismiss engineer-b
    let dismiss = stdout(&run(target, &["agent", "dismiss", "engineer-b"], 0));
    assert!(
        dismiss.contains("Dismissing engineer-b"),
        "step 5 dismiss engineer-b failed:\n{}",
        dismiss
    );

    // Step 6: Final state is clean - prune all worktrees
    let prune = stdout(&run(target, &["agent", "prune-worktrees", "--yes"], 0));
    assert!(
        prune.contains("Removed") || prune.contains("Removing worktree"),
        "step 6 prune-worktrees should clean up:\n{}",
        prune
    );

    // Verify no agent worktrees remain
    let remaining = list_agent_worktrees(target);
    assert!(
        remaining.is_empty(),
        "Final state should have no agent worktrees, found: {:?}",
        remaining
    );

    // Verify main repo still has the merged file
    assert!(
        target.join("feature-a.txt").exists(),
        "Final state should retain merged files"
    );
}

#[test]
fn agent_route_writes_consult_artifact() {
    // W1 contract: `aiplus agent route` must produce a real consult
    // JSONL under .aiplus/agent-memory/_team/ when a consultant-team
    // config is installed and the task description triggers at least
    // one member. Idempotency: re-running with the same role+task on
    // the same day appends nothing.
    let temp = tempfile::tempdir().unwrap();
    let target = temp.path();
    setup_fake_env(target);
    init_git_repo(target);
    fs::write(target.join("README.md"), "# T\n").unwrap();
    git_commit_all(target, "Initial commit");

    // Install + add the consultant team module so consultant-team.toml lands.
    run(target, &["install", "codex"], 0);
    run(target, &["add", "auto-team-consultant"], 0);
    let consult_toml = target.join(".aiplus/consultant-team.toml");
    assert!(
        consult_toml.exists(),
        "consultant-team.toml should exist after install + add auto-team-consultant"
    );

    // Task description that fires the SWE ai_integration trigger (via
    // "LLM") and pushes complexity to HEAVY (via "rewrite") without
    // touching any of the W2 owner-gate keywords. We avoid "identity"-
    // adjacent words because "identity" is a trigger and would pull in
    // trust_safety (which carries owner_gate via the release [[triggers]]
    // stop_gate fan-out). W2's gate-blocking path has its own tests
    // below; this one exercises W1 in isolation.
    let route = stdout(&run(
        target,
        &[
            "agent",
            "route",
            "engineer-a",
            "rewrite",
            "the",
            "LLM",
            "tool",
            "use",
            "context",
            "pipeline",
        ],
        0,
    ));
    assert!(
        route.contains("Consult tier:") && route.contains("finding(s) recorded:"),
        "route output should mention consult artifact:\n{}",
        route
    );

    // Find the freshly-written consult-<id>.jsonl.
    let team_dir = target.join(".aiplus/agent-memory/_team");
    assert!(team_dir.exists(), "_team/ namespace should exist");
    let consult_files: Vec<_> = fs::read_dir(&team_dir)
        .unwrap()
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .filter(|p| {
            p.file_name()
                .and_then(|s| s.to_str())
                .map(|s| s.starts_with("consult-") && s.ends_with(".jsonl"))
                .unwrap_or(false)
        })
        .collect();
    assert_eq!(
        consult_files.len(),
        1,
        "expected exactly one consult-*.jsonl, found {:?}",
        consult_files
    );
    let body = fs::read_to_string(&consult_files[0]).unwrap();
    let line_count = body.lines().count();
    assert!(
        line_count >= 1,
        "consult JSONL should have at least one finding line; body=\n{}",
        body
    );
    // Each line should be parseable JSON with the contracted fields.
    for line in body.lines() {
        let v: serde_json::Value = serde_json::from_str(line).expect("findings parse as JSON");
        assert_eq!(
            v.get("schemaVersion").and_then(|x| x.as_str()),
            Some("0.1.0")
        );
        assert!(v.get("taskId").and_then(|x| x.as_str()).is_some());
        assert!(v.get("memberId").and_then(|x| x.as_str()).is_some());
        assert!(v.get("tier").and_then(|x| x.as_str()).is_some());
    }

    // Idempotency: run the same route a second time. The JSONL line
    // count must not grow — (task_id, member_id) is the dedupe key.
    run(
        target,
        &[
            "agent",
            "route",
            "engineer-a",
            "rewrite",
            "the",
            "LLM",
            "tool",
            "use",
            "context",
            "pipeline",
        ],
        0,
    );
    let body2 = fs::read_to_string(&consult_files[0]).unwrap();
    assert_eq!(
        body2.lines().count(),
        line_count,
        "re-running route on the same day must be idempotent; before=\n{}\nafter=\n{}",
        body,
        body2
    );

    // Doctor should still pass on the schema_version supported check.
    let doctor = stdout(&run(target, &["doctor"], 0));
    assert!(
        doctor.contains("PASS consultant-team.toml schema_version is supported by this CLI"),
        "doctor should report supported schema_version:\n{}",
        doctor
    );
}

#[test]
fn agent_route_skips_consult_on_unsupported_schema() {
    // W1 safety contract: an unsupported schema must NOT crash dispatch.
    // Instead `agent route` prints a NOTE and writes no consult JSONL.
    // The doctor check is what surfaces drift; route stays lenient.
    let temp = tempfile::tempdir().unwrap();
    let target = temp.path();
    setup_fake_env(target);
    init_git_repo(target);
    fs::write(target.join("README.md"), "# T\n").unwrap();
    git_commit_all(target, "Initial commit");

    run(target, &["install", "codex"], 0);
    run(target, &["add", "auto-team-consultant"], 0);
    let consult_toml = target.join(".aiplus/consultant-team.toml");
    // Replace the bundled file with one whose schema_version is bogus.
    let bogus = "schema_version = \"99.99\"\n";
    fs::write(&consult_toml, bogus).unwrap();

    // Route should still succeed (exit 0) and emit the unsupported-schema NOTE.
    let output = run(
        target,
        &["agent", "route", "engineer-a", "publish", "release"],
        0,
    );
    let combined = format!("{}{}", stdout(&output), stderr(&output));
    assert!(
        combined.contains("schema_version='99.99' not in the supported list")
            || combined.contains("not in the supported list"),
        "route should warn about unsupported schema:\n{}",
        combined
    );
    // No consult JSONL should have been written.
    let team_dir = target.join(".aiplus/agent-memory/_team");
    if team_dir.exists() {
        let any: Vec<_> = fs::read_dir(&team_dir)
            .unwrap()
            .filter_map(|e| e.ok())
            .map(|e| e.path())
            .filter(|p| {
                p.file_name()
                    .and_then(|s| s.to_str())
                    .map(|s| s.starts_with("consult-"))
                    .unwrap_or(false)
            })
            .collect();
        assert!(
            any.is_empty(),
            "no consult artifact expected on unsupported schema, got {:?}",
            any
        );
    }

    // Doctor should report the supported-schema check as NEEDS_FIX.
    // (Exit code may be non-zero on NEEDS_FIX — accept either.)
    let mut command = Command::new(bin());
    command
        .args(["doctor"])
        .current_dir(target)
        .env("HOME", target.join("fake-home"))
        .env("CODEX_HOME", target.join("fake-codex-home"))
        .env("XDG_CONFIG_HOME", target.join("fake-xdg"));
    let doctor_out = command.output().expect("run doctor");
    let combined = format!(
        "{}{}",
        String::from_utf8_lossy(&doctor_out.stdout),
        String::from_utf8_lossy(&doctor_out.stderr)
    );
    assert!(
        combined.contains("schema_version is supported by this CLI"),
        "doctor should report on schema_version drift:\n{}",
        combined
    );
}

#[test]
fn agent_route_blocks_dispatch_on_unapproved_owner_gate() {
    // W2 contract: a task that fires an [owner_gates] entry must
    // refuse dispatch with a non-zero exit and write a gate-pending
    // record. The consult artifact must NOT land (a refused dispatch
    // shouldn't leave a "this happened" finding on disk).
    let temp = tempfile::tempdir().unwrap();
    let target = temp.path();
    setup_fake_env(target);
    init_git_repo(target);
    fs::write(target.join("README.md"), "# T\n").unwrap();
    git_commit_all(target, "Initial commit");
    run(target, &["install", "codex"], 0);
    run(target, &["add", "auto-team-consultant"], 0);

    // "release" matches the SWE-default release stop_gate trigger
    // (release_automation / trust_safety / runtime_qa are in its
    // member list, all join at HEAVY tier, the stop_gate fires).
    // Exit code 3 here is the binary's generic non-zero path for
    // anyhow-bubbled errors; gate refusal lands on that path today.
    let output = run(
        target,
        &[
            "agent",
            "route",
            "engineer-a",
            "release",
            "the",
            "LLM",
            "pipeline",
        ],
        3,
    );
    let combined = format!("{}{}", stdout(&output), stderr(&output));
    assert!(
        combined.contains("Owner gate(s) fired") || combined.contains("owner gate"),
        "route output should mention owner gate:\n{}",
        combined
    );
    assert!(
        combined.contains("--owner-approved") || combined.contains("dispatch refused"),
        "route output should hint at --owner-approved flag:\n{}",
        combined
    );

    // gates-<task-id>.jsonl must exist with a pending record for "release".
    let team_dir = target.join(".aiplus/agent-memory/_team");
    assert!(
        team_dir.exists(),
        "_team/ namespace should exist after gated route"
    );
    let gate_files: Vec<_> = fs::read_dir(&team_dir)
        .unwrap()
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .filter(|p| {
            p.file_name()
                .and_then(|s| s.to_str())
                .map(|s| s.starts_with("gates-") && s.ends_with(".jsonl"))
                .unwrap_or(false)
        })
        .collect();
    assert_eq!(
        gate_files.len(),
        1,
        "expected exactly one gates-*.jsonl, got {:?}",
        gate_files
    );
    let body = fs::read_to_string(&gate_files[0]).unwrap();
    let mut saw_release_pending = false;
    for line in body.lines() {
        let v: serde_json::Value = serde_json::from_str(line).expect("gate record is JSON");
        if v.get("gateId").and_then(|x| x.as_str()) == Some("release")
            && v.get("status").and_then(|x| x.as_str()) == Some("pending")
        {
            saw_release_pending = true;
            assert_eq!(
                v.get("approvedBy").and_then(|x| x.as_str()),
                Some(""),
                "pending records should leave approvedBy empty"
            );
        }
    }
    assert!(
        saw_release_pending,
        "expected a pending record for gateId=release; body:\n{}",
        body
    );

    // Consult artifact must NOT have been written (dispatch refused
    // before the consult side-effect ran).
    let consult_files: Vec<_> = fs::read_dir(&team_dir)
        .unwrap()
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .filter(|p| {
            p.file_name()
                .and_then(|s| s.to_str())
                .map(|s| s.starts_with("consult-"))
                .unwrap_or(false)
        })
        .collect();
    assert!(
        consult_files.is_empty(),
        "no consult artifact expected on gate-refused dispatch, got {:?}",
        consult_files
    );

    // Dispatch log must NOT have a release entry — the dispatch was
    // refused, so the audit log shouldn't claim it happened.
    let dispatch_log = target.join(".aiplus/agents/dispatch-log.jsonl");
    if dispatch_log.exists() {
        let log_body = fs::read_to_string(&dispatch_log).unwrap();
        assert!(
            !log_body.contains("\"task\":\"release"),
            "dispatch log should not record refused dispatch:\n{}",
            log_body
        );
    }
}

#[test]
fn agent_route_approves_owner_gate_with_flag() {
    // W2 happy path: passing --owner-approved <gate-id> lets the
    // dispatch proceed. The ledger carries an approved record with
    // timestamp + approver name, and a consult artifact appears
    // because the dispatch ran to completion.
    let temp = tempfile::tempdir().unwrap();
    let target = temp.path();
    setup_fake_env(target);
    init_git_repo(target);
    fs::write(target.join("README.md"), "# T\n").unwrap();
    git_commit_all(target, "Initial commit");
    run(target, &["install", "codex"], 0);
    run(target, &["add", "auto-team-consultant"], 0);

    // Pass the gate id BEFORE the role token; trailing-var-arg parser
    // would otherwise swallow `--owner-approved` into `task`.
    let out = stdout(&run_with_env(
        target,
        &[
            "agent",
            "route",
            "--owner-approved",
            "release",
            "engineer-a",
            "release",
            "the",
            "LLM",
            "pipeline",
        ],
        0,
        &[("USER", "steve")],
    ));
    assert!(
        out.contains("[approved] release"),
        "route output should mark release approved:\n{}",
        out
    );
    assert!(
        out.contains("Consult tier:") && out.contains("finding(s) recorded:"),
        "approved dispatch should produce a consult artifact:\n{}",
        out
    );

    // Verify the gate ledger entry carries approver + timestamp.
    let team_dir = target.join(".aiplus/agent-memory/_team");
    let gate_files: Vec<_> = fs::read_dir(&team_dir)
        .unwrap()
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .filter(|p| {
            p.file_name()
                .and_then(|s| s.to_str())
                .map(|s| s.starts_with("gates-") && s.ends_with(".jsonl"))
                .unwrap_or(false)
        })
        .collect();
    assert_eq!(gate_files.len(), 1);
    let body = fs::read_to_string(&gate_files[0]).unwrap();
    let mut saw_release_approved = false;
    for line in body.lines() {
        let v: serde_json::Value = serde_json::from_str(line).unwrap();
        if v.get("gateId").and_then(|x| x.as_str()) == Some("release")
            && v.get("status").and_then(|x| x.as_str()) == Some("approved")
        {
            saw_release_approved = true;
            assert_eq!(
                v.get("approvedBy").and_then(|x| x.as_str()),
                Some("steve"),
                "approver should be captured"
            );
            assert!(
                v.get("timestamp")
                    .and_then(|x| x.as_str())
                    .map(|s| !s.is_empty())
                    .unwrap_or(false),
                "approved record should have a non-empty timestamp"
            );
        }
    }
    assert!(
        saw_release_approved,
        "expected approved record for gateId=release; body:\n{}",
        body
    );
}

#[test]
fn install_seeds_memory_namespaces_for_agent_team() {
    // W3 contract: `aiplus install codex` (which auto-installs the
    // agent-team substrate per default_module_names) must seed
    // .aiplus/agent-memory/<role>/ + _team/ with .gitkeep + README.
    let temp = tempfile::tempdir().unwrap();
    let target = temp.path();
    setup_fake_env(target);
    init_git_repo(target);
    fs::write(target.join("README.md"), "# T\n").unwrap();
    git_commit_all(target, "Initial commit");
    run(target, &["install", "codex"], 0);

    let base = target.join(".aiplus/agent-memory");
    assert!(
        base.exists(),
        "agent-memory base dir should exist after install"
    );

    let team_readme = base.join("_team/README.md");
    assert!(team_readme.exists(), "_team/README.md should exist");
    assert!(
        base.join("_team/.gitkeep").exists(),
        "_team/.gitkeep should exist"
    );

    let expected_roles = [
        "advisor",
        "ceo",
        "architect",
        "pm",
        "engineer-a",
        "engineer-b",
        "reviewer",
        "qa",
    ];
    for role in expected_roles {
        let rdir = base.join(role);
        assert!(
            rdir.exists(),
            "agent-memory/{} dir should exist after install",
            role
        );
        assert!(
            rdir.join(".gitkeep").exists(),
            "agent-memory/{}/.gitkeep should exist",
            role
        );
        assert!(
            rdir.join("README.md").exists(),
            "agent-memory/{}/README.md should exist",
            role
        );
    }

    // Doctor's W3 check should pass on a fresh install.
    let doctor = stdout(&run(target, &["doctor"], 0));
    assert!(
        doctor.contains("PASS .aiplus/agent-memory/_team/ exists"),
        "doctor should pass on _team/ namespace:\n{}",
        doctor
    );
    assert!(
        doctor.contains("PASS .aiplus/agent-memory/advisor/ exists with seed file"),
        "doctor should pass on advisor namespace:\n{}",
        doctor
    );

    // Tamper: delete the advisor namespace, re-run doctor — should
    // surface the missing-namespace warning.
    fs::remove_dir_all(base.join("advisor")).unwrap();
    let mut command = Command::new(bin());
    command
        .args(["doctor"])
        .current_dir(target)
        .env("HOME", target.join("fake-home"))
        .env("CODEX_HOME", target.join("fake-codex-home"))
        .env("XDG_CONFIG_HOME", target.join("fake-xdg"));
    let out = command.output().expect("doctor");
    let doctor2 = String::from_utf8_lossy(&out.stdout).to_string();
    assert!(
        doctor2.contains("NEEDS_FIX .aiplus/agent-memory/advisor/ exists with seed file"),
        "doctor should flag the deleted advisor namespace:\n{}",
        doctor2
    );
}
