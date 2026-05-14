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

/// Resolve the user's configuration root (e.g. `~/.config` on
/// macOS/Linux, `%APPDATA%` on Windows). Mirrors the CLI-side helper
/// but lives in core so `aiplus-core` callers don't need a layering
/// inversion. Order: `XDG_CONFIG_HOME` → `APPDATA` (Windows) →
/// `HOME/.config` → `USERPROFILE/.config`.
pub fn config_home() -> Result<PathBuf> {
    if let Ok(path) = std::env::var("XDG_CONFIG_HOME") {
        if !path.trim().is_empty() {
            return Ok(PathBuf::from(path));
        }
    }
    #[cfg(target_os = "windows")]
    if let Ok(appdata) = std::env::var("APPDATA") {
        if !appdata.trim().is_empty() {
            return Ok(PathBuf::from(appdata));
        }
    }
    if let Ok(home) = std::env::var("HOME") {
        if !home.trim().is_empty() {
            return Ok(PathBuf::from(home).join(".config"));
        }
    }
    if let Ok(profile) = std::env::var("USERPROFILE") {
        if !profile.trim().is_empty() {
            return Ok(PathBuf::from(profile).join(".config"));
        }
    }
    Err(anyhow!(
        "Cannot determine config directory: none of XDG_CONFIG_HOME, APPDATA, HOME, USERPROFILE \
         are set"
    ))
}

/// User-level global velocity ledger directory at
/// `<config_home>/aiplus/velocity/`. Parallel to per-project
/// `.aiplus/velocity/`. Spec v2 §1.
pub fn global_velocity_dir() -> Result<PathBuf> {
    Ok(config_home()?.join("aiplus").join("velocity"))
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
