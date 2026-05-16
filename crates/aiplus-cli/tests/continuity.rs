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

    let alpha_add = stdout(&run(
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
    ));
    let alpha_id = alpha_add
        .lines()
        .find_map(|line| line.strip_prefix("id="))
        .unwrap()
        .to_string();
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
    assert!(alpha_context.contains("records_used=1"));
    assert!(alpha_context.contains("records_ignored=0"));
    assert!(alpha_context.contains("sources=[.aiplus/memory/project-memory.jsonl,.aiplus/memory/decisions.jsonl,.aiplus/memory/facts.jsonl]"));
    assert!(alpha_context
        .contains("owner_gates=[publish,deploy,global config,external accounts,secret exposure]"));
    assert!(alpha_context.contains("secret_values=none"));
    assert!(alpha_context.contains("Alpha release summaries"));
    assert!(!alpha_context.contains("Beta reviews"));
    assert!(!alpha_context.contains("Gamma handoffs"));

    let alpha_list = stdout(&run(&alpha, &env, &["memory", "list"], 0));
    assert!(alpha_list.contains("MEMORY_LIST_STATUS=PASS"));
    assert!(alpha_list.contains("records_total=1"));
    assert!(alpha_list.contains(&alpha_id));
    let alpha_recent = stdout(&run(&alpha, &env, &["memory", "recent"], 0));
    assert!(alpha_recent.contains("MEMORY_RECENT_STATUS=PASS"));
    assert!(alpha_recent.contains("limit=5"));
    assert!(alpha_recent.contains(&alpha_id));
    let forget = stdout(&run(&alpha, &env, &["memory", "forget", &alpha_id], 0));
    assert!(forget.contains("MEMORY_FORGET_STATUS=PASS"));
    assert!(forget.contains("forgotten=yes"));
    let alpha_context_after_forget = stdout(&run(
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
    assert!(alpha_context_after_forget.contains("records_used=0"));
    assert!(alpha_context_after_forget.contains("records_ignored=1"));
    assert!(!alpha_context_after_forget.contains("Alpha release summaries"));

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

    let help = stdout(&run(&project, &env, &["identity", "context", "--help"], 0));
    assert!(help.contains("--runtime <RUNTIME>"));
    assert!(help.contains("--with-memory"));
    assert!(help.contains("--memory-budget <MEMORY_BUDGET>"));
    assert!(help.contains("--memory-scope <MEMORY_SCOPE>"));

    let before = stdout(&run(&project, &env, &["identity", "status"], 0));
    assert!(before.contains("installed=no"));
    run(&project, &env, &["identity", "init", "--project"], 0);
    let status = stdout(&run(&project, &env, &["identity", "status"], 0));
    assert!(status.contains("advisor=present"));
    assert!(status.contains("ceo=present"));
    let list = stdout(&run(&project, &env, &["identity", "list"], 0));
    assert!(list.contains("IDENTITY_LIST_STATUS=PASS"));
    assert!(list.contains("advisor=present"));
    assert!(list.contains("ceo=present"));
    let advisor = stdout(&run(
        &project,
        &env,
        &["identity", "context", "--role", "advisor"],
        0,
    ));
    assert!(advisor.contains("role=advisor"));
    assert!(advisor.contains("activation="));
    assert!(advisor.contains("output_contract="));
    assert!(advisor.contains("owner_gates="));
    assert!(advisor.contains("permissions=none"));
    assert!(advisor.contains("identity_grants_permission=no"));
    assert!(advisor.contains("activation_patterns_count="));
    assert!(advisor.contains("role_activation_count=1"));
    assert!(advisor.contains("memory_bundle=none"));
    assert!(advisor.contains("memory_is_instruction=no"));
    assert!(advisor.contains("secret_values=none"));
    assert!(!advisor.contains("ROLE_ACTIVATED"));
    assert!(!advisor.contains("MEMORY_BUNDLE"));
    let ceo = stdout(&run(
        &project,
        &env,
        &["identity", "context", "--role", "ceo"],
        0,
    ));
    assert!(ceo.contains("role=ceo"));
    assert!(ceo.contains("role_name=CEO"));
    assert!(ceo.contains("identity_grants_permission=no"));
}

#[test]
fn identity_context_with_memory_bundle_counts_and_role_personal_alias() {
    let temp = tempfile::tempdir().unwrap();
    let env = fake_env(temp.path());
    let project = temp.path().join("project");
    fs::create_dir_all(&project).unwrap();

    run(
        &project,
        &env,
        &[
            "memory",
            "add",
            "--scope",
            "role-personal",
            "--role",
            "advisor",
            "--kind",
            "preference",
            "--text",
            "Advisor personal memory should stay scoped.",
        ],
        0,
    );
    run(
        &project,
        &env,
        &[
            "memory",
            "add",
            "--scope",
            "team",
            "--kind",
            "preference",
            "--text",
            "Team memory should be counted in bundles.",
        ],
        0,
    );
    run(
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
            "Project memory should be counted in bundles.",
        ],
        0,
    );

    let all = stdout(&run(
        &project,
        &env,
        &[
            "identity",
            "context",
            "--role",
            "advisor",
            "--runtime",
            "codex",
            "--with-memory",
            "--memory-budget",
            "4000",
        ],
        0,
    ));
    assert!(all.contains("memory_bundle=present"));
    assert!(all.contains("MEMORY_BUNDLE"));
    assert!(all.contains("runtime=codex"));
    assert!(all.contains("memory_scope=all"));
    assert!(all.contains("budget=4000"));
    assert!(all.contains("record_load_cap=20"));
    assert!(all.contains("role_personal_total=1"));
    assert!(all.contains("role_personal_used=1"));
    assert!(all.contains("team_total=1"));
    assert!(all.contains("team_used=1"));
    assert!(all.contains("project_total=1"));
    assert!(all.contains("project_used=1"));
    assert!(all.contains("records_total=3"));
    assert!(all.contains("records_used=3"));
    assert!(all.contains("secret_values=none"));
    assert!(all.contains("memory_is_instruction=no"));
    assert!(all.contains("MEMORY_BUNDLE_STATUS=PASS"));
    assert!(!all.contains("ROLE_ACTIVATED"));

    let role_personal = stdout(&run(
        &project,
        &env,
        &[
            "identity",
            "context",
            "--role",
            "advisor",
            "--with-memory",
            "--memory-scope",
            "role-personal",
        ],
        0,
    ));
    assert!(role_personal.contains("memory_scope=personal"));
    assert!(role_personal.contains("memory_scope_input=role-personal"));
    assert!(role_personal.contains("role_personal_total=1"));
    assert!(role_personal.contains("role_personal_used=1"));
    assert!(role_personal.contains("team_total=1"));
    assert!(role_personal.contains("team_used=0"));
    assert!(role_personal.contains("project_total=1"));
    assert!(role_personal.contains("project_used=0"));
}

