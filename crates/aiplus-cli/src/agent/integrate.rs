use crate::agent::worktree;
use anyhow::Result;

pub fn handle_integrate(role: &str) -> Result<()> {
    let project_root = std::env::current_dir()?;
    worktree::merge_agent_branch(&project_root, role)
}
