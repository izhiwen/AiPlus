use crate::agent::worktree;
use anyhow::Result;

pub fn handle_prune_worktrees(yes: bool) -> Result<()> {
    let project_root = std::env::current_dir()?;

    let agent_worktrees = worktree::get_all_agent_worktrees(&project_root)?;

    if agent_worktrees.is_empty() {
        println!("No agent worktrees found.");
        return Ok(());
    }

    println!("Agent worktrees to prune:");
    for (role, path) in &agent_worktrees {
        println!("  {}: {}", role, path.display());
    }

    if !yes {
        println!("Run with --yes to confirm pruning.");
        return Ok(());
    }

    println!("Pruning {} agent worktree(s)...", agent_worktrees.len());

    let removed = worktree::prune_agent_worktrees(&project_root)?;
    println!("Removed {} worktree(s).", removed.len());

    Ok(())
}
