use crate::agent::cache;
use crate::agent::core::get_role_config;
use crate::agent::state;
use crate::agent::worktree;
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
                            match worktree::create_worktree(
                                &project_root,
                                candidate,
                                template,
                            ) {
                                Ok(path) => {
                                    println!("  Worktree created: {}", path.display());
                                }
                                Err(e) => {
                                    eprintln!(
                                        "  ERROR: Failed to create worktree: {}",
                                        e
                                    );
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
                    let (tier, why) = state::score_task_tier(task);
                    match tier {
                        "HEAVY" => {
                            println!();
                            println!("  ⚠  Task tier: HEAVY ({why}).");
                            println!(
                                "     Recommendation: run `aiplus-auto-team-consultant` before \
                                 staffing this dispatch, and double-check Owner sign-off on \
                                 STOP-gated actions (submission / posting / authorship / release)."
                            );
                        }
                        "MEDIUM" => {
                            println!();
                            println!("  ℹ  Task tier: MEDIUM ({why}).");
                            println!(
                                "     Recommendation: brief the consultant team before \
                                 implementation; flag identification-adjacent assumptions to \
                                 Theorist/Architect first."
                            );
                        }
                        _ => {
                            // LIGHT — no nudge.
                        }
                    }
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
                return Ok(());
            }
        }
    }
    println!("Routing task to PI/CEO for scoring and dispatch: {task}");
    Ok(())
}
