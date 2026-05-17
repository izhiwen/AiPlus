use crate::agent::worktree;
use anyhow::Result;
use std::collections::HashMap;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WorktreePoolStatus {
    Created,
    Reused,
    Skipped,
}

impl WorktreePoolStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            WorktreePoolStatus::Created => "created",
            WorktreePoolStatus::Reused => "reused",
            WorktreePoolStatus::Skipped => "skipped",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorktreeLease {
    pub status: WorktreePoolStatus,
    pub path: Option<PathBuf>,
}

impl WorktreeLease {
    pub fn skipped() -> Self {
        Self {
            status: WorktreePoolStatus::Skipped,
            path: None,
        }
    }
}

#[derive(Debug, Default)]
pub struct WorktreePool {
    leases: HashMap<String, PathBuf>,
}

impl WorktreePool {
    pub fn acquire(
        &mut self,
        project_root: &Path,
        role: &str,
        needs_worktree: bool,
        template: Option<&str>,
    ) -> Result<WorktreeLease> {
        if !needs_worktree {
            return Ok(WorktreeLease::skipped());
        }

        if let Some(path) = self.leases.get(role) {
            if path.exists() {
                return Ok(WorktreeLease {
                    status: WorktreePoolStatus::Reused,
                    path: Some(path.clone()),
                });
            }
        }

        if let Some(path) = worktree::worktree_exists_for_role(project_root, role)? {
            self.leases.insert(role.to_string(), path.clone());
            return Ok(WorktreeLease {
                status: WorktreePoolStatus::Reused,
                path: Some(path),
            });
        }

        let path = worktree::create_worktree(project_root, role, template)?;
        self.leases.insert(role.to_string(), path.clone());
        Ok(WorktreeLease {
            status: WorktreePoolStatus::Created,
            path: Some(path),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn skipped_roles_do_not_require_git() {
        let mut pool = WorktreePool::default();
        let lease = pool
            .acquire(Path::new("/definitely/not/a/repo"), "reviewer", false, None)
            .unwrap();
        assert_eq!(lease.status, WorktreePoolStatus::Skipped);
        assert!(lease.path.is_none());
    }
}
