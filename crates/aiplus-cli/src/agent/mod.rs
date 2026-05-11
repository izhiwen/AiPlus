pub mod commands;
pub mod core;
pub mod handlers;

pub use commands::{AgentArgs, AgentSub};
pub use handlers::*;

use anyhow::Result;

pub fn dispatch(args: AgentArgs) -> Result<()> {
    match args.subcommand {
        AgentSub::Status => handle_status(),
        AgentSub::Doctor => handle_doctor(),
        AgentSub::List { functional } => handle_list(functional),
        AgentSub::Talk { role } => handle_talk(&role),
        AgentSub::Route { role, task } => handle_route(role.as_deref(), &task.join(" ")),
        AgentSub::Reset => handle_reset(),
        AgentSub::Invite { role } => handle_invite(&role),
        AgentSub::Dismiss { role } => handle_dismiss(&role),
        AgentSub::Disable { role } => handle_disable(&role),
        AgentSub::Enable { role } => handle_enable(&role),
        AgentSub::Integrate { role } => handle_integrate(&role),
        AgentSub::Transcript => handle_transcript(),
        AgentSub::PruneWorktrees { yes } => handle_prune_worktrees(yes),
    }
}
