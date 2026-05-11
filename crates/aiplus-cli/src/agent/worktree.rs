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

/// Detect if the project root is inside a cloud sync folder.
pub fn detect_sync_folder(project_root: &Path) -> Option<String> {
    let path_str = project_root.to_string_lossy().to_lowercase();
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

/// Generate a short unique suffix for collision avoidance.
fn generate_short_uuid() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    let random_part = secs % 10000;
    format!("{:04x}", random_part)
}

/// Resolve the worktree path from template or default.
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

/// Find a non-colliding path by appending a short UUID suffix if needed.
pub fn find_non_colliding_path(base_path: &Path) -> PathBuf {
    if !base_path.exists() {
        return base_path.to_path_buf();
    }

    let short_id = generate_short_uuid();
    let new_path = if let Some(parent) = base_path.parent() {
        if let Some(name) = base_path.file_name().and_then(|n| n.to_str()) {
            parent.join(format!("{}-{}", name, short_id))
        } else {
            PathBuf::from(format!("{}-{}", base_path.display(), short_id))
        }
    } else {
        PathBuf::from(format!("{}-{}", base_path.display(), short_id))
    };

    if !new_path.exists() {
        return new_path;
    }

    // Very unlikely, but try once more
    let short_id2 = generate_short_uuid();
    if let Some(parent) = base_path.parent() {
        if let Some(name) = base_path.file_name().and_then(|n| n.to_str()) {
            parent.join(format!("{}-{}", name, short_id2))
        } else {
            PathBuf::from(format!("{}-{}", base_path.display(), short_id2))
        }
    } else {
        PathBuf::from(format!("{}-{}", base_path.display(), short_id2))
    }
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
/// Handles:
/// - Sync folder detection and fallback
/// - Path collision avoidance
/// - Branch existence checks (uses plain form if branch exists, -B if new)
pub fn create_worktree(project_root: &Path, role: &str, template: Option<&str>) -> Result<PathBuf> {
    // Ensure this is a git repo
    if !project_root.join(".git").exists() {
        anyhow::bail!(
            "Not a git repository: {}. \
             Agent worktrees require the project to be a git repository.",
            project_root.display()
        );
    }

    // Check for sync folders
    if let Some(sync_name) = detect_sync_folder(project_root) {
        eprintln!(
            "ERROR: Project root appears to be inside a {} sync folder.",
            sync_name
        );
        eprintln!("Git worktrees in sync folders can cause conflicts and data loss.");
        eprintln!();

        let fallback = get_fallback_worktree_path(project_root, role)?;
        eprintln!("Recommended fallback location: {}", fallback.display());
        eprintln!("Auto-selecting fallback location to avoid sync conflicts.");

        return create_worktree_at_path(project_root, role, &fallback);
    }

    let base_path = resolve_worktree_path(project_root, role, template)?;
    let path = find_non_colliding_path(&base_path);

    if path != base_path {
        eprintln!(
            "WARNING: Worktree path {} already exists. Using {} instead.",
            base_path.display(),
            path.display()
        );
    }

    create_worktree_at_path(project_root, role, &path)
}

fn create_worktree_at_path(project_root: &Path, role: &str, path: &Path) -> Result<PathBuf> {
    if path.exists() {
        anyhow::bail!(
            "Worktree path already exists and could not be resolved: {}",
            path.display()
        );
    }

    check_parent_dir(project_root, path)?;

    let branch = format!("agent/{}", role);
    let branch_exists = branch_exists(project_root, &branch)?;

    let output = if branch_exists {
        // Branch exists - use plain form to reattach (don't reset with -B)
        Command::new("git")
            .args(["worktree", "add", path.to_str().unwrap_or(""), &branch])
            .current_dir(project_root)
            .output()
            .with_context(|| {
                format!(
                    "Failed to run 'git worktree add {} {}' for {}",
                    path.display(),
                    branch,
                    role
                )
            })?
    } else {
        // Branch doesn't exist - use -B to create and reset
        Command::new("git")
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
            })?
    };

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        anyhow::bail!(
            "Failed to create worktree for {} at {}: {}",
            role,
            path.display(),
            stderr
        );
    }

    println!("Created worktree for {} at {}", role, path.display());
    if branch_exists {
        println!("  (Reattached to existing branch {})", branch);
    } else {
        println!("  (Created new branch {} with -B)", branch);
    }

    Ok(path.to_path_buf())
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

    // Check for uncommitted changes
    let status_output = Command::new("git")
        .args(["status", "--porcelain"])
        .current_dir(project_root)
        .output()
        .context("Failed to check git status")?;

    if !status_output.stdout.is_empty() {
        anyhow::bail!(
            "Working directory has uncommitted changes. \
             Please commit or stash them before integrating."
        );
    }

    // Perform the merge
    println!("Merging {} into current branch...", branch);
    let output = Command::new("git")
        .args([
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
