#[cfg(unix)]
mod unix_tui {
    use rexpect::process::{Signal, WaitStatus};
    use rexpect::session::spawn_command;
    use serde_json::Value;
    use std::env;
    use std::fs;
    use std::path::{Path, PathBuf};
    use std::process::{Command, Output};
    use std::thread;
    use std::time::{Duration, Instant};

    const ENABLE_ENV: &str = "AIPLUS_OPENCODE_TUI_TESTS";
    const KEEP_ENV: &str = "AIPLUS_KEEP_TUI_FIXTURES";
    const READY_ENV: &str = "AIPLUS_OPENCODE_TUI_READY_MARKERS";
    const ROLE_FORBID_TOKENS: &[&str] = &["ROLE_ACTIVATED", "ROLE_BIND_REFUSED", "ROLE_"];

    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    enum ExpectedPolicy {
        Coordinator,
        Builder,
        Reviewer,
    }

    impl ExpectedPolicy {
        fn as_str(self) -> &'static str {
            match self {
                ExpectedPolicy::Coordinator => "coordinator",
                ExpectedPolicy::Builder => "builder",
                ExpectedPolicy::Reviewer => "reviewer",
            }
        }
    }

    #[derive(Debug, Clone, Copy)]
    struct PositiveCase {
        id: &'static str,
        prompt: &'static str,
        expected_role: &'static str,
        requested_role: &'static str,
        expected_policy: ExpectedPolicy,
    }

    #[derive(Debug, Clone, Copy)]
    struct NegativeCase {
        id: &'static str,
        prompt: &'static str,
    }

    #[derive(Debug)]
    struct FakeEnv {
        home: PathBuf,
        xdg: PathBuf,
        codex: PathBuf,
        path: String,
    }

    #[derive(Debug)]
    struct MatrixCell {
        id: &'static str,
        prompt: &'static str,
        expected: String,
        status: CellStatus,
        duration: Duration,
        opencode_version: String,
        aiplus_version: String,
        fixture_path: Option<PathBuf>,
    }

    #[derive(Debug)]
    enum CellStatus {
        Pass,
        Skip(&'static str, String),
        Fail(String),
    }

    const POSITIVE_CASES: &[PositiveCase] = &[
        PositiveCase {
            id: "positive_you_are_ceo_en",
            prompt: "you are CEO",
            expected_role: "ceo",
            requested_role: "ceo",
            expected_policy: ExpectedPolicy::Coordinator,
        },
        PositiveCase {
            id: "positive_you_are_ceo_zh",
            prompt: "你是 CEO",
            expected_role: "ceo",
            requested_role: "ceo",
            expected_policy: ExpectedPolicy::Coordinator,
        },
        PositiveCase {
            id: "positive_you_are_qa_en",
            prompt: "you are qa",
            expected_role: "qa",
            requested_role: "qa",
            expected_policy: ExpectedPolicy::Builder,
        },
        PositiveCase {
            id: "positive_you_are_qa_zh",
            prompt: "你是 qa",
            expected_role: "qa",
            requested_role: "qa",
            expected_policy: ExpectedPolicy::Builder,
        },
        PositiveCase {
            id: "positive_open_advisor",
            prompt: "开 advisor",
            expected_role: "advisor",
            requested_role: "advisor",
            expected_policy: ExpectedPolicy::Coordinator,
        },
        PositiveCase {
            id: "positive_do_engineer_b",
            prompt: "做 engineer-b",
            expected_role: "engineer-b",
            requested_role: "engineer-b",
            expected_policy: ExpectedPolicy::Builder,
        },
        PositiveCase {
            id: "positive_take_reviewer",
            prompt: "take reviewer",
            expected_role: "reviewer",
            requested_role: "reviewer",
            expected_policy: ExpectedPolicy::Reviewer,
        },
        PositiveCase {
            id: "positive_take_reviewer_role",
            prompt: "take the reviewer role",
            expected_role: "reviewer",
            requested_role: "reviewer",
            expected_policy: ExpectedPolicy::Reviewer,
        },
        PositiveCase {
            id: "positive_switch_architect_fresh",
            prompt: "switch to architect",
            expected_role: "architect",
            requested_role: "architect",
            expected_policy: ExpectedPolicy::Builder,
        },
        PositiveCase {
            id: "positive_ceo_perspective",
            prompt: "以 CEO 的视角看一下",
            expected_role: "ceo",
            requested_role: "ceo",
            expected_policy: ExpectedPolicy::Coordinator,
        },
    ];

    const NEGATIVE_CASES: &[NegativeCase] = &[
        NegativeCase {
            id: "negative_ceo_question_zh",
            prompt: "你是 CEO 吗？",
        },
        NegativeCase {
            id: "negative_markdown_blockquote_ceo",
            prompt: "> you are CEO",
        },
        NegativeCase {
            id: "negative_inline_code_ceo",
            prompt: "`you are CEO`",
        },
        NegativeCase {
            id: "negative_quoted_pi_example",
            prompt: "I wrote \"you are PI\" in the prompt",
        },
        NegativeCase {
            id: "negative_show_phrase_reviewer",
            prompt: "show me the phrase: take the reviewer role",
        },
        NegativeCase {
            id: "negative_compare_ceo_advisor",
            prompt: "compare CEO and advisor",
        },
        NegativeCase {
            id: "negative_do_not_switch_ceo",
            prompt: "不要切到 CEO",
        },
    ];

    fn aiplus_bin() -> PathBuf {
        PathBuf::from(env!("CARGO_BIN_EXE_aiplus"))
    }

    fn path_with_source_aiplus() -> String {
        let bin_dir = aiplus_bin().parent().unwrap().to_path_buf();
        let inherited = env::var("PATH").unwrap_or_default();
        format!("{}:{inherited}", bin_dir.display())
    }

    fn fake_env(root: &Path) -> FakeEnv {
        let home = root.join("home");
        let xdg = root.join("xdg");
        let codex = root.join("codex");
        fs::create_dir_all(&home).unwrap();
        fs::create_dir_all(&xdg).unwrap();
        fs::create_dir_all(&codex).unwrap();

        FakeEnv {
            home,
            xdg,
            codex,
            path: path_with_source_aiplus(),
        }
    }

    fn run_aiplus(cwd: &Path, envs: &FakeEnv, args: &[&str], expected: i32) -> Output {
        let output = Command::new(aiplus_bin())
            .args(args)
            .current_dir(cwd)
            .env("HOME", &envs.home)
            .env("XDG_CONFIG_HOME", &envs.xdg)
            .env("CODEX_HOME", &envs.codex)
            .env("PATH", &envs.path)
            .env("AIPLUS_SKIP_VERSION_CHECK", "1")
            .output()
            .expect("run source-built aiplus");
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

    fn source_aiplus_version(cwd: &Path, envs: &FakeEnv) -> String {
        stdout(&run_aiplus(cwd, envs, &["--version"], 0))
            .trim()
            .to_string()
    }

    fn opencode_version(path: &str) -> Result<String, String> {
        let output = Command::new("opencode")
            .arg("--version")
            .env("PATH", path)
            .output()
            .map_err(|err| format!("SKIP_MISSING_BINARY opencode --version spawn failed: {err}"))?;
        if output.status.success() {
            Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
        } else {
            Err(format!(
                "FAIL_VERSION_PROBE opencode --version exited {:?}: {}{}",
                output.status.code(),
                String::from_utf8_lossy(&output.stdout),
                String::from_utf8_lossy(&output.stderr)
            ))
        }
    }

    fn setup_project(root: &Path, case_id: &str, role: Option<&str>) -> (PathBuf, FakeEnv, String) {
        let envs = fake_env(root);
        let project = root.join("project");
        fs::create_dir_all(&project).unwrap();

        run_aiplus(&project, &envs, &["install", "opencode", "--yes"], 0);
        run_aiplus(
            &project,
            &envs,
            &[
                "memory",
                "--scope",
                "team",
                "add",
                "--kind",
                "handoff_note",
                "--text",
                &format!("G1.1 OpenCode TUI team memory fixture for {case_id}"),
            ],
            0,
        );
        run_aiplus(
            &project,
            &envs,
            &[
                "memory",
                "--scope",
                "project",
                "add",
                "--kind",
                "project_fact",
                "--text",
                &format!("G1.1 OpenCode TUI project memory fixture for {case_id}"),
            ],
            0,
        );
        if let Some(role) = role {
            run_aiplus(
                &project,
                &envs,
                &[
                    "memory",
                    "--scope",
                    "personal",
                    "--role",
                    role,
                    "add",
                    "--kind",
                    "preference",
                    "--text",
                    &format!("G1.1 OpenCode TUI role-personal fixture for {role}"),
                ],
                0,
            );
        }

        assert_opencode_adapter_contract(&project, &envs);
        let version = source_aiplus_version(&project, &envs);
        (project, envs, version)
    }

    fn assert_opencode_adapter_contract(project: &Path, envs: &FakeEnv) {
        let config_path = project.join(".opencode/opencode.json");
        let instructions_path = project.join(".opencode/instructions/aiplus.md");
        let config: Value = serde_json::from_str(&fs::read_to_string(&config_path).unwrap())
            .expect(".opencode/opencode.json parses as JSON");
        let entries = config
            .get("instructions")
            .and_then(Value::as_array)
            .expect(".opencode/opencode.json instructions array");
        assert!(
            entries
                .iter()
                .any(|entry| entry.as_str() == Some("instructions/aiplus.md")),
            ".opencode/opencode.json missing instructions/aiplus.md"
        );
        let instructions = fs::read_to_string(&instructions_path).unwrap();
        assert!(
            instructions.contains("AIPLUS_OPENCODE_G1_ROLE_TRIGGERS_V1"),
            ".opencode/instructions/aiplus.md missing G1 marker"
        );
        let doctor = stdout(&run_aiplus(project, envs, &["doctor"], 0));
        assert!(
            doctor.contains("nl_role_triggers=PASS"),
            "doctor should pass nl_role_triggers before TUI spawn:\n{doctor}"
        );
    }

    fn spawn_opencode(
        project: &Path,
        envs: &FakeEnv,
    ) -> Result<rexpect::session::PtySession, String> {
        let mut command = Command::new("opencode");
        command
            .current_dir(project)
            .env("HOME", &envs.home)
            .env("XDG_CONFIG_HOME", &envs.xdg)
            .env("CODEX_HOME", &envs.codex)
            .env("PATH", &envs.path)
            .env("AIPLUS_SKIP_VERSION_CHECK", "1")
            .env("NO_COLOR", "1")
            .env("TERM", "xterm-256color")
            .env("COLUMNS", "120")
            .env("LINES", "40");
        spawn_command(command, Some(5_000)).map_err(|err| format!("FAIL_TUI_STARTUP {err}"))
    }

    fn default_ready_markers() -> Vec<String> {
        ready_markers_from_env(env::var(READY_ENV).ok())
    }

    fn ready_markers_from_env(raw: Option<String>) -> Vec<String> {
        raw.map(|raw| {
            raw.split('|')
                .map(str::trim)
                .filter(|part| !part.is_empty())
                .map(ToOwned::to_owned)
                .collect()
        })
        .filter(|markers: &Vec<String>| !markers.is_empty())
        .unwrap_or_else(|| vec!["ctrl+p commands".to_string(), "Kimi For Coding".to_string()])
    }

    #[test]
    fn default_ready_markers_prioritize_stable_opencode_footer_marker() {
        assert_eq!(
            ready_markers_from_env(None),
            vec!["ctrl+p commands".to_string(), "Kimi For Coding".to_string()]
        );
    }

    #[test]
    fn ready_markers_env_override_trims_and_skips_empty_entries() {
        assert_eq!(
            ready_markers_from_env(Some(" custom one | | custom two ".to_string())),
            vec!["custom one".to_string(), "custom two".to_string()]
        );
    }

    #[test]
    fn empty_ready_markers_env_override_falls_back_to_defaults() {
        assert_eq!(
            ready_markers_from_env(Some(" |  | ".to_string())),
            vec!["ctrl+p commands".to_string(), "Kimi For Coding".to_string()]
        );
    }

    fn wait_for_ready(
        session: &mut rexpect::session::PtySession,
        raw: &mut String,
    ) -> Result<(), CellStatus> {
        let deadline = Instant::now() + Duration::from_secs(30);
        let ready_markers = default_ready_markers();
        while Instant::now() < deadline {
            drain_available(session, raw, Duration::from_millis(100));
            let normalized = normalize_terminal(raw);
            if auth_or_model_unavailable(&normalized) {
                return Err(CellStatus::Skip(
                    "SKIP_AUTH_MODEL",
                    first_matching_line(
                        &normalized,
                        &["auth", "login", "quota", "model", "provider"],
                    )
                    .unwrap_or_else(|| "auth/model unavailable".to_string()),
                ));
            }
            if ready_markers
                .iter()
                .any(|marker| normalized.contains(marker.as_str()))
            {
                return Ok(());
            }
            if let Some(status) = session.process().status() {
                if status != WaitStatus::StillAlive {
                    return Err(CellStatus::Fail(format!(
                        "FAIL_TUI_STARTUP process exited before ready: {status:?}\n{}",
                        tail(&normalized, 2000)
                    )));
                }
            }
            thread::sleep(Duration::from_millis(100));
        }
        Err(CellStatus::Skip(
            "SKIP_UNSUPPORTED_TUI",
            format!(
                "ready marker not found within 30s; set {READY_ENV} to configure markers\n{}",
                tail(&normalize_terminal(raw), 2000)
            ),
        ))
    }

    fn submit_prompt(
        session: &mut rexpect::session::PtySession,
        prompt: &str,
    ) -> Result<(), String> {
        if env::var("AIPLUS_OPENCODE_TUI_BRACKETED_PASTE").as_deref() == Ok("1") {
            session
                .send(&format!("\x1b[200~{prompt}\x1b[201~"))
                .map_err(|err| err.to_string())?;
            session.flush().map_err(|err| err.to_string())?;
            session.send_line("").map_err(|err| err.to_string())?;
        } else {
            session.send_line(prompt).map_err(|err| err.to_string())?;
        }
        Ok(())
    }

    fn wait_for_activation(
        session: &mut rexpect::session::PtySession,
        raw: &mut String,
        case: PositiveCase,
    ) -> Result<String, CellStatus> {
        let deadline = Instant::now() + Duration::from_secs(90);
        while Instant::now() < deadline {
            drain_available(session, raw, Duration::from_millis(150));
            let normalized = normalize_terminal(raw);
            if auth_or_model_unavailable(&normalized) {
                return Err(CellStatus::Skip(
                    "SKIP_AUTH_MODEL",
                    first_matching_line(
                        &normalized,
                        &["auth", "login", "quota", "model", "provider"],
                    )
                    .unwrap_or_else(|| "auth/model unavailable".to_string()),
                ));
            }
            for line in normalized.lines() {
                if line.starts_with("ROLE_ACTIVATED ") {
                    if activation_line_matches(line, case) {
                        return Ok(line.to_string());
                    }
                    return Err(CellStatus::Fail(format!(
                        "activation schema mismatch for {}\nline: {line}\nexpected role={} requested_role={} policy={}",
                        case.id,
                        case.expected_role,
                        case.requested_role,
                        case.expected_policy.as_str()
                    )));
                }
                if line.contains("runtime=codex") || line.contains("runtime=claude-code") {
                    return Err(CellStatus::Fail(format!(
                        "wrong runtime leaked in OpenCode transcript line: {line}"
                    )));
                }
            }
            thread::sleep(Duration::from_millis(150));
        }
        Err(CellStatus::Fail(format!(
            "timeout waiting for ROLE_ACTIVATED for {}\n{}",
            case.id,
            tail(&normalize_terminal(raw), 4000)
        )))
    }

    fn wait_for_no_trigger(
        session: &mut rexpect::session::PtySession,
        raw: &mut String,
        case: NegativeCase,
    ) -> CellStatus {
        let deadline = Instant::now() + Duration::from_secs(20);
        while Instant::now() < deadline {
            drain_available(session, raw, Duration::from_millis(150));
            let normalized = normalize_terminal(raw);
            if auth_or_model_unavailable(&normalized) {
                return CellStatus::Skip(
                    "SKIP_AUTH_MODEL",
                    first_matching_line(
                        &normalized,
                        &["auth", "login", "quota", "model", "provider"],
                    )
                    .unwrap_or_else(|| "auth/model unavailable".to_string()),
                );
            }
            if let Some(token) = ROLE_FORBID_TOKENS
                .iter()
                .find(|token| normalized.contains(**token))
            {
                return CellStatus::Fail(format!(
                    "negative case {} leaked forbidden token {token}\n{}",
                    case.id,
                    tail(&normalized, 4000)
                ));
            }
            thread::sleep(Duration::from_millis(150));
        }
        CellStatus::Pass
    }

    fn activation_line_matches(line: &str, case: PositiveCase) -> bool {
        let fields: Vec<&str> = line.split_whitespace().collect();
        fields.first() == Some(&"ROLE_ACTIVATED")
            && has_field(&fields, "role", case.expected_role)
            && has_numeric_field(&fields, "count", false)
            && has_field(&fields, "schema", "v1")
            && has_field(&fields, "runtime", "opencode")
            && has_field(&fields, "trigger", "nl_role_bind")
            && has_field(&fields, "requested_role", case.requested_role)
            && has_numeric_field(&fields, "memory_personal", false)
            && has_numeric_field(&fields, "memory_team", true)
            && memory_project_matches(&fields, case.expected_policy)
            && has_field(&fields, "memory_policy", case.expected_policy.as_str())
            && has_field(&fields, "identity_context", "PASS")
            && has_field(&fields, "memory_loaded", "yes")
            && has_field(&fields, "permissions", "none")
            && has_field(&fields, "identity_grants_permission", "no")
            && has_field(&fields, "secret_values", "none")
            && has_field(&fields, "global_agent_config_edits", "none")
            && !line.contains("runtime=codex")
            && !line.contains("runtime=claude-code")
    }

    fn has_field(fields: &[&str], key: &str, expected: &str) -> bool {
        fields
            .iter()
            .any(|field| *field == format!("{key}={expected}"))
    }

    fn has_numeric_field(fields: &[&str], key: &str, nonzero: bool) -> bool {
        fields.iter().any(|field| {
            field
                .strip_prefix(&format!("{key}="))
                .and_then(|value| value.parse::<u64>().ok())
                .is_some_and(|value| !nonzero || value > 0)
        })
    }

    fn memory_project_matches(fields: &[&str], policy: ExpectedPolicy) -> bool {
        match policy {
            ExpectedPolicy::Coordinator => has_numeric_field(fields, "memory_project", true),
            ExpectedPolicy::Builder | ExpectedPolicy::Reviewer => {
                has_field(fields, "memory_project", "null")
            }
        }
    }

    fn drain_available(
        session: &mut rexpect::session::PtySession,
        raw: &mut String,
        duration: Duration,
    ) {
        let until = Instant::now() + duration;
        while Instant::now() < until {
            let mut read_any = false;
            while let Some(ch) = session.try_read() {
                raw.push(ch);
                read_any = true;
            }
            if !read_any {
                thread::sleep(Duration::from_millis(25));
            }
        }
    }

    fn normalize_terminal(raw: &str) -> String {
        let mut out = String::with_capacity(raw.len());
        let bytes = raw.as_bytes();
        let mut i = 0;
        while i < bytes.len() {
            match bytes[i] {
                0x1b => {
                    i += 1;
                    if i < bytes.len() && bytes[i] == b']' {
                        i += 1;
                        while i < bytes.len() && bytes[i] != 0x07 {
                            if bytes[i] == 0x1b && i + 1 < bytes.len() && bytes[i + 1] == b'\\' {
                                i += 2;
                                break;
                            }
                            i += 1;
                        }
                        if i < bytes.len() && bytes[i] == 0x07 {
                            i += 1;
                        }
                    } else {
                        while i < bytes.len() && !(0x40..=0x7e).contains(&bytes[i]) {
                            i += 1;
                        }
                        if i < bytes.len() {
                            i += 1;
                        }
                    }
                }
                b'\r' => {
                    out.push('\n');
                    i += 1;
                }
                byte if byte < 0x20 && byte != b'\n' && byte != b'\t' => {
                    i += 1;
                }
                _ => {
                    let rest = &raw[i..];
                    if let Some(ch) = rest.chars().next() {
                        out.push(ch);
                        i += ch.len_utf8();
                    } else {
                        break;
                    }
                }
            }
        }
        out.lines()
            .map(str::trim_end)
            .filter(|line| !line.is_empty())
            .collect::<Vec<_>>()
            .join("\n")
    }

    fn auth_or_model_unavailable(normalized: &str) -> bool {
        let lower = normalized.to_lowercase();
        [
            "not logged in",
            "login",
            "authentication",
            "unauthorized",
            "provider",
            "model not found",
            "quota",
            "rate limit",
            "api key",
        ]
        .iter()
        .any(|needle| lower.contains(needle))
    }

    fn first_matching_line(normalized: &str, needles: &[&str]) -> Option<String> {
        let lowered_needles = needles
            .iter()
            .map(|needle| needle.to_lowercase())
            .collect::<Vec<_>>();
        normalized.lines().find_map(|line| {
            let lower = line.to_lowercase();
            lowered_needles
                .iter()
                .any(|needle| lower.contains(needle))
                .then(|| line.to_string())
        })
    }

    fn tail(text: &str, max_chars: usize) -> String {
        let chars = text.chars().collect::<Vec<_>>();
        let start = chars.len().saturating_sub(max_chars);
        chars[start..].iter().collect()
    }

    fn cleanup_session(session: &mut rexpect::session::PtySession) {
        let _ = session.send_control('c');
        thread::sleep(Duration::from_millis(250));
        if let Some(WaitStatus::StillAlive) = session.process().status() {
            let _ = session.process_mut().signal(Signal::SIGTERM);
            thread::sleep(Duration::from_secs(1));
        }
        if let Some(WaitStatus::StillAlive) = session.process().status() {
            let _ = session.process_mut().signal(Signal::SIGKILL);
        }
    }

    fn persist_artifacts(root: &Path, raw: &str, normalized: &str) -> Option<PathBuf> {
        if env::var(KEEP_ENV).as_deref() != Ok("1") {
            return None;
        }
        let artifact_dir = root.join("artifacts");
        fs::create_dir_all(&artifact_dir).unwrap();
        fs::write(artifact_dir.join("raw-transcript.txt"), raw).unwrap();
        fs::write(artifact_dir.join("normalized-transcript.txt"), normalized).unwrap();
        Some(root.to_path_buf())
    }

    fn run_positive(case: PositiveCase, opencode_version: &str) -> MatrixCell {
        let started = Instant::now();
        let temp = tempfile::Builder::new()
            .prefix(&format!("aiplus-opencode-tui-{}-", case.id))
            .tempdir()
            .unwrap();
        let root = temp.path().to_path_buf();
        let (project, envs, aiplus_version) =
            setup_project(&root, case.id, Some(case.expected_role));
        let mut raw = String::new();
        let status = match spawn_opencode(&project, &envs) {
            Ok(mut session) => {
                let status = match wait_for_ready(&mut session, &mut raw) {
                    Ok(()) => match submit_prompt(&mut session, case.prompt) {
                        Ok(()) => match wait_for_activation(&mut session, &mut raw, case) {
                            Ok(_) => CellStatus::Pass,
                            Err(status) => status,
                        },
                        Err(err) => CellStatus::Fail(format!("prompt submit failed: {err}")),
                    },
                    Err(status) => status,
                };
                cleanup_session(&mut session);
                status
            }
            Err(err) => CellStatus::Fail(err),
        };
        let normalized = normalize_terminal(&raw);
        let fixture_path = persist_artifacts(&root, &raw, &normalized);
        if fixture_path.is_some() {
            std::mem::forget(temp);
        }
        MatrixCell {
            id: case.id,
            prompt: case.prompt,
            expected: format!(
                "role={} policy={}",
                case.expected_role,
                case.expected_policy.as_str()
            ),
            status,
            duration: started.elapsed(),
            opencode_version: opencode_version.to_string(),
            aiplus_version,
            fixture_path,
        }
    }

    fn run_negative(case: NegativeCase, opencode_version: &str) -> MatrixCell {
        let started = Instant::now();
        let temp = tempfile::Builder::new()
            .prefix(&format!("aiplus-opencode-tui-{}-", case.id))
            .tempdir()
            .unwrap();
        let root = temp.path().to_path_buf();
        let (project, envs, aiplus_version) = setup_project(&root, case.id, None);
        let mut raw = String::new();
        let status = match spawn_opencode(&project, &envs) {
            Ok(mut session) => {
                let status = match wait_for_ready(&mut session, &mut raw) {
                    Ok(()) => match submit_prompt(&mut session, case.prompt) {
                        Ok(()) => wait_for_no_trigger(&mut session, &mut raw, case),
                        Err(err) => CellStatus::Fail(format!("prompt submit failed: {err}")),
                    },
                    Err(status) => status,
                };
                cleanup_session(&mut session);
                status
            }
            Err(err) => CellStatus::Fail(err),
        };
        let normalized = normalize_terminal(&raw);
        let fixture_path = persist_artifacts(&root, &raw, &normalized);
        if fixture_path.is_some() {
            std::mem::forget(temp);
        }
        MatrixCell {
            id: case.id,
            prompt: case.prompt,
            expected: "no_trigger no ROLE_ leakage".to_string(),
            status,
            duration: started.elapsed(),
            opencode_version: opencode_version.to_string(),
            aiplus_version,
            fixture_path,
        }
    }

    fn print_matrix(cells: &[MatrixCell]) {
        eprintln!("G1.1 OpenCode interactive/TUI role-trigger matrix");
        eprintln!("case_id\tstatus\tduration_ms\texpected\topencode\taiplus\tfixture\tprompt");
        for cell in cells {
            let status = match &cell.status {
                CellStatus::Pass => "PASS".to_string(),
                CellStatus::Skip(kind, reason) => format!("{kind}: {reason}"),
                CellStatus::Fail(reason) => format!("FAIL: {reason}"),
            };
            let fixture = cell
                .fixture_path
                .as_ref()
                .map(|path| path.display().to_string())
                .unwrap_or_else(|| "-".to_string());
            eprintln!(
                "{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}",
                cell.id,
                status.replace('\n', " "),
                cell.duration.as_millis(),
                cell.expected,
                cell.opencode_version,
                cell.aiplus_version,
                fixture,
                cell.prompt.replace('\n', "\\n")
            );
        }
    }

    #[test]
    #[ignore = "live OpenCode TUI harness; run with AIPLUS_OPENCODE_TUI_TESTS=1 cargo test -p aiplus-cli --test opencode_tui_role_trigger -- --ignored"]
    fn opencode_interactive_tui_role_trigger_matrix() {
        assert_eq!(POSITIVE_CASES.len(), 10);
        assert!(NEGATIVE_CASES.len() >= 5);
        assert!(NEGATIVE_CASES
            .iter()
            .any(|case| case.prompt == "> you are CEO"));

        if env::var(ENABLE_ENV).as_deref() != Ok("1") {
            eprintln!(
                "SKIP_ENV_GATED set {ENABLE_ENV}=1 to run live OpenCode interactive/TUI cases"
            );
            return;
        }

        let path = path_with_source_aiplus();
        let opencode_version = match opencode_version(&path) {
            Ok(version) => version,
            Err(reason) if reason.starts_with("SKIP_MISSING_BINARY") => {
                eprintln!("{reason}");
                return;
            }
            Err(reason) => panic!("{reason}"),
        };

        let mut cells = Vec::new();
        for case in POSITIVE_CASES {
            cells.push(run_positive(*case, &opencode_version));
        }
        for case in NEGATIVE_CASES {
            cells.push(run_negative(*case, &opencode_version));
        }
        print_matrix(&cells);

        let failures = cells
            .iter()
            .filter_map(|cell| match &cell.status {
                CellStatus::Fail(reason) => Some(format!("{}: {reason}", cell.id)),
                CellStatus::Pass | CellStatus::Skip(_, _) => None,
            })
            .collect::<Vec<_>>();
        if !failures.is_empty() {
            panic!("OpenCode TUI harness failures:\n{}", failures.join("\n"));
        }
    }
}

#[cfg(not(unix))]
#[test]
#[ignore = "OpenCode TUI harness uses rexpect PTY support on Unix hosts"]
fn opencode_interactive_tui_role_trigger_matrix() {
    eprintln!("SKIP_UNSUPPORTED_TUI rexpect PTY harness is Unix-only");
}
