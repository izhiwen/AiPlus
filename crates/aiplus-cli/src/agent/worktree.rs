use anyhow::{Context, Result};
use std::path::{Path, PathBuf};
use std::process::Command;

#[derive(Debug, Clone)]
pub struct WorktreeEntry {
    pub path: PathBuf,
    pub branch: Option<String>,
    pub head: Option<String>,
    pub bare: bool,
}

/// List all git worktrees for the repository at project_root.
pub fn list_worktrees(project_root: &Path) -> Result<Vec<WorktreeEntry>> {
    let output = Command::new("git")
        .args(["worktree", "list", "--porcelain"])
        .current_dir(project_root)
        .output()
        .context("Failed to run 'git worktree list --porcelain'. Is git installed?")?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        anyhow::bail!("git worktree list failed: {}", stderr);
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    parse_worktree_list(&stdout)
}

fn parse_worktree_list(output: &str) -> Result<Vec<WorktreeEntry>> {
    let mut entries = Vec::new();
    let mut current: Option<WorktreeEntry> = None;

    for line in output.lines() {
        if line.is_empty() {
            if let Some(entry) = current.take() {
                entries.push(entry);
            }
            continue;
        }

        if line.starts_with("worktree ") {
            if let Some(entry) = current.take() {
                entries.push(entry);
            }
            let path = line.strip_prefix("worktree ").unwrap_or("").trim();
            current = Some(WorktreeEntry {
                path: PathBuf::from(path),
                branch: None,
                head: None,
                bare: false,
            });
        } else if let Some(ref mut entry) = current {
            if line.starts_with("branch ") {
                entry.branch = Some(
                    line.strip_prefix("branch ")
                        .unwrap_or("")
                        .trim()
                        .to_string(),
                );
            } else if line.starts_with("HEAD ") {
                entry.head = Some(line.strip_prefix("HEAD ").unwrap_or("").trim().to_string());
            } else if line == "bare" {
                entry.bare = true;
            }
        }
    }

    if let Some(entry) = current {
        entries.push(entry);
    }

    Ok(entries)
}

/// Check if a worktree already exists for the given agent role.
/// Returns the path if found and the directory still exists.
pub fn worktree_exists_for_role(project_root: &Path, role: &str) -> Result<Option<PathBuf>> {
    let worktrees = list_worktrees(project_root)?;
    let expected_branch = format!("agent/{}", role);
    let expected_branch_ref = format!("refs/heads/{}", expected_branch);

    for wt in worktrees {
        if let Some(ref branch) = wt.branch {
            if (branch == &expected_branch || branch == &expected_branch_ref) && wt.path.exists() {
                return Ok(Some(wt.path));
            }
        }
    }

    Ok(None)
}

/// Check if a given path is tracked as a git worktree (any branch).
pub fn is_tracked_worktree_path(project_root: &Path, path: &Path) -> Result<bool> {
    let worktrees = list_worktrees(project_root)?;
    let canonical_path = std::fs::canonicalize(path).unwrap_or_else(|_| path.to_path_buf());

    for wt in &worktrees {
        let wt_canonical = std::fs::canonicalize(&wt.path).unwrap_or_else(|_| wt.path.clone());
        if wt_canonical == canonical_path {
            return Ok(true);
        }
    }
    Ok(false)
}

/// Detect if a path is inside a cloud sync folder.
pub fn detect_sync_folder(path: &Path) -> Option<String> {
    let path_str = path.to_string_lossy().to_lowercase();
    let sync_names = [
        ("dropbox", "Dropbox"),
        ("icloud", "iCloud"),
        ("onedrive", "OneDrive"),
        ("google drive", "Google Drive"),
        ("box sync", "Box Sync"),
    ];

    for (lower, display) in &sync_names {
        if path_str.contains(lower) {
            return Some(display.to_string());
        }
    }

    None
}

/// Get the repository name from the project root directory basename.
pub fn get_repo_name(project_root: &Path) -> Result<String> {
    project_root
        .file_name()
        .and_then(|n| n.to_str())
        .map(|s| s.to_string())
        .context("Could not determine repository name from project root")
}