#[test]
fn memory_role_scopes_and_bounded_context() {
    let temp = tempfile::tempdir().unwrap();
    let env = fake_env(temp.path());
    let project = temp.path().join("project");
    fs::create_dir_all(&project).unwrap();

    let personal_add = stdout(&run(
        &project,
        &env,
        &[
            "memory",
            "add",
            "--scope",
            "personal",
            "--role",
            "engineer-a",
            "--kind",
            "preference",
            "--text",
            "Engineer A prefers narrow implementation diffs.",
        ],
        0,
    ));
    let personal_id = personal_add
        .lines()
        .find_map(|line| line.strip_prefix("id="))
        .unwrap()
        .to_string();
    run(
        &project,
        &env,
        &[
            "memory",
            "add",
            "--scope",
            "team",
            "--kind",
            "preference",
            "--text",
            "Team handoffs must include verification evidence.",
        ],
        0,
    );
    run(
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
            "Project memory remains shared project context.",
        ],
        0,
    );

    assert!(project
        .join(".aiplus/agent-memory/engineer-a/memory.jsonl")
        .exists());
    assert!(project
        .join(".aiplus/agent-memory/_team/memory.jsonl")
        .exists());

    let personal = stdout(&run(
        &project,
        &env,
        &[
            "memory",
            "list",
            "--scope",
            "personal",
            "--role",
            "engineer-a",
        ],
        0,
    ));
    assert!(personal.contains("scope=personal"));
    assert!(personal.contains("role=engineer-a"));
    assert!(personal.contains("records_total=1"));
    assert!(personal.contains("Engineer A prefers"));
    assert!(!personal.contains("Team handoffs"));

    let personal_alias = stdout(&run(
        &project,
        &env,
        &[
            "memory",
            "list",
            "--scope",
            "role-personal",
            "--role",
            "engineer-a",
        ],
        0,
    ));
    assert!(personal_alias.contains("scope=personal"));
    assert!(personal_alias.contains("role=engineer-a"));
    assert!(personal_alias.contains("records_total=1"));

    let team = stdout(&run(
        &project,
        &env,
        &["memory", "list", "--scope", "team"],
        0,
    ));
    assert!(team.contains("scope=team"));
    assert!(team.contains("records_total=1"));
    assert!(team.contains("Team handoffs"));
    assert!(!team.contains("Engineer A prefers"));

    let context = stdout(&run(
        &project,
        &env,
        &[
            "memory",
            "context",
            "--runtime",
            "codex",
            "--role",
            "engineer-a",
            "--budget",
            "4000",
            "--limit",
            "2",
        ],
        0,
    ));
    assert!(context.contains("role=engineer-a"));
    assert!(context.contains("record_load_cap=2"));
    assert!(context.contains("records_used=2"));
    assert!(context.contains("records_ignored=1"));
    assert!(context.contains(".aiplus/agent-memory/engineer-a/memory.jsonl"));
    assert!(context.contains(".aiplus/agent-memory/_team/memory.jsonl"));

    let forgotten = stdout(&run(
        &project,
        &env,
        &[
            "memory",
            "forget",
            &personal_id,
            "--scope",
            "personal",
            "--role",
            "engineer-a",
        ],
        0,
    ));
    assert!(forgotten.contains("MEMORY_FORGET_STATUS=PASS"));
    assert!(forgotten.contains("scope=personal"));
    let personal_after_forget = stdout(&run(
        &project,
        &env,
        &[
            "memory",
            "list",
            "--scope",
            "personal",
            "--role",
            "engineer-a",
        ],
        0,
    ));
    assert!(personal_after_forget.contains("records_total=0"));
}

