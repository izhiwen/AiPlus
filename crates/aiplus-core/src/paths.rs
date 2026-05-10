use anyhow::{anyhow, Result};
use std::path::{Component, Path, PathBuf};

pub fn project_local_path(root: &Path, rel: &str) -> Result<PathBuf> {
    let rel_path = Path::new(rel);
    if rel_path.is_absolute() {
        return Err(anyhow!("absolute paths are not project-local: {rel}"));
    }
    if rel_path
        .components()
        .any(|component| matches!(component, Component::ParentDir))
    {
        return Err(anyhow!("parent traversal is not project-local: {rel}"));
    }
    Ok(root.join(rel_path))
}

pub fn memory_dir(root: &Path) -> Result<PathBuf> {
    project_local_path(root, ".aiplus/memory")
}

pub fn identity_dir(root: &Path) -> Result<PathBuf> {
    project_local_path(root, ".aiplus/identities")
}

pub fn skills_dir(root: &Path) -> Result<PathBuf> {
    project_local_path(root, ".aiplus/skills")
}

pub fn is_allowed_project_write(rel: &str) -> bool {
    rel == "AGENTS.md"
        || rel.starts_with(".aiplus/")
        || rel.starts_with(".codex/compact/")
        || rel.starts_with(".claude/")
        || rel.starts_with(".opencode/")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn project_local_path_rejects_absolute_and_parent_paths() {
        let root = Path::new("/tmp/project");
        assert!(project_local_path(root, ".aiplus/manifest.json").is_ok());
        assert!(project_local_path(root, "../outside").is_err());
        assert!(project_local_path(root, "/tmp/outside").is_err());
    }
}
