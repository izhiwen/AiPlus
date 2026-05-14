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

fn prepare(temp: &Path) {
    fs::create_dir(temp.join("fake-home")).unwrap();
    fs::create_dir(temp.join("fake-codex-home")).unwrap();
    fs::create_dir(temp.join("fake-xdg")).unwrap();
}

#[test]
fn install_writes_hooks_and_claude_md_and_subagents() {
    let temp = tempfile::tempdir().unwrap();
    let target = temp.path();
    prepare(target);

    run(target, &["install", "claude-code"], 0);

    for p in [
        ".claude/agents/aiplus-advisor.md",
        ".claude/agents/aiplus-memory.md",
        ".claude/agents/aiplus-compact.md",
        ".claude/agents/aiplus-velocity.md",
        ".claude/agents/aiplus-team-consultant.md",
        ".claude/commands/aiplus-refresh.md",
        ".claude/settings.local.json",
        "CLAUDE.md",
    ] {
        assert!(target.join(p).exists(), "missing {p}");
    }

    let hooks: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(target.join(".claude/settings.local.json")).unwrap())
            .unwrap();
    for event in ["SessionStart", "PreCompact"] {
        let arr = hooks
            .pointer(&format!("/hooks/{event}"))
            .and_then(|v| v.as_array())
            .unwrap_or_else(|| panic!("missing /hooks/{event}"));
        assert!(
            arr.iter()
                .any(|m| m.get("aiplus_managed").and_then(|v| v.as_bool()) == Some(true)),
            "{event} has no aiplus_managed matcher"
        );
    }

    let claude_md = fs::read_to_string(target.join("CLAUDE.md")).unwrap();
    assert_eq!(
        claude_md.matches("<!-- BEGIN AIPLUS MANAGED BLOCK -->").count(),
        1
    );
    assert_eq!(
        claude_md.matches("<!-- END AIPLUS MANAGED BLOCK -->").count(),
        1
    );

    for sub in ["memory", "compact", "velocity", "team-consultant"] {
        let body =
            fs::read_to_string(target.join(format!(".claude/agents/aiplus-{sub}.md"))).unwrap();
        assert!(
            body.starts_with("---\nname: aiplus-"),
            "aiplus-{sub}.md missing frontmatter"
        );
        assert!(
            body.contains("description:"),
            "aiplus-{sub}.md missing description"
        );
    }

    // Reinstall is idempotent (same content, no error).
    let before = fs::read_to_string(target.join(".claude/settings.local.json")).unwrap();
    run(target, &["install", "claude-code"], 0);
    let after = fs::read_to_string(target.join(".claude/settings.local.json")).unwrap();
    assert_eq!(before, after, "settings.local.json changed on reinstall");
}

#[test]
fn install_preserves_user_hooks_and_permissions() {
    let temp = tempfile::tempdir().unwrap();
    let target = temp.path();
    prepare(target);

    fs::create_dir_all(target.join(".claude")).unwrap();
    fs::write(
        target.join(".claude/settings.local.json"),
        r#"{
          "permissions": { "allow": ["Bash(ls *)"] },
          "hooks": {
            "Stop": [
              { "matcher": "*", "hooks": [{ "type": "command", "command": "echo user-stop" }] }
            ],
            "SessionStart": [
              { "matcher": "*", "hooks": [{ "type": "command", "command": "echo user-start" }] }
            ]
          }
        }"#,
    )
    .unwrap();

    run(target, &["install", "claude-code"], 0);

    let value: serde_json::Value = serde_json::from_str(
        &fs::read_to_string(target.join(".claude/settings.local.json")).unwrap(),
    )
    .unwrap();

    // User Stop hook untouched.
    let stop = value.pointer("/hooks/Stop").unwrap().as_array().unwrap();
    assert_eq!(stop.len(), 1);
    assert_eq!(
        stop[0].pointer("/hooks/0/command").unwrap().as_str(),
        Some("echo user-stop")
    );

    // SessionStart has both user and AiPlus entries.
    let session = value.pointer("/hooks/SessionStart").unwrap().as_array().unwrap();
    assert_eq!(session.len(), 2);
    let has_user = session.iter().any(|m| {
        m.pointer("/hooks/0/command").and_then(|v| v.as_str()) == Some("echo user-start")
    });
    let has_aiplus = session.iter().any(|m| {
        m.get("aiplus_managed").and_then(|v| v.as_bool()) == Some(true)
    });
    assert!(has_user, "lost user SessionStart hook");
    assert!(has_aiplus, "missing aiplus SessionStart hook");

    // Permissions preserved.
    assert_eq!(
        value
            .pointer("/permissions/allow/0")
            .and_then(|v| v.as_str()),
        Some("Bash(ls *)")
    );
}