#[test]
fn identity_context_supports_installed_team_and_ael_roles() {
    let temp = tempfile::tempdir().unwrap();
    let env = fake_env(temp.path());
    let project = temp.path().join("project");
    let agents = project.join(".aiplus/agents");
    fs::create_dir_all(&agents).unwrap();
    fs::write(
        agents.join("engineer-a.toml"),
        r#"
schema_version = "1.0"

[agent]
role = "engineer-a"
display_name = "Engineer A"
tier = "internal"
status = "inactive"

[memory]
personal_dir = ".aiplus/agent-memory/engineer-a"
read_team_memory = true
read_project_memory = true
write_team_memory = false

[invocation]
english_aliases = ["engineer-a", "eng-a"]
chinese_aliases = []
"#,
    )
    .unwrap();
    fs::write(
        agents.join("pi.toml"),
        r#"
schema_version = "1.0"

[agent]
role = "pi"
display_name = "PI"
tier = "owner_facing"
status = "inactive"

[memory]
personal_dir = ".aiplus/agent-memory/pi"
read_team_memory = true
read_project_memory = true
write_team_memory = true

[invocation]
english_aliases = ["pi", "lead-author"]
chinese_aliases = ["主作者"]
"#,
    )
    .unwrap();

    let engineer = stdout(&run(
        &project,
        &env,
        &["identity", "context", "--role", "engineer-a"],
        0,
    ));
    assert!(engineer.contains("role=engineer-a"));
    assert!(engineer.contains("role_name=Engineer A"));
    assert!(engineer.contains("identity_source=.aiplus/agents"));
    assert!(engineer.contains("role_activation_count=1"));
    assert!(!engineer.contains("ROLE_ACTIVATED"));

    let engineer_again = stdout(&run(
        &project,
        &env,
        &["identity", "context", "--role", "engineer-a"],
        0,
    ));
    assert!(engineer_again.contains("role_activation_count=2"));
    assert!(!engineer_again.contains("ROLE_ACTIVATED"));

    fs::write(agents.join("active-team.txt"), "aieconlab\n").unwrap();
    let pi = stdout(&run(
        &project,
        &env,
        &["identity", "context", "--role", "ceo"],
        0,
    ));
    assert!(pi.contains("role=pi"));
    assert!(pi.contains("role_input=ceo"));
    assert!(pi.contains("role_name=PI"));
    assert!(pi.contains("role_activation_count=1"));
    assert!(!pi.contains("ROLE_ACTIVATED"));
}

