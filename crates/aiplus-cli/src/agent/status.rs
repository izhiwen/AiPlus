use crate::agent::core::load_team_config;
use anyhow::Result;
use serde_json;

pub fn handle_status(verbose: bool, json_output: bool) -> Result<()> {
    let project_root = std::env::current_dir()?;
    let state = load_team_config(&project_root)?;
    let cache_status = crate::agent::cache::disk_cache_status(&project_root).ok();

    if json_output {
        let cache_json = cache_status.as_ref().map(|status| {
            let roles = status
                .meta
                .as_ref()
                .map(|meta| {
                    meta.roles
                        .iter()
                        .map(|(role, entry)| {
                            serde_json::json!({
                                "role": role,
                                "cache_source": entry.cache_source,
                                "ttl_seconds": entry.ttl_seconds,
                                "bytes": entry.bytes,
                                "last_used_at_ms": entry.last_used_at_ms,
                            })
                        })
                        .collect::<Vec<_>>()
                })
                .unwrap_or_default();
            serde_json::json!({
                "enabled": status.enabled,
                "project": status.project,
                "cache_dir": status.project_dir.display().to_string(),
                "roles": roles,
                "sync_warning": status.sync_warning,
            })
        });
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
            "disk_cache": cache_json,
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
                Some(other) => {
                    println!("Active team: {team}  (switch with `aiplus agent set-team {other}`)")
                }
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

        if verbose {
            println!("\nDisk warm cache:");
            if let Some(status) = cache_status {
                println!(
                    "  disk_cache={}",
                    if status.enabled {
                        "enabled"
                    } else {
                        "disabled"
                    }
                );
                println!("  cache_dir={}", status.project_dir.display());
                if let Some(meta) = status.meta {
                    if meta.roles.is_empty() {
                        println!("  roles=[]");
                    } else {
                        for (role, entry) in meta.roles {
                            println!(
                                "  role={} cache_source={} ttl_seconds={} bytes={}",
                                role, entry.cache_source, entry.ttl_seconds, entry.bytes
                            );
                        }
                    }
                }
                if let Some(warning) = status.sync_warning {
                    println!("  WARNING: {warning}");
                }
            } else {
                println!("  disk_cache=unavailable");
            }
        }
    }

    Ok(())
}
