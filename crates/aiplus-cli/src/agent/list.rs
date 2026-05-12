use crate::agent::core::{list_roles, load_team_config};
use anyhow::Result;

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