fn assert_nl_role_trigger_catalog(text: &str) {
    for required in [
        "## Natural-language role triggers",
        "`你是 <role>` / `you are <role>`",
        "`开 <role>` / `做 <role>` / `take <role>` / `take the <role> role`",
        "`转 <role>` / `switch to <role>`",
        "以 CEO 的视角看一下",
        "let me hear from the PI",
        "quote blocks, code blocks, and third-person references",
        "Ask once before binding",
        "AiPlus roles: advisor, ceo, architect, pm, engineer-a, engineer-b, reviewer",
        "AiEconLab roles: advisor, pi, theorist, pm, ra-stata, ra-python",
        "aiplus memory list --scope personal --role <role> --limit 20",
        "aiplus memory list --scope team --limit 20",
        "ROLE_ACTIVATED role=<role> count=<activation_count> schema=v1 runtime=<codex|claude-code|opencode>",
        "ROLE_BIND_REFUSED current_role=<current_role> requested_role=<requested_role> reason=session_already_bound schema=v1",
    ] {
        assert!(text.contains(required), "missing catalog text: {required}");
    }
}

fn opencode_instruction_entries(project: &Path) -> Vec<String> {
    let config = fs::read_to_string(project.join(".opencode/opencode.json")).unwrap();
    let parsed: serde_json::Value = serde_json::from_str(&config).unwrap();
    parsed
        .get("instructions")
        .and_then(|value| value.as_array())
        .unwrap()
        .iter()
        .map(|value| value.as_str().unwrap().to_string())
        .collect()
}

#[test]
fn nl_role_trigger_catalog_installs_for_codex_and_claude_code() {
    let temp = tempfile::tempdir().unwrap();
    let env = fake_env(temp.path());

    let codex_project = temp.path().join("codex-project");
    fs::create_dir_all(&codex_project).unwrap();
    run(&codex_project, &env, &["install", "codex", "--yes"], 0);
    let agents = fs::read_to_string(codex_project.join(".aiplus/AGENTS.aiplus.md")).unwrap();
    assert_nl_role_trigger_catalog(&agents);
    let doctor = stdout(&run(&codex_project, &env, &["doctor"], 0));
    assert!(doctor.contains("nl_role_triggers=PASS"));
    assert!(doctor.contains("PASS nl_role_triggers=PASS"));

    let claude_project = temp.path().join("claude-project");
    fs::create_dir_all(&claude_project).unwrap();
    run(
        &claude_project,
        &env,
        &["install", "claude-code", "--yes"],
        0,
    );
    let agents = fs::read_to_string(claude_project.join(".aiplus/AGENTS.aiplus.md")).unwrap();
    let claude = fs::read_to_string(claude_project.join("CLAUDE.md")).unwrap();
    assert_nl_role_trigger_catalog(&agents);
    assert_nl_role_trigger_catalog(&claude);
    let doctor = stdout(&run(&claude_project, &env, &["doctor"], 0));
    assert!(doctor.contains("nl_role_triggers=PASS"));
    assert!(doctor.contains("PASS nl_role_triggers=PASS"));
}

#[test]
fn doctor_reports_stable_fail_when_nl_role_catalog_is_stale() {
    let temp = tempfile::tempdir().unwrap();
    let env = fake_env(temp.path());
    let project = temp.path().join("project");
    fs::create_dir_all(&project).unwrap();
    run(&project, &env, &["install", "codex", "--yes"], 0);

    let agents_path = project.join(".aiplus/AGENTS.aiplus.md");
    let stale = fs::read_to_string(&agents_path).unwrap().replace(
        "## Natural-language role triggers",
        "## Old role trigger notes",
    );
    fs::write(&agents_path, stale).unwrap();

    let doctor = stdout(&run(&project, &env, &["doctor"], 0));
    assert!(doctor.contains("nl_role_triggers=FAIL_AGENTS_CATALOG_STALE"));
    assert!(doctor.contains("NEEDS_FIX nl_role_triggers=FAIL_AGENTS_CATALOG_STALE"));
    assert!(doctor.contains("DOCTOR_STATUS=NEEDS_FIX"));
}

