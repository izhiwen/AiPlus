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
        // Show which virtual team is currently active.
        if let Some(team) = crate::agent::set_team::read_active_team(&project_root) {
            let other = match team.as_str() {
                "agent-team" => Some("aieconlab"),
                "aieconlab" => Some("agent-team"),
                _ => None,
            };
            match other {
                Some(other) => println!(
                    "Active team: {team}  (switch with `aiplus agent set-team {other}`)"
                ),
                None => println!("Active team: {team}"),
            }
        }
        println!();

        println!("Team Roster:");
        // Read persisted active roles from .aiplus/agents/active-roles.json
        // (populated by `aiplus agent route <role>`). Falls back to whatever
        // load_team_config inferred if persistence layer is empty.
        let project_root = std::env::current_dir().ok();
        let persisted = project_root
            .as_ref()
            .and_then(|root| crate::agent::state::load_active_roles(root).ok())
            .map(|state| state.active_roles)
            .unwrap_or_default();
        if !persisted.is_empty() {
            let mut sorted: Vec<&String> = persisted.iter().collect();
            sorted.sort();
            println!("  Active roles: {:?}", sorted);
        } else if state.active_roles.is_empty() && state.disabled_roles.is_empty() {
            let core_count = state.agents.len() - state.stub_roles.len();
            println!(
                "  Active roles: [] ({} core roles configured; no dispatches recorded yet — run `aiplus agent route <role>` to mark one active)",
                core_count
            );
        } else {
            println!("  Active roles: {:?}", state.active_roles);
        }
        println!("  Disabled roles: {:?}", state.disabled_roles);
        println!("  Stub roles (v0.2): {:?}", state.stub_roles);
        println!("  Total agents: {}", state.agents.len());

        if !state.worktree_paths.is_empty() {
            let missing_count = state
                .worktree_paths
                .values()
                .filter(|p| !p.exists())
                .count();
            let total = state.worktree_paths.len();
            let provisioned = total - missing_count;
            if missing_count == total {
                println!(
                    "\nWorktree status: 0 of {} provisioned (lazy — created on demand when the PI/CEO dispatches a task to a role):",
                    total
                );
            } else {
                println!(
                    "\nWorktree status: {} of {} provisioned:",
                    provisioned, total
                );
            }
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
