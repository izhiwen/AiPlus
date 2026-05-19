use crate::agent::coordinator;
use crate::agent::core::load_team_config;
use anyhow::Result;
use std::process::Command;

pub fn handle_doctor(quiet: bool) -> Result<()> {
    let project_root = std::env::current_dir()?;
    let agents_dir = project_root.join(".aiplus").join("agents");
    let mut out = DoctorOutput::new(quiet);

    out.info("Running agent team doctor...");
    out.info("Checking .aiplus/agents/ directory...");
    match crate::agent::audit::verify_log::verify_dispatch_log(&project_root) {
        Ok(report) => out.info(format!(
            "  INFO dispatch_log_chain={}",
            report.doctor_status()
        )),
        Err(e) => out.info(format!("  INFO dispatch_log_chain=unavailable reason={e}")),
    }
    out.info(format!("  INFO commit_signing={}", detect_commit_signing()));

    if !agents_dir.exists() {
        out.warn("  WARNING: .aiplus/agents/ does not exist");
        out.finish();
        return Ok(());
    }

    let state = load_team_config(&project_root)?;
    out.info(format!("  Found {} agent config(s)", state.agents.len()));
    match crate::agent::cache::disk_cache_status(&project_root) {
        Ok(status) => {
            out.info(format!(
                "  Disk cache: {} enforce_ttl={} ({})",
                if status.enabled {
                    "enabled"
                } else {
                    "disabled"
                },
                status.enforce_ttl,
                status.project_dir.display()
            ));
            if let Some(meta) = &status.meta {
                let now = crate::agent::cache::current_epoch_millis();
                for (role, entry) in &meta.roles {
                    let age_ms = now.saturating_sub(entry.last_used_at_ms);
                    out.info(format!(
                        "    INFO cache_age role={} age_seconds={} ttl_seconds={}",
                        role,
                        age_ms / 1000,
                        entry.ttl_seconds
                    ));
                    if status.enforce_ttl && age_ms > entry.ttl_seconds as u128 * 1000 {
                        out.warn(format!(
                            "    WARN cache_ttl_expired role={} age_seconds={} ttl_seconds={}",
                            role,
                            age_ms / 1000,
                            entry.ttl_seconds
                        ));
                    }
                }
            }
            if let Some(warning) = status.sync_warning {
                out.warn(format!("    WARNING: {warning}"));
            }
        }
        Err(e) => out.warn(format!("  WARNING: disk cache status unavailable: {e}")),
    }
    for warning in crate::agent::cache::cache_warnings(&project_root) {
        out.warn(format!("  WARNING: disk cache {warning}"));
    }
    if should_warn_secret_broker_runtime_auth(&state) {
        out.warn("  WARN_SECRET_BROKER_RUNTIME_AUTH");
        out.warn(
            "  WARN Detected BWS backend + active agent-team roles, but no provider key in env.",
        );
        out.warn("  WARN Adapter dispatch will fail with auth error.");
        out.warn("  WARN Fix: wrap your dispatch with");
        out.warn(
            "    aiplus secret-broker run --aliases anthropic,openai -- aiplus agent route \"<task>\""
        );
        out.warn("  WARN Or set ANTHROPIC_API_KEY / OPENAI_API_KEY in your shell once.");
    }

    for (role, config) in &state.agents {
        if out.quiet {
            continue;
        }
        print!("  {} ({})", role, config.display_name);
        if config.stub {
            print!(" [STUB]");
        }
        if state.active_roles.contains(role) {
            print!(" [ACTIVE]");
        } else if state.disabled_roles.contains(role) {
            print!(" [DISABLED]");
        }
        println!();

        if let Some(path) = state.worktree_paths.get(role) {
            if !path.exists() && config.needs_worktree {
                // Lazy worktree creation is by design — worktrees are
                // only provisioned when the PI/CEO dispatches a task
                // to the role. Demote WARNING → INFO so the output is
                // not noisy by default. Users who *have* dispatched
                // and find the worktree gone will still see this line,
                // but it won't drown out real issues.
                println!(
                    "    INFO: worktree {} not yet provisioned (lazy)",
                    path.display()
                );
            }
        }
    }

    if coordinator::thresholds_match_design() {
        out.pass("  PASS coordinator scoring config valid");
        out.pass("  PASS coordinator tier thresholds match DESIGN.md §9.2");
    } else {
        out.warn("  WARNING: coordinator tier thresholds drift from DESIGN.md §9.2");
    }

    out.info("Doctor check complete.");
    out.finish();
    Ok(())
}

struct DoctorOutput {
    quiet: bool,
    warning_seen: bool,
}

impl DoctorOutput {
    fn new(quiet: bool) -> Self {
        Self {
            quiet,
            warning_seen: false,
        }
    }

    fn info(&self, line: impl AsRef<str>) {
        if !self.quiet {
            println!("{}", line.as_ref());
        }
    }

    fn pass(&self, line: impl AsRef<str>) {
        if !self.quiet {
            println!("{}", line.as_ref());
        }
    }

    fn warn(&mut self, line: impl AsRef<str>) {
        self.warning_seen = true;
        println!("{}", line.as_ref());
    }

    fn finish(&self) {
        println!(
            "DOCTOR_STATUS={}",
            if self.warning_seen { "WARN" } else { "PASS" }
        );
    }
}

fn should_warn_secret_broker_runtime_auth(state: &crate::agent::core::TeamState) -> bool {
    let provider_is_bws = std::env::var("AIPLUS_SECRET_PROVIDER")
        .map(|provider| provider.trim().eq_ignore_ascii_case("bws"))
        .unwrap_or(false);
    let active_roles_present = !state.active_roles.is_empty();
    let provider_key_present = env_present("ANTHROPIC_API_KEY") || env_present("OPENAI_API_KEY");

    provider_is_bws && active_roles_present && !provider_key_present
}

fn env_present(name: &str) -> bool {
    std::env::var(name)
        .map(|value| !value.trim().is_empty())
        .unwrap_or(false)
}

fn detect_commit_signing() -> &'static str {
    let format = git_config_get("gpg.format");
    let signing_key = git_config_get("user.signingkey");
    if format
        .as_deref()
        .map(|value| value.eq_ignore_ascii_case("ssh"))
        .unwrap_or(false)
    {
        if signing_key
            .as_deref()
            .map(|value| value.contains("id_ecdsa_sk_aiplus"))
            .unwrap_or(false)
        {
            return "secure_enclave";
        }
        return "ssh";
    }
    if format.is_some() || git_config_get("commit.gpgsign").as_deref() == Some("true") {
        return "gpg";
    }
    "none"
}

fn git_config_get(key: &str) -> Option<String> {
    let output = Command::new("git")
        .args(["config", "--global", "--get", key])
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    let value = String::from_utf8_lossy(&output.stdout).trim().to_string();
    if value.is_empty() {
        None
    } else {
        Some(value)
    }
}