#[test]
fn opencode_install_writes_project_local_g1_instructions() {
    let temp = tempfile::tempdir().unwrap();
    let env = fake_env(temp.path());
    let project = temp.path().join("project");
    fs::create_dir_all(&project).unwrap();
    run(&project, &env, &["install", "opencode", "--yes"], 0);

    let agents = fs::read_to_string(project.join(".aiplus/AGENTS.aiplus.md")).unwrap();
    let instructions =
        fs::read_to_string(project.join(".opencode/instructions/aiplus.md")).unwrap();
    assert_nl_role_trigger_catalog(&agents);
    assert_nl_role_trigger_catalog(&instructions);
    assert!(instructions.contains("AIPLUS_OPENCODE_G1_ROLE_TRIGGERS_V1"));
    assert!(instructions.contains("Identity grants no permissions"));
    assert!(instructions.contains("machine-level config"));
    assert_eq!(
        opencode_instruction_entries(&project),
        vec!["instructions/aiplus.md".to_string()]
    );

    let doctor = stdout(&run(&project, &env, &["doctor"], 0));
    assert!(doctor.contains("nl_role_triggers=PASS"));
    assert!(doctor.contains("PASS nl_role_triggers=PASS"));
    assert!(doctor
        .contains("PASS .opencode/opencode.json instructions is an array of strings when present"));
    assert!(doctor.contains("PASS .opencode/opencode.json includes AiPlus instructions path"));
    assert!(doctor.contains("PASS .opencode/instructions/aiplus.md exists"));
    assert!(doctor.contains("PASS .opencode/instructions/aiplus.md contains G1 marker and catalog"));
    assert!(doctor.contains(
        "INFO OpenCode live role-trigger validation deferred; project-local instructions only"
    ));
    assert!(doctor.contains("DOCTOR_STATUS=PASS"));
}

#[test]
fn opencode_install_preserves_existing_instructions_without_duplicates() {
    let temp = tempfile::tempdir().unwrap();
    let env = fake_env(temp.path());
    let project = temp.path().join("project");
    fs::create_dir_all(project.join(".opencode")).unwrap();
    fs::write(
        project.join(".opencode/opencode.json"),
        r#"{"theme":"dark","instructions":["CONTRIBUTING.md","docs/guidelines.md"]}"#,
    )
    .unwrap();

    run(&project, &env, &["install", "opencode", "--yes"], 0);
    run(&project, &env, &["install", "opencode", "--yes"], 0);

    let config = fs::read_to_string(project.join(".opencode/opencode.json")).unwrap();
    let parsed: serde_json::Value = serde_json::from_str(&config).unwrap();
    assert_eq!(
        parsed.get("theme").and_then(|value| value.as_str()),
        Some("dark")
    );
    let entries = opencode_instruction_entries(&project);
    assert!(entries.contains(&"CONTRIBUTING.md".to_string()));
    assert!(entries.contains(&"docs/guidelines.md".to_string()));
    assert_eq!(
        entries
            .iter()
            .filter(|entry| entry.as_str() == "instructions/aiplus.md")
            .count(),
        1
    );
}

#[test]
fn opencode_doctor_detects_malformed_instructions_type() {
    let temp = tempfile::tempdir().unwrap();
    let env = fake_env(temp.path());
    let project = temp.path().join("project");
    fs::create_dir_all(&project).unwrap();
    run(&project, &env, &["install", "opencode", "--yes"], 0);
    fs::write(
        project.join(".opencode/opencode.json"),
        r#"{"$schema":"https://opencode.ai/config.json","instructions":"instructions/aiplus.md"}"#,
    )
    .unwrap();

    let doctor = stdout(&run(&project, &env, &["doctor"], 0));
    assert!(doctor.contains(
        "NEEDS_FIX .opencode/opencode.json instructions is an array of strings when present"
    ));
    assert!(doctor.contains("nl_role_triggers=FAIL_OPENCODE_INSTRUCTIONS_TYPE"));
    assert!(doctor.contains("DOCTOR_STATUS=NEEDS_FIX"));
}

