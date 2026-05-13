pub mod audit;
pub mod cache;
pub mod commands;
pub mod core;
pub mod disable;
pub mod dismiss;
pub mod doctor;
pub mod enable;
pub mod integrate;
pub mod invite;
pub mod list;
pub mod prune_worktrees;
pub mod reset;
pub mod route;
pub mod set_team;
pub mod state;
pub mod status;
pub mod talk;
pub mod transcript;
pub mod worktree;

#[allow(unused_imports)]
pub use aiplus_core::agent_team::{
    AcceptanceMode, AuditArgs, AuditSub, AuditorVerdict, BlockedReason, CheckKind, RoleId, Tier,
};
pub use commands::{AgentArgs, AgentSub};

use anyhow::Result;

pub fn dispatch(args: AgentArgs) -> Result<()> {
    match args.subcommand {
        AgentSub::Status { verbose: _, json } => status::handle_status(json),
        AgentSub::Doctor => doctor::handle_doctor(),
        AgentSub::List { functional } => list::handle_list(functional),
        AgentSub::Talk { role } => talk::handle_talk(&role),
        AgentSub::Route {
            role,
            task,
            role_opt: _,
        } => route::handle_route(role.as_deref(), &task.join(" ")),
        AgentSub::Reset => reset::handle_reset(),
        AgentSub::SetTeam { team } => set_team::handle_set_team(&team),
        AgentSub::Invite { role } => invite::handle_invite(&role),
        AgentSub::Dismiss { role } => dismiss::handle_dismiss(&role),
        AgentSub::Disable { role } => disable::handle_disable(&role),
        AgentSub::Enable { role } => enable::handle_enable(&role),
        AgentSub::Integrate { role } => integrate::handle_integrate(&role),
        AgentSub::Transcript => transcript::handle_transcript(),
        AgentSub::PruneWorktrees { yes } => prune_worktrees::handle_prune_worktrees(yes),
        AgentSub::Audit(args) => audit::dispatch(args),
    }
}
