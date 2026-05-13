use crate::agent::cache;
use crate::agent::core::get_role_config;
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
                if config.needs_worktree {
                    let project_root = std::env::current_dir()?;
                    match worktree::worktree_exists_for_role(&project_root, candidate)? {
                        Some(path) => {
                            println!("  Using existing worktree: {}", path.display());
                        }
                        None => {
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