/// Get the user's home directory.
fn home_dir() -> Result<PathBuf> {
    #[cfg(not(windows))]
    {
        std::env::var("HOME")
            .map(PathBuf::from)
            .context("HOME environment variable not set")
    }
    #[cfg(windows)]
    {
        std::env::var("USERPROFILE")
            .map(PathBuf::from)
            .context("USERPROFILE environment variable not set")
    }
}

/// Compute the fallback worktree path under ~/.aiplus/worktrees/.
pub fn get_fallback_worktree_path(project_root: &Path, role: &str) -> Result<PathBuf> {
    let home = home_dir()?;
    let repo_name = get_repo_name(project_root)?;

    Ok(home
        .join(".aiplus")
        .join("worktrees")
        .join(&repo_name)
        .join(role))
}

/// Validate that the parent directory of the worktree path is suitable.
pub fn check_parent_dir(project_root: &Path, path: &Path) -> Result<()> {
    let parent = path
        .parent()
        .context("Worktree path has no parent directory")?;

    if !parent.exists() {
        std::fs::create_dir_all(parent)
            .with_context(|| format!("Failed to create parent directory: {}", parent.display()))?;
    }

    // Check parent is writable by trying to write a temp file
    let test_file = parent.join(".aiplus-write-test");
    match std::fs::write(&test_file, b"") {
        Ok(_) => {
            let _ = std::fs::remove_file(&test_file);
        }
        Err(e) => {
            anyhow::bail!(
                "Parent directory is not writable: {} ({})",
                parent.display(),
                e
            );
        }
    }

    // Check no foreign .git in parent
    let parent_git = parent.join(".git");
    if parent_git.exists() {
        let project_git = project_root.join(".git");
        if parent_git != project_git {
            anyhow::bail!(
                "Foreign .git directory detected in parent: {}. \
                 This would create a nested repository. \
                 Please choose a different worktree location.",
                parent.display()
            );
        }
    }

    Ok(())
}

/// Resolve the worktree path from template or default.
/// Default is sibling: ../{project_name}.{role}
pub fn resolve_worktree_path(
    project_root: &Path,
    role: &str,
    template: Option<&str>,
) -> Result<PathBuf> {
    let repo_name = get_repo_name(project_root)?;
    let path_str = match template {
        Some(t) if !t.is_empty() => t.replace("{project_name}", &repo_name),
        _ => format!("../{}.{}", repo_name, role),
    };
    Ok(project_root.join(path_str))
}

/// Check if a git branch exists.
pub fn branch_exists(project_root: &Path, branch: &str) -> Result<bool> {
    let output = Command::new("git")
        .args(["rev-parse", "--verify", branch])
        .current_dir(project_root)
        .output()
        .context("Failed to check if branch exists")?;

    Ok(output.status.success())
}

/// Create a git worktree for the given agent role.
///
/// Schema v0.1.4 requirements:
/// - Always uses `git worktree add -B agent/<role> <path>`
/// - Creates worktree at sibling level by default
/// - Warns about sync folders but continues with adjusted path
/// - Reuses existing worktrees, errors on untracked directory collisions
pub fn create_worktree(project_root: &Path, role: &str, template: Option<&str>) -> Result<PathBuf> {
    // Ensure this is a git repo
    if !project_root.join(".git").exists() {
        anyhow::bail!(
            "Not a git repository: {}. \
             Agent worktrees require the project to be a git repository.",
            project_root.display()
        );
    }

    // 1. Check if worktree already exists for this role -> reuse
    if let Some(existing_path) = worktree_exists_for_role(project_root, role)? {
        println!(
            "Reusing existing worktree for {} at {}",
            role,
            existing_path.display()
        );
        return Ok(existing_path);
    }

    // 2. Resolve the target path
    let mut path = resolve_worktree_path(project_root, role, template)?;

    // 3. Sync folder detection
    if let Some(sync_name) = detect_sync_folder(project_root) {
        eprintln!(
            "WARNING: Project root appears to be inside a {} sync folder.",
            sync_name
        );
        eprintln!("Git worktrees in sync folders can cause conflicts and data loss.");

        // If the resolved path would also be in a sync folder, use fallback
        if detect_sync_folder(&path).is_some() {
            path = get_fallback_worktree_path(project_root, role)?;
            eprintln!(
                "Using fallback location outside sync folder: {}",
                path.display()
            );
        } else {
            eprintln!("Using sibling path: {}", path.display());
        }
        eprintln!();
    }

    // 4. Collision handling
    if path.exists() {
        // Path exists - check if it's tracked as a worktree
        if is_tracked_worktree_path(project_root, &path)? {
            anyhow::bail!(
                "Worktree path {} already exists and is tracked by git for a different branch. \
                 Please resolve this collision manually.",
                path.display()
            );
        } else {
            anyhow::bail!(
                "Worktree directory {} already exists but is not tracked by git worktree. \
                 Please remove it or choose a different location.",
                path.display()
            );
        }
    }

    create_worktree_at_path(project_root, role, &path)
}