#[test]
fn opencode_uninstall_removes_only_aiplus_instruction_entry_and_file() {
    let temp = tempfile::tempdir().unwrap();
    let env = fake_env(temp.path());
    let project = temp.path().join("project");
    fs::create_dir_all(project.join(".opencode/instructions")).unwrap();
    fs::write(project.join(".opencode/instructions/user.md"), "# User\n").unwrap();
    fs::write(
        project.join(".opencode/opencode.json"),
        r#"{"instructions":["README.md"],"theme":"dark"}"#,
    )
    .unwrap();

    run(&project, &env, &["install", "opencode", "--yes"], 0);
    assert!(project.join(".opencode/instructions/aiplus.md").exists());
    run(&project, &env, &["uninstall", "--yes"], 0);

    assert!(!project.join(".opencode/instructions/aiplus.md").exists());
    assert!(project.join(".opencode/instructions/user.md").exists());
    let config = fs::read_to_string(project.join(".opencode/opencode.json")).unwrap();
    let parsed: serde_json::Value = serde_json::from_str(&config).unwrap();
    assert_eq!(
        parsed.get("theme").and_then(|value| value.as_str()),
        Some("dark")
    );
    assert_eq!(
        parsed
            .get("instructions")
            .and_then(|value| value.as_array())
            .unwrap()
            .iter()
            .map(|value| value.as_str().unwrap())
            .collect::<Vec<_>>(),
        vec!["README.md"]
    );
}

#[test]
fn opencode_legacy_aiplus_key_migrates_with_instructions_and_mixed_key_rejects() {
    let temp = tempfile::tempdir().unwrap();
    let env = fake_env(temp.path());
    let legacy = temp.path().join("legacy");
    fs::create_dir_all(legacy.join(".opencode")).unwrap();
    fs::write(
        legacy.join(".opencode/opencode.json"),
        r#"{"aiplus":{"localOnly":true}}"#,
    )
    .unwrap();

    run(&legacy, &env, &["install", "opencode", "--yes"], 0);
    let config = fs::read_to_string(legacy.join(".opencode/opencode.json")).unwrap();
    let parsed: serde_json::Value = serde_json::from_str(&config).unwrap();
    assert!(parsed.get("aiplus").is_none());
    assert_eq!(
        parsed.get("$schema").and_then(|value| value.as_str()),
        Some("https://opencode.ai/config.json")
    );
    assert_eq!(
        opencode_instruction_entries(&legacy),
        vec!["instructions/aiplus.md".to_string()]
    );

    let mixed = temp.path().join("mixed");
    fs::create_dir_all(mixed.join(".opencode")).unwrap();
    fs::write(
        mixed.join(".opencode/opencode.json"),
        r#"{"theme":"dark","aiplus":{"localOnly":true}}"#,
    )
    .unwrap();
    let rejected = run(&mixed, &env, &["install", "opencode", "--yes"], 1);
    assert!(stderr(&rejected).contains("CONFLICT .opencode/opencode.json exists and differs"));
}

#[test]
fn non_opencode_install_does_not_write_opencode_instructions() {
    let temp = tempfile::tempdir().unwrap();
    let env = fake_env(temp.path());
    let project = temp.path().join("project");
    fs::create_dir_all(&project).unwrap();
    run(&project, &env, &["install", "codex", "--yes"], 0);

    assert!(!project.join(".opencode/opencode.json").exists());
    assert!(!project.join(".opencode/instructions/aiplus.md").exists());
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
    assert!(proposed.contains("candidate_is_approved_skill=no"));
    assert!(proposed.contains("approval_requires=qa_and_owner_gate"));
    let id = proposed
        .lines()
        .find_map(|line| line.strip_prefix("id="))
        .unwrap()
        .to_string();
    let status = stdout(&run(&project, &env, &["skill-candidate", "status"], 0));
    assert!(status.contains("candidate_proposed=1"));
    assert!(status.contains("candidate_is_approved_skill=no"));
    assert!(status.contains("approval_requires=qa_and_owner_gate"));
    assert!(status.contains("rejected_auto_load=no"));
    run(&project, &env, &["skill-candidate", "reject", &id], 0);
    let status = stdout(&run(&project, &env, &["skill-candidate", "status"], 0));
    assert!(status.contains("rejected=1"));
    assert!(status.contains("rejected_auto_load=no"));
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
