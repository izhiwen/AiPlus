use crate::agent::core::load_team_config;
use anyhow::Result;
use serde_json;

pub fn handle_status(json_output: bool) -> Result<()> {
    let project_root = std::env::current_dir()?;
    let state = load_team_config(&project_root)?;

    if json_output {
        let output = serde_json::json!({
            "version": "0.1",
            "project_root": project_root.display().to_string(),
            "roles": state.agents.values().map(|a| {
                serde_json::json!({
                    "role_id": a.role,
                    "name": a.display_name,
                    "tier": a.tier,
                    "status": a.status,
                    "stub": a.stub,
                })
            }).collect::<Vec<_>>(),
            "active_roles": state.active_roles,
            "disabled_roles": state.disabled_roles,
            "stub_roles": state.stub_roles,
            "total_agents": state.agents.len(),
            "worktrees": state.worktree_paths.iter().map(|(k, v)| {
                serde_json::json!({
                    "role": k,
                    "path": v.display().to_string(),
                    "exists": v.exists(),
                })
            }).collect::<Vec<_>>(),
        });
        println!("{}", serde_json::to_string_pretty(&output)?);
    } else {
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
    }

    Ok(())
}