fn create_worktree_at_path(project_root: &Path, role: &str, path: &Path) -> Result<PathBuf> {
    check_parent_dir(project_root, path)?;

    let branch = format!("agent/{}", role);

    // Schema v0.1.4: Always use -B to create/reset the branch
    let output = Command::new("git")
        .args([
            "worktree",
            "add",
            "-B",
            &branch,
            path.to_str().unwrap_or(""),
        ])
        .current_dir(project_root)
        .output()
        .with_context(|| {
            format!(
                "Failed to run 'git worktree add -B {} {}' for {}",
                branch,
                path.display(),
                role
            )
        })?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        anyhow::bail!(
            "Failed to create worktree for {} at {}: {}",
            role,
            path.display(),
            stderr
        );
    }

    // Return canonical path for consistent comparisons with git output
    let canonical_path = std::fs::canonicalize(path).unwrap_or_else(|_| path.to_path_buf());

    println!(
        "Created worktree for {} at {}",
        role,
        canonical_path.display()
    );
    println!("  (Branch: {})", branch);

    Ok(canonical_path)
}

/// Remove a git worktree at the given path.
/// Uses `git worktree remove` (never rm -rf).
/// Falls back to --force if the directory is already gone.
pub fn remove_worktree(project_root: &Path, path: &Path) -> Result<()> {
    let path_str = path.to_str().unwrap_or("");

    let output = Command::new("git")
        .args(["worktree", "remove", path_str])
        .current_dir(project_root)
        .output()
        .with_context(|| format!("Failed to run 'git worktree remove {}'", path.display()))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);

        // Try force remove
        let force_output = Command::new("git")
            .args(["worktree", "remove", "--force", path_str])
            .current_dir(project_root)
            .output()
            .with_context(|| {
                format!(
                    "Failed to run 'git worktree remove --force {}'",
                    path.display()
                )
            })?;

        if !force_output.status.success() {
            let force_stderr = String::from_utf8_lossy(&force_output.stderr);
            anyhow::bail!(
                "Failed to remove worktree at {}: {}\nForce attempt also failed: {}",
                path.display(),
                stderr,
                force_stderr
            );
        }

        println!("Force-removed worktree at {}", path.display());
    } else {
        println!("Removed worktree at {}", path.display());
    }

    Ok(())
}

/// Prune orphaned worktrees (remove stale entries from git worktree list).
pub fn prune_orphaned_worktrees(project_root: &Path) -> Result<()> {
    let output = Command::new("git")
        .args(["worktree", "prune"])
        .current_dir(project_root)
        .output()
        .context("Failed to run 'git worktree prune'")?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        anyhow::bail!("git worktree prune failed: {}", stderr);
    }

    Ok(())
}

/// Get all worktrees with agent/* branches.
pub fn get_all_agent_worktrees(project_root: &Path) -> Result<Vec<(String, PathBuf)>> {
    let worktrees = list_worktrees(project_root)?;
    let mut agent_worktrees = Vec::new();

    for wt in worktrees {
        // Skip the main worktree
        if wt.path == project_root {
            continue;
        }

        if let Some(ref branch) = wt.branch {
            let role = branch
                .strip_prefix("refs/heads/agent/")
                .or_else(|| branch.strip_prefix("agent/"))
                .map(|s| s.to_string());

            if let Some(role) = role {
                agent_worktrees.push((role, wt.path));
            }
        }
    }

    Ok(agent_worktrees)
}

