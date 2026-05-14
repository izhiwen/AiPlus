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

    println!("Doctor check complete.");
    Ok(())
}
