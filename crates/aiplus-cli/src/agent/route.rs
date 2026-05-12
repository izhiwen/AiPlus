use crate::agent::cache;
use crate::agent::core::get_role_config;
use crate::agent::worktree;
use anyhow::Result;

pub fn handle_route(role: Option<&str>, task: &str) -> Result<()> {
    match role {
        Some(r) => {
            println!("Routing task to {}: {}", r, task);
            let config = get_role_config(r)?;
            if let Ok(cache) = cache::global_cache().lock() {
                cache.invalidate(r, cache::InvalidationReason::RoleRouteCalled);
            }
            if config.needs_worktree {
                let project_root = std::env::current_dir()?;
                match worktree::worktree_exists_for_role(&project_root, r)? {
                    Some(path) => {
                        println!("  Using existing worktree: {}", path.display());
                    }
                    None => {
                        println!("  Creating worktree for {}...", r);
                        let template = config.worktree_path.as_deref();
                        match worktree::create_worktree(&project_root, r, template) {
                            Ok(path) => {
                                println!("  Worktree created: {}", path.display());
                            }
                            Err(e) => {
                                eprintln!("  ERROR: Failed to create worktree: {}", e);
                                return Err(e);
                            }
                        }
                    }
                }
            }
        }
        None => {
            println!("Routing task to CEO for scoring and dispatch: {}", task);
        }
    }
    Ok(())
}