/// Remove all agent worktrees.
pub fn prune_agent_worktrees(project_root: &Path) -> Result<Vec<PathBuf>> {
    let agent_worktrees = get_all_agent_worktrees(project_root)?;
    let mut removed = Vec::new();

    for (role, path) in agent_worktrees {
        println!("Removing worktree for {} at {}", role, path.display());
        match remove_worktree(project_root, &path) {
            Ok(_) => removed.push(path),
            Err(e) => eprintln!("  ERROR: Failed to remove worktree for {}: {}", role, e),
        }
    }

    // Prune orphaned entries
    if let Err(e) = prune_orphaned_worktrees(project_root) {
        eprintln!("WARNING: Failed to prune orphaned worktrees: {}", e);
    }

    Ok(removed)
}

/// Merge an agent's branch into the current branch.
pub fn merge_agent_branch(project_root: &Path, role: &str) -> Result<()> {
    let branch = format!("agent/{}", role);

    // Ensure this is a git repo
    if !project_root.join(".git").exists() {
        anyhow::bail!(
            "Not a git repository: {}. \
             Agent integration requires the project to be a git repository.",
            project_root.display()
        );
    }

    // Verify the branch exists
    if !branch_exists(project_root, &branch)? {
        anyhow::bail!(
            "Agent branch '{}' does not exist. Has {} done any work yet?",
            branch,
            role
        );
    }

    // Check for uncommitted changes. Skip runtime-state files that the agent
    // CLI writes during normal operation (dispatch log, active-roles state)
    // — these are local-only artifacts equivalent to log files and should
    // not block git operations.
    let status_output = Command::new("git")
        .args(["status", "--porcelain"])
        .current_dir(project_root)
        .output()
        .context("Failed to check git status")?;

    let runtime_state_paths: &[&str] = &[
        ".aiplus/agents/dispatch-log.jsonl",
        ".aiplus/agents/active-roles.json",
        ".aiplus/agents/active-team.txt",
        ".aiplus/agents/_teams/",
        ".aiplus/memory/audit.jsonl",
        ".aiplus/memory/facts.jsonl",
        ".aiplus/memory/decisions.jsonl",
        ".aiplus/memory/project-memory.jsonl",
    ];
    let blocking = String::from_utf8_lossy(&status_output.stdout)
        .lines()
        .filter(|line| {
            // git status --porcelain format: "XY path" (3 prefix bytes)
            line.len() > 3
                && !runtime_state_paths
                    .iter()
                    .any(|state| line[3..].trim_start_matches('"').starts_with(state))
        })
        .count();

    if blocking > 0 {
        anyhow::bail!(
            "Working directory has uncommitted changes. \
             Please commit or stash them before integrating."
        );
    }

    // Perform the merge
    println!("Merging {} into current branch...", branch);
    let output = Command::new("git")
        .args([
            "-c",
            "core.hooksPath=/dev/null",
            "merge",
            "--no-ff",
            &branch,
            "-m",
            &format!("Integrate {} work", role),
        ])
        .current_dir(project_root)
        .output()
        .with_context(|| format!("Failed to merge {}", branch))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);

        // Check if it's a merge conflict
        let has_conflicts = Command::new("git")
            .args(["diff", "--name-only", "--diff-filter=U"])
            .current_dir(project_root)
            .output()
            .context("Failed to check for merge conflicts")?;

        let conflict_list = String::from_utf8_lossy(&has_conflicts.stdout);
        if !conflict_list.trim().is_empty() {
            eprintln!("Merge conflicts detected!");
            eprintln!("Please resolve conflicts manually and then commit.");
            eprintln!("Conflicted files:");
            for line in conflict_list.lines() {
                if !line.is_empty() {
                    eprintln!("  {}", line);
                }
            }
            eprintln!();
            eprintln!("To abort the merge, run: git merge --abort");

            anyhow::bail!(
                "Merge of {} resulted in conflicts. Resolve them manually and commit.",
                branch
            );
        }

        anyhow::bail!("Merge failed: {}", stderr);
    }

    println!("Successfully integrated {} into current branch.", role);
    Ok(())
}