#[test]
fn install_preserves_existing_claude_md_content_above_block() {
    let temp = tempfile::tempdir().unwrap();
    let target = temp.path();
    prepare(target);

    let preexisting = "## Project Notes\nKeep this content.\n";
    fs::write(target.join("CLAUDE.md"), preexisting).unwrap();

    run(target, &["install", "claude-code"], 0);

    let body = fs::read_to_string(target.join("CLAUDE.md")).unwrap();
    assert!(
        body.starts_with("## Project Notes\nKeep this content."),
        "user content not preserved:\n{body}"
    );
    assert!(body.contains("<!-- BEGIN AIPLUS MANAGED BLOCK -->"));
    assert!(body.contains("<!-- END AIPLUS MANAGED BLOCK -->"));
}

#[test]
fn uninstall_strips_managed_block_and_aiplus_hooks_only() {
    let temp = tempfile::tempdir().unwrap();
    let target = temp.path();
    prepare(target);

    run(target, &["install", "claude-code"], 0);

    // Add user content next to the AiPlus block.
    let claude_md = fs::read_to_string(target.join("CLAUDE.md")).unwrap();
    fs::write(
        target.join("CLAUDE.md"),
        format!("## My Notes\nPre-existing.\n\n{claude_md}"),
    )
    .unwrap();

    // Add a user Stop hook to settings.local.json.
    let mut settings: serde_json::Value = serde_json::from_str(
        &fs::read_to_string(target.join(".claude/settings.local.json")).unwrap(),
    )
    .unwrap();
    settings
        .pointer_mut("/hooks")
        .unwrap()
        .as_object_mut()
        .unwrap()
        .insert(
            "Stop".to_string(),
            serde_json::json!([
                { "matcher": "*", "hooks": [{ "type": "command", "command": "echo keep-me" }] }
            ]),
        );
    fs::write(
        target.join(".claude/settings.local.json"),
        serde_json::to_string_pretty(&settings).unwrap() + "\n",
    )
    .unwrap();

    run(target, &["uninstall", "--yes"], 0);

    // .aiplus removed.
    assert!(!target.join(".aiplus").exists());

    // CLAUDE.md preserved, block removed.
    let body = fs::read_to_string(target.join("CLAUDE.md")).unwrap();
    assert!(body.contains("## My Notes"));
    assert!(body.contains("Pre-existing."));
    assert!(!body.contains("<!-- BEGIN AIPLUS MANAGED BLOCK -->"));
    assert!(!body.contains("AiPlus is installed in this project"));

    // settings.local.json: user Stop hook preserved, AiPlus events stripped.
    let value: serde_json::Value = serde_json::from_str(
        &fs::read_to_string(target.join(".claude/settings.local.json")).unwrap(),
    )
    .unwrap();
    let stop = value.pointer("/hooks/Stop").unwrap().as_array().unwrap();
    assert_eq!(stop.len(), 1);
    assert_eq!(
        stop[0].pointer("/hooks/0/command").unwrap().as_str(),
        Some("echo keep-me")
    );
    assert!(
        value.pointer("/hooks/SessionStart").is_none()
            || value
                .pointer("/hooks/SessionStart")
                .unwrap()
                .as_array()
                .map(|a| a.is_empty())
                .unwrap_or(true),
        "SessionStart should be empty or absent after uninstall"
    );
    assert!(
        value.pointer("/hooks/PreCompact").is_none(),
        "PreCompact should be removed after uninstall"
    );
}

#[test]
fn doctor_passes_after_install() {
    let temp = tempfile::tempdir().unwrap();
    let target = temp.path();
    prepare(target);

    run(target, &["install", "claude-code"], 0);
    let out = run(target, &["doctor"], 0);
    let text = String::from_utf8_lossy(&out.stdout);
    assert!(text.contains("DOCTOR_STATUS=PASS"));
    assert!(text.contains("PASS .claude/agents/aiplus-memory.md exists"));
    assert!(text.contains("PASS .claude/agents/aiplus-compact.md exists"));
    assert!(text.contains("PASS .claude/agents/aiplus-velocity.md exists"));
    assert!(text.contains("PASS .claude/agents/aiplus-team-consultant.md exists"));
    assert!(text.contains("PASS .claude/settings.local.json parses as JSON"));
    assert!(text.contains("PASS settings.local.json has AiPlus SessionStart hook"));
    assert!(text.contains("PASS settings.local.json has AiPlus PreCompact hook"));
    assert!(text.contains("PASS CLAUDE.md contains exactly one AiPlus managed block"));
}
