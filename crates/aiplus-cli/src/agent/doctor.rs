use crate::agent::core::load_team_config;
use anyhow::Result;

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

        if let Some(path) = state.worktree_paths.get(role) {
            if !path.exists() && config.needs_worktree {
                println!("    WARNING: worktree {} does not exist", path.display());
            }
        }
    }

    println!("Doctor check complete.");
    Ok(())
}
