use crate::agent::cache;
use crate::agent::core::get_role_config;
use crate::agent::state;
use crate::agent::worktree;
use aiplus_core::consult;
use anyhow::Result;

pub fn handle_route(role: Option<&str>, task: &str) -> Result<()> {
    // Heuristic for "is this arg a role name?":
    //   - If the user passed only one positional and no task, the arg might
    //     either be a role (`aiplus agent route advisor`) or a single-word
    //     free-form task (`aiplus agent route "estimate IV"`).
    //   - If we can resolve it as a role via get_role_config, treat it as
    //     a direct role dispatch.
    //   - Otherwise, treat the role token as part of the free-form task
    //     description and route it through the PI/CEO for scoring.
    if let Some(candidate) = role {
        match get_role_config(candidate) {
            Ok(config) => {
                println!("Routing task to {}: {}", candidate, task);
                if let Ok(cache) = cache::global_cache().lock() {
                    cache.invalidate(candidate, cache::InvalidationReason::RoleRouteCalled);
                }
                let project_root = std::env::current_dir()?;
                if config.needs_worktree {
                    // Worktree provisioning requires a git repo. If the project
                    // isn't one, surface a clear note but still record the
                    // dispatch so the audit log entry isn't lost.
                    match worktree::worktree_exists_for_role(&project_root, candidate) {
                        Ok(Some(path)) => {
                            println!("  Using existing worktree: {}", path.display());
                        }
                        Ok(None) => {
                            println!("  Creating worktree for {}...", candidate);
                            let template = config.worktree_path.as_deref();
                            match worktree::create_worktree(&project_root, candidate, template) {
                                Ok(path) => {
                                    println!("  Worktree created: {}", path.display());
                                }
                                Err(e) => {
                                    eprintln!("  ERROR: Failed to create worktree: {}", e);
                                    return Err(e);
                                }
                            }
                        }
                        Err(err) => {
                            // Likely "not a git repository" — non-fatal. Warn
                            // and continue so we still record the dispatch.
                            eprintln!(
                                "  NOTE: skipping worktree provisioning ({err}); \
                                 init this project with `git init` to enable per-role worktrees."
                            );
                        }
                    }
                }
                // Persist the dispatch so this becomes a real side effect, not
                // just narrative. Phase D v0: writes audit log + marks role
                // active. v1: mirrors to project memory and surfaces a
                // consultant nudge for medium/heavy tasks.
                if let Err(e) =
                    state::record_dispatch(&project_root, candidate, task, "aiplus agent route")
                {
                    eprintln!("  WARN: failed to record dispatch: {e}");
                } else {
                    println!("  Dispatch recorded: .aiplus/agents/dispatch-log.jsonl");
                }
                if !task.is_empty() {
                    run_consult(&project_root, candidate, task)?;
                }
                return Ok(());
            }
            Err(_) => {
                // Not a known role — rebuild the full free-form task and
                // route to PI/CEO for scoring.
                let full_task = if task.is_empty() {
                    candidate.to_string()
                } else {
                    format!("{candidate} {task}")
                };
                println!("Routing task to PI/CEO for scoring and dispatch: {full_task}");
                let project_root = std::env::current_dir()?;
                run_consult(&project_root, "pi", &full_task)?;
                return Ok(());
            }
        }
    }
    println!("Routing task to PI/CEO for scoring and dispatch: {task}");
    let project_root = std::env::current_dir()?;
    run_consult(&project_root, "pi", task)?;
    Ok(())
}

/// Walk the consultant team for `task` and persist per-member findings
/// under `.aiplus/agent-memory/_team/consult-<task-id>.jsonl`. The
/// JSONL is what makes the consult a real side effect instead of a
/// narrative — downstream `agent transcript` reads it, and W2 (owner
/// gates) will gate dispatch on what it finds.
///
/// Failure to consult is intentionally non-fatal: missing config or
/// unsupported schema prints a NOTE and lets dispatch continue. The
/// goal is "consult-as-side-effect when possible," not "force every
/// route through a complete consult."
fn run_consult(project_root: &std::path::Path, role: &str, task: &str) -> Result<()> {
    let team = match consult::load_consult_team(project_root) {
        Ok(Some(team)) => team,
        Ok(None) => {
            return Ok(());
        }
        Err(e) => {
            eprintln!(
                "  NOTE: consultant team config could not be loaded ({e}); skipping consult."
            );
            return Ok(());
        }
    };
    if !consult::is_supported_schema(&team.schema_version) {
        eprintln!(
            "  NOTE: consultant-team.toml schema_version='{}' not in the supported list \
             ({:?}); skipping consult. Run `aiplus doctor` for guidance.",
            team.schema_version,
            consult::SUPPORTED_CONSULT_SCHEMAS,
        );
        return Ok(());
    }
    let today = aiplus_core::now_iso();
    // YYYY-MM-DD slice as the task-id salt so re-running the same
    // command on the same day yields the same id (idempotent), while
    // re-running tomorrow opens a fresh consult file.
    let date_salt: String = today.chars().take(10).collect();
    let task_id = consult::derive_task_id(role, task, &date_salt);
    let (tier, complexity, risk, findings) = consult::build_findings(&team, task, &task_id, &today);

    if findings.is_empty() {
        println!(
            "  Consult tier: {} (complexity {}, risk {:.2}). No member triggers matched — skipping artifact.",
            tier.as_str(),
            complexity,
            risk,
        );
        return Ok(());
    }
    match consult::write_findings(project_root, &task_id, &findings) {
        Ok(path) => {
            // Convert to a project-relative path for the on-screen
            // hint; absolute paths are noisy and break copy/paste.
            let rel = path
                .strip_prefix(project_root)
                .map(|p| p.to_path_buf())
                .unwrap_or(path.clone());
            println!(
                "  Consult tier: {} (complexity {}, risk {:.2}). {} finding(s) recorded: {}",
                tier.as_str(),
                complexity,
                risk,
                findings.len(),
                rel.display(),
            );
        }
        Err(e) => {
            eprintln!("  WARN: failed to write consult findings: {e}");
        }
    }
    Ok(())
}
