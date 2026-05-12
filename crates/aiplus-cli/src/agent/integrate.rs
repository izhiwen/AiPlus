use crate::agent::cache;
use crate::agent::worktree;
use anyhow::Result;

pub fn handle_integrate(role: &str) -> Result<()> {
    if let Ok(cache) = cache::global_cache().lock() {
        cache.invalidate(role, cache::InvalidationReason::RoleIntegrateCompleted);
    }
    let project_root = std::env::current_dir()?;
    worktree::merge_agent_branch(&project_root, role)
}
