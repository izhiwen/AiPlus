use crate::agent::core::load_team_config;
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
