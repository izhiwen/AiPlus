use crate::agent::worktree;
use anyhow::Result;

pub fn handle_prune_worktrees(yes: bool) -> Result<()> {
    let project_root = std::env::current_dir()?;

    let stale_worktrees = worktree::get_stale_worktrees(&project_root)?;

    if stale_worktrees.is_empty() {
        println!("No stale worktrees found.");
        return Ok(());
    }

    println!("Stale worktrees to prune:");
    for (role, path) in &stale_worktrees {
        println!("  {}: {}", role, path.display());
    }

    if !yes {
        println!("Run with --yes to confirm pruning.");
        return Ok(());
    }

    println!("Pruning {} stale worktree(s)...", stale_worktrees.len());
    for (_role, path) in &stale_worktrees {
        if let Err(e) = worktree::remove_worktree(&project_root, path) {
            eprintln!("  ERROR: Failed to remove {}: {}", path.display(), e);
        }
    }

    Ok(())
}
