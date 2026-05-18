pub mod audit;
pub mod cache;
pub mod commands;
pub mod coordinator;
pub mod core;
pub mod disable;
pub mod dismiss;
pub mod dispatch_history;
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
pub mod worktree_pool;

#[allow(unused_imports)]
pub use aiplus_core::agent_team::{
    AcceptanceMode, AuditArgs, AuditSub, AuditorVerdict, BlockedReason, CheckKind, RoleId, Tier,
};
pub use commands::{AgentArgs, AgentSub};

use anyhow::Result;

pub fn dispatch(args: AgentArgs) -> Result<()> {
    match args.subcommand {
        AgentSub::Status { verbose, json } => status::handle_status(verbose, json),
        AgentSub::Doctor => doctor::handle_doctor(),
        AgentSub::Cache {
            enable_disk,
            disable_disk,
            clear,
            status,
        } => cache::handle_cache_command(enable_disk, disable_disk, clear, status),
        AgentSub::List { functional } => list::handle_list(functional),
        AgentSub::Talk { role, runtime } => talk::handle_talk(&role, runtime.as_deref()),
        AgentSub::Route {
            workflow,
            role,
            task,
            role_opt: _,
            owner_approved,
        } => route::handle_route(
            role.as_deref(),
            &task.join(" "),
            &owner_approved,
            workflow.as_deref(),
        ),
        AgentSub::Reset => reset::handle_reset(),
        AgentSub::SetTeam { team } => set_team::handle_set_team(&team),
        AgentSub::Invite { role } => invite::handle_invite(&role),
        AgentSub::Dismiss { role } => dismiss::handle_dismiss(&role),
        AgentSub::Disable { role } => disable::handle_disable(&role),
        AgentSub::Enable { role } => enable::handle_enable(&role),
        AgentSub::Integrate { role } => integrate::handle_integrate(&role),
        AgentSub::Transcript => transcript::handle_transcript(),
        AgentSub::DispatchHistory {
            role,
            outcome,
            since_days,
            json,
        } => dispatch_history::handle_dispatch_history(
            role.as_deref(),
            outcome.as_deref(),
            since_days,
            json,
        ),
        AgentSub::PruneWorktrees { yes } => prune_worktrees::handle_prune_worktrees(yes),
        AgentSub::Audit(args) => audit::dispatch(args),
    }
}
