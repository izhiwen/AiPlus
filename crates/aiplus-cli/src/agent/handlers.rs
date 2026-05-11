use crate::agent::core::{get_role_config, is_stub, list_roles, load_team_config};
use crate::agent::worktree;
use anyhow::Result;

pub fn handle_status() -> Result<()> {
    let project_root = std::env::current_dir()?;
    let state = load_team_config(&project_root)?;

    println!("AiPlus Agent Team v0.1");
    println!("Project root: {}", project_root.display());
    println!();

    println!("Team Roster:");
    println!("  Active roles: {:?}", state.active_roles);
    println!("  Disabled roles: {:?}", state.disabled_roles);
    println!("  Stub roles (v0.2): {:?}", state.stub_roles);
    println!("  Total agents: {}", state.agents.len());

    if !state.worktree_paths.is_empty() {
        println!("\nWorktree status:");
        for (role, path) in &state.worktree_paths {
            let exists = if path.exists() { "exists" } else { "missing" };
            println!("  {}: {} [{}]", role, path.display(), exists);
        }
    } else {
        println!("\nNo worktrees configured.");
    }

    Ok(())
}

pub fn handle_doctor() -> Result<()> {
    let project_root = std::env::current_dir()?;
    let agents_dir = project_root.join(".aiplus").join("agents");

    println!("Running agent team doctor...");
    println!("Checking .aiplus/agents/ directory...");

    if !agents_dir.exists() {
        println!("  WARNING: .aiplus/agents/ does not exist");
        return Ok(());
    }

    let state = load_team_config(&project_root)?;
    println!("  Found {} agent config(s)", state.agents.len());

    for (role, config) in &state.agents {
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

        if let Some(ref wt) = config.worktree_path {
            let path = std::path::PathBuf::from(wt);
            if !path.exists() && config.needs_worktree {
                println!("    WARNING: worktree {} does not exist", path.display());
            }
        }
    }

    println!("Doctor check complete.");
    Ok(())
}

pub fn handle_list(functional_only: bool) -> Result<()> {
    let project_root = std::env::current_dir()?;
    let state = load_team_config(&project_root)?;

    if functional_only {
        println!("Functional experts (v0.1):");
        let functional = [
            "ai-integration",
            "security-reviewer",
            "tech-writer",
            "devops",
            "ui-designer",
            "researcher",
        ];
        for role in &functional {
            let display = state
                .agents
                .get(*role)
                .map(|c| c.display_name.as_str())
                .unwrap_or(role);
            let status = if state.active_roles.contains(&role.to_string()) {
                "active"
            } else {
                "inactive"
            };
            println!("  - {} ({}) [{}]", role, display, status);
        }
    } else {
        println!("All roles:");
        for config in list_roles(&state, false) {
            let status = if config.stub {
                "v0.2 stub"
            } else if state.active_roles.contains(&config.role) {
                "active"
            } else if state.disabled_roles.contains(&config.role) {
                "disabled"
            } else {
                "inactive"
            };
            println!("  - {} ({}) [{}]", config.role, config.display_name, status);
        }
    }

    Ok(())
}

pub fn handle_talk(role: &str) -> Result<()> {
    println!("Starting conversation with {}...", role);
    Ok(())
}

pub fn handle_route(role: Option<&str>, task: &str) -> Result<()> {
    match role {
        Some(r) => {
            println!("Routing task to {}: {}", r, task);
            let config = get_role_config(r)?;
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

pub fn handle_reset() -> Result<()> {
    println!("Resetting agent team state...");
    Ok(())
}

pub fn handle_invite(role: &str) -> Result<()> {
    if is_stub(role) {
        return Err(anyhow::anyhow!(
            "STUB_NOT_INVITABLE: expert is v0.2 stub, not yet functional"
        ));
    }

    println!("Inviting {} to the active team...", role);
    Ok(())
}

pub fn handle_dismiss(role: &str) -> Result<()> {
    println!("Dismissing {} from the active team...", role);
    Ok(())
}

pub fn handle_disable(role: &str) -> Result<()> {
    println!("Disabling {}...", role);
    Ok(())
}

pub fn handle_enable(role: &str) -> Result<()> {
    println!("Enabling {}...", role);
    Ok(())
}

pub fn handle_integrate(role: &str) -> Result<()> {
    let project_root = std::env::current_dir()?;
    worktree::merge_agent_branch(&project_root, role)
}

pub fn handle_transcript() -> Result<()> {
    println!("Transcript feature not yet implemented in v0.1");
    Ok(())
}

pub fn handle_prune_worktrees(yes: bool) -> Result<()> {
    let project_root = std::env::current_dir()?;

    let stale_worktrees = worktree::get_stale_worktrees(&project_root)?;

    if stale_worktrees.is_empty() {
        println!("No stale worktrees found.");
        return Ok(());
    }

    println!("Stale worktrees to prune:");
    for (role, path) in &stale_worktrees {
        println!("  {}: {}", role, path.display());
    }

    if !yes {
        println!("Run with --yes to confirm pruning.");
        return Ok(());
    }

    println!("Pruning {} stale worktree(s)...", stale_worktrees.len());
    for (_role, path) in &stale_worktrees {
        if let Err(e) = worktree::remove_worktree(&project_root, path) {
            eprintln!("  ERROR: Failed to remove {}: {}", path.display(), e);
        }
    }

    Ok(())
}