/// Find all stale worktrees (registered in git but directory is missing).
// TODO(v0.2): used by `aiplus agent prune-worktrees --stale-only` once
// added. Currently `prune-worktrees` is unconditional; this helper waits.
#[allow(dead_code)]
pub fn get_stale_worktrees(project_root: &Path) -> Result<Vec<(String, PathBuf)>> {
    let worktrees = list_worktrees(project_root)?;
    let mut stale = Vec::new();

    for wt in worktrees {
        // Skip the main worktree (the project root itself)
        if wt.path == project_root {
            continue;
        }

        // A worktree is stale if its directory doesn't exist
        if !wt.path.exists() {
            // Try to extract role from branch name
            let role = wt
                .branch
                .as_ref()
                .and_then(|b| b.strip_prefix("refs/heads/agent/"))
                .or_else(|| wt.branch.as_ref().and_then(|b| b.strip_prefix("agent/")))
                .unwrap_or("unknown")
                .to_string();

            stale.push((role, wt.path));
        }
    }

    Ok(stale)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    fn setup_git_repo() -> (TempDir, PathBuf) {
        let temp = TempDir::new().unwrap();
        let repo = temp.path().join("test-repo");
        fs::create_dir(&repo).unwrap();

        // Initialize git repo
        let output = Command::new("git")
            .args(["init"])
            .current_dir(&repo)
            .output()
            .expect("git init failed");
        assert!(
            output.status.success(),
            "git init failed: {}",
            String::from_utf8_lossy(&output.stderr)
        );

        // Configure git user for commits
        Command::new("git")
            .args(["config", "user.email", "test@example.com"])
            .current_dir(&repo)
            .output()
            .unwrap();
        Command::new("git")
            .args(["config", "user.name", "Test User"])
            .current_dir(&repo)
            .output()
            .unwrap();

        // Create initial commit
        fs::write(repo.join("README.md"), "# Test\n").unwrap();
        Command::new("git")
            .args(["add", "README.md"])
            .current_dir(&repo)
            .output()
            .unwrap();
        Command::new("git")
            .args(["commit", "--no-verify", "-m", "Initial commit"])
            .current_dir(&repo)
            .output()
            .unwrap();

        (temp, repo)
    }

    #[test]
    fn test_create_worktree_succeeds() {
        let (_temp, repo) = setup_git_repo();
        let worktree_path = create_worktree(&repo, "engineer-a", None).unwrap();

        assert!(worktree_path.exists());
    }

    #[test]
    fn test_worktree_branch_is_correct() {
        let (_temp, repo) = setup_git_repo();
        let worktree_path = create_worktree(&repo, "engineer-a", None).unwrap();

        // Check branch in worktree
        let output = Command::new("git")
            .args(["branch", "--show-current"])
            .current_dir(&worktree_path)
            .output()
            .unwrap();
        let branch = String::from_utf8_lossy(&output.stdout).trim().to_string();
        assert_eq!(branch, "agent/engineer-a");
    }

    #[test]
    fn test_worktree_listed_by_git() {
        let (_temp, repo) = setup_git_repo();
        let worktree_path = create_worktree(&repo, "engineer-a", None).unwrap();

        let worktrees = list_worktrees(&repo).unwrap();
        let canonical_wt = std::fs::canonicalize(&worktree_path).unwrap();
        let found = worktrees.iter().any(|wt| {
            let wt_canonical = std::fs::canonicalize(&wt.path).unwrap_or_else(|_| wt.path.clone());
            wt_canonical == canonical_wt
                && (wt.branch.as_deref() == Some("agent/engineer-a")
                    || wt.branch.as_deref() == Some("refs/heads/agent/engineer-a"))
        });
        assert!(found, "Worktree should be listed by git worktree list");
    }

    #[test]
    fn test_reuse_existing_worktree() {
        let (_temp, repo) = setup_git_repo();
        let path1 = create_worktree(&repo, "engineer-a", None).unwrap();
        let path2 = create_worktree(&repo, "engineer-a", None).unwrap();

        assert_eq!(path1, path2, "Should reuse existing worktree");
    }

    #[test]
    fn test_untracked_directory_collision_errors() {
        let (_temp, repo) = setup_git_repo();

        // Create the collision directory
        let collision_path = repo.parent().unwrap().join("test-repo.engineer-a");
        fs::create_dir(&collision_path).unwrap();

        let result = create_worktree(&repo, "engineer-a", None);
        assert!(
            result.is_err(),
            "Should error on untracked directory collision"
        );
        let err = result.unwrap_err().to_string();
        assert!(
            err.contains("not tracked by git worktree"),
            "Error should mention untracked: {}",
            err
        );
    }

    #[test]
    fn test_dismiss_removes_worktree() {
        let (_temp, repo) = setup_git_repo();
        let worktree_path = create_worktree(&repo, "engineer-a", None).unwrap();

        assert!(worktree_path.exists());
        remove_worktree(&repo, &worktree_path).unwrap();
        assert!(!worktree_path.exists(), "Worktree should be removed");
    }

    #[test]
    fn test_integration_merges_correctly() {
        let (_temp, repo) = setup_git_repo();

        // Create worktree
        let worktree_path = create_worktree(&repo, "engineer-a", None).unwrap();

        // Make a commit in the worktree
        fs::write(worktree_path.join("feature.txt"), "new feature\n").unwrap();
        Command::new("git")
            .args(["add", "feature.txt"])
            .current_dir(&worktree_path)
            .output()
            .unwrap();
        Command::new("git")
            .args(["commit", "--no-verify", "-m", "Add feature"])
            .current_dir(&worktree_path)
            .output()
            .unwrap();

        // Merge back to main
        merge_agent_branch(&repo, "engineer-a").unwrap();

        // Verify the file exists in main
        assert!(
            repo.join("feature.txt").exists(),
            "Feature file should be merged to main"
        );
    }

    #[test]
    fn test_sync_folder_detection() {
        let dropbox_path = PathBuf::from("/Users/steve/Dropbox/Project/test");
        assert_eq!(
            detect_sync_folder(&dropbox_path),
            Some("Dropbox".to_string())
        );

        let icloud_path = PathBuf::from("/Users/steve/iCloud Drive/test");
        assert_eq!(detect_sync_folder(&icloud_path), Some("iCloud".to_string()));

        let normal_path = PathBuf::from("/Users/steve/projects/test");
        assert_eq!(detect_sync_folder(&normal_path), None);
    }

    #[test]
    fn test_prune_agent_worktrees() {
        let (_temp, repo) = setup_git_repo();

        // Create multiple agent worktrees
        let path1 = create_worktree(&repo, "engineer-a", None).unwrap();
        let path2 = create_worktree(&repo, "engineer-b", None).unwrap();

        assert!(path1.exists());
        assert!(path2.exists());

        let removed = prune_agent_worktrees(&repo).unwrap();
        assert_eq!(removed.len(), 2);

        assert!(!path1.exists());
        assert!(!path2.exists());
    }

    #[test]
    fn test_get_stale_worktrees() {
        let (_temp, repo) = setup_git_repo();
        let worktree_path = create_worktree(&repo, "engineer-a", None).unwrap();

        // Manually remove the directory to make it stale
        fs::remove_dir_all(&worktree_path).unwrap();

        let stale = get_stale_worktrees(&repo).unwrap();
        assert_eq!(stale.len(), 1);
        assert_eq!(stale[0].0, "engineer-a");
    }

    #[test]
    fn test_create_worktree_uses_sibling_path() {
        let (_temp, repo) = setup_git_repo();
        let worktree_path = create_worktree(&repo, "engineer-a", None).unwrap();

        // Should be at sibling level: ../test-repo.engineer-a
        let expected_name = "test-repo.engineer-a";
        assert!(
            worktree_path.to_string_lossy().contains(expected_name),
            "Worktree should be at sibling path, got: {}",
            worktree_path.display()
        );
    }

    #[test]
    fn test_create_worktree_always_uses_dash_b() {
        let (_temp, repo) = setup_git_repo();

        // Create a branch manually first
        Command::new("git")
            .args(["branch", "agent/engineer-a"])
            .current_dir(&repo)
            .output()
            .unwrap();

        // create_worktree should still succeed (the branch exists but no worktree)
        let worktree_path = create_worktree(&repo, "engineer-a", None).unwrap();
        assert!(worktree_path.exists());

        // Verify it's on the correct branch
        let output = Command::new("git")
            .args(["branch", "--show-current"])
            .current_dir(&worktree_path)
            .output()
            .unwrap();
        let branch = String::from_utf8_lossy(&output.stdout).trim().to_string();
        assert_eq!(branch, "agent/engineer-a");
    }
}
