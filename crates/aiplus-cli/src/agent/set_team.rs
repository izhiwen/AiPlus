//! `aiplus agent set-team <name>` — switch which installed virtual team is
//! the *active* team. The active team is the one whose role configs and
//! personas live directly under `.aiplus/agents/`, which is where `aiplus
//! agent status / list / talk / route` read from.
//!
//! Both `agent-team` and `aieconlab` install their team into a snapshot
//! directory under `.aiplus/agents/_teams/<team>/` at install time, and
//! then activate themselves. Calling `set-team` swaps which snapshot is
//! current. The snapshots that are not active stay on disk untouched so
//! switching back is fast (no re-download or re-init from bundled assets).

use anyhow::{anyhow, Context, Result};
use std::path::Path;

const ACTIVE_TEAM_PATH: &str = ".aiplus/agents/active-team.txt";
const SNAPSHOTS_DIR: &str = ".aiplus/agents/_teams";

pub fn handle_set_team(team: &str) -> Result<()> {
    let project_root = std::env::current_dir()?;
    let canonical = match team {
        "agent-team" | "aiplus-agent-team" | "swe" => "agent-team",
        "aieconlab" | "AiEconLab" | "ael" | "AEL" | "econ" | "econ-team" => "aieconlab",
        other => {
            return Err(anyhow!(
                "Unknown team '{}'. Supported: agent-team, aieconlab (and aliases ael, swe).",
                other
            ));
        }
    };
    // Refuse to flip the active marker if the target team isn't actually
    // installed (no snapshot under .aiplus/agents/_teams/<canonical>/).
    // Without this check, `set-team aieconlab` on a project that only has
    // agent-team would silently update active-team.txt but leave the
    // personas/configs as agent-team's — a confusing half-state where
    // `aiplus agent route theorist` then fails because theorist.toml
    // doesn't exist. AEL is opt-in, so this is the typical mistake.
    let snapshot_dir = project_root.join(SNAPSHOTS_DIR).join(canonical);
    let is_installed = snapshot_dir.exists()
        && std::fs::read_dir(&snapshot_dir)
            .map(|mut it| it.next().is_some())
            .unwrap_or(false);
    if !is_installed {
        return Err(anyhow!(
            "Team '{canonical}' is not installed in this project (no snapshot \
             at .aiplus/agents/_teams/{canonical}/). Run `aiplus add {canonical}` \
             first to install it, then retry `aiplus agent set-team {canonical}`."
        ));
    }
    set_active_team(&project_root, canonical)?;
    println!("Active team set to `{canonical}`.");
    println!(
        "Role configs at .aiplus/agents/ now reflect the {} team. \
         Run `aiplus agent status` to confirm.",
        canonical
    );
    Ok(())
}

/// Mark `team_name` as the currently-active team in the project state.
/// Idempotent: if a snapshot exists at `.aiplus/agents/_teams/<team_name>/`,
/// it is copied into the active layout. If not, only the active-team.txt
/// marker is updated (caller is presumed to have just written the active
/// layout directly).
pub fn set_active_team(project_root: &Path, team_name: &str) -> Result<()> {
    let active_path = project_root.join(ACTIVE_TEAM_PATH);
    if let Some(parent) = active_path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(&active_path, format!("{team_name}\n"))
        .with_context(|| format!("write {}", active_path.display()))?;

    let snapshot = project_root.join(SNAPSHOTS_DIR).join(team_name);
    if snapshot.exists() {
        let active_dir = project_root.join(".aiplus").join("agents");
        // Clear current active layout (but preserve _teams/, active-team.txt,
        // dispatch-log.jsonl, active-roles.json).
        prune_active_layout(&active_dir)?;
        // Copy snapshot into active dir.
        copy_dir_recursive(&snapshot, &active_dir)?;
    }
    Ok(())
}

/// Read which team is currently active. None if no marker exists.
pub fn read_active_team(project_root: &Path) -> Option<String> {
    let path = project_root.join(ACTIVE_TEAM_PATH);
    std::fs::read_to_string(path)
        .ok()
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
}

/// Snapshot a team's currently-loaded files (role TOMLs, personas, experts,
/// team config) into `.aiplus/agents/_teams/<team_name>/` so it can be
/// restored later. Called by the init hooks at install time.
pub fn snapshot_active_team(project_root: &Path, team_name: &str) -> Result<()> {
    let active_dir = project_root.join(".aiplus").join("agents");
    let snapshot_dir = project_root.join(SNAPSHOTS_DIR).join(team_name);
    if snapshot_dir.exists() {
        std::fs::remove_dir_all(&snapshot_dir)?;
    }
    std::fs::create_dir_all(&snapshot_dir)?;
    for entry in std::fs::read_dir(&active_dir)? {
        let entry = entry?;
        let name = entry.file_name();
        // Skip runtime / cross-team state.
        if matches!(
            name.to_str(),
            Some("_teams")
                | Some("active-team.txt")
                | Some("dispatch-log.jsonl")
                | Some("active-roles.json")
        ) {
            continue;
        }
        let src = entry.path();
        let dst = snapshot_dir.join(&name);
        if src.is_dir() {
            copy_dir_recursive(&src, &dst)?;
        } else {
            std::fs::copy(&src, &dst)?;
        }
    }
    Ok(())
}

fn prune_active_layout(active_dir: &Path) -> Result<()> {
    if !active_dir.exists() {
        return Ok(());
    }
    for entry in std::fs::read_dir(active_dir)? {
        let entry = entry?;
        let name = entry.file_name();
        // Preserve runtime / cross-team state.
        if matches!(
            name.to_str(),
            Some("_teams")
                | Some("active-team.txt")
                | Some("dispatch-log.jsonl")
                | Some("active-roles.json")
        ) {
            continue;
        }
        let path = entry.path();
        if path.is_dir() {
            std::fs::remove_dir_all(&path)?;
        } else {
            std::fs::remove_file(&path)?;
        }
    }
    Ok(())
}

fn copy_dir_recursive(src: &Path, dst: &Path) -> Result<()> {
    if !dst.exists() {
        std::fs::create_dir_all(dst)?;
    }
    for entry in std::fs::read_dir(src)? {
        let entry = entry?;
        let name = entry.file_name();
        if matches!(name.to_str(), Some(".git" | ".DS_Store")) {
            continue;
        }
        let src_path = entry.path();
        let dst_path = dst.join(&name);
        if src_path.is_dir() {
            copy_dir_recursive(&src_path, &dst_path)?;
        } else {
            std::fs::copy(&src_path, &dst_path)?;
        }
    }
    Ok(())
}
