use crate::capsule::timestamp;
use crate::memory::{epoch_millis, MemoryRecord, MemoryStore};
use crate::redaction::reject_sensitive_memory_text;
use crate::skill_candidate::read_all as read_skill_candidates;
use anyhow::Result;
use std::fs;
use std::path::{Path, PathBuf};

pub const SNAPSHOT_SCHEMA_VERSION: &str = "0.2.0";

#[derive(Debug, Clone)]
pub struct SnapshotBuilder {
    root: PathBuf,
}

impl SnapshotBuilder {
    pub fn new(root: &Path) -> Self {
        Self {
            root: root.to_path_buf(),
        }
    }

    pub fn build_project_snapshot(&self) -> Result<String> {
        let memory_store = MemoryStore::new(&self.root);
        let records = memory_store.read_all()?;
        let candidates = read_skill_candidates(&self.root)?;

        let mut lines = Vec::new();
        lines.push("# AiPlus Project Memory Snapshot".to_string());
        lines.push(String::new());
        lines.push(format!("Generated: {}", timestamp()));
        lines.push(format!("Schema: {}", SNAPSHOT_SCHEMA_VERSION));
        lines.push(String::new());

        let mut push_section = |title: &str, filter: fn(&MemoryRecord) -> bool| {
            lines.push(format!("## {}", title));
            let section_records: Vec<&MemoryRecord> =
                records.iter().filter(|r| filter(r)).collect();
            if section_records.is_empty() {
                lines.push("_None recorded._".to_string());
            } else {
                for r in section_records {
                    if let Ok(()) = reject_sensitive_memory_text(&r.summary) {
                        lines.push(format!(
                            "- **{}** ({}) — {}",
                            r.id, r.record_type, r.summary
                        ));
                    } else {
                        lines.push(format!("- **{}** ({}) — [REDACTED]", r.id, r.record_type));
                    }
                }
            }
            lines.push(String::new());
        };

        push_section("Project Facts", |r| r.record_type == "project_fact");
        push_section("Project Decisions", |r| r.record_type == "project_decision");
        push_section("Active Risks", |r| r.record_type == "risk");
        push_section("Workflow Rules", |r| r.record_type == "workflow_rule");
        push_section("Owner Preferences", |r| r.record_type == "owner_preference");
        push_section("Handoff Notes", |r| r.record_type == "handoff_note");

        lines.push("## Skill Candidates".to_string());
        if candidates.is_empty() {
            lines.push("_None recorded._".to_string());
        } else {
            for c in candidates {
                lines.push(format!("- **{}** ({}) — {}", c.id, c.status, c.title));
            }
        }
        lines.push(String::new());

        lines.push("## Recent Commands".to_string());
        lines.push("_See session index for recent commands._".to_string());
        lines.push(String::new());

        Ok(lines.join("\n"))
    }

    pub fn build_profile_snapshot(&self, profile_root: &Path) -> Result<String> {
        let memory_store = MemoryStore::new(profile_root);
        let records = memory_store.read_all()?;

        let mut lines = Vec::new();
        lines.push("# AiPlus User Profile Snapshot".to_string());
        lines.push(String::new());
        lines.push(format!("Generated: {}", timestamp()));
        lines.push(format!("Schema: {}", SNAPSHOT_SCHEMA_VERSION));
        lines.push(String::new());

        let mut push_section = |title: &str, filter: fn(&MemoryRecord) -> bool| {
            lines.push(format!("## {}", title));
            let section_records: Vec<&MemoryRecord> =
                records.iter().filter(|r| filter(r)).collect();
            if section_records.is_empty() {
                lines.push("_None recorded._".to_string());
            } else {
                for r in section_records {
                    if let Ok(()) = reject_sensitive_memory_text(&r.summary) {
                        lines.push(format!("- **{}** — {}", r.id, r.summary));
                    } else {
                        lines.push(format!("- **{}** — [REDACTED]", r.id));
                    }
                }
            }
            lines.push(String::new());
        };

        push_section("Communication Preferences", |r| {
            r.record_type == "owner_preference" && r.scope == "profile"
        });
        push_section("Default CEO/Advisor Style", |r| {
            r.record_type == "role_identity" && r.scope == "profile"
        });
        push_section("Cross-Project Workflow Preferences", |r| {
            r.record_type == "workflow_rule" && r.scope == "profile"
        });
        push_section("Risk Posture", |r| {
            r.record_type == "risk" && r.scope == "profile"
        });
        push_section("Common Owner Gates", |r| r.record_type == "owner_gate");
        push_section("Model/Tool Preferences", |r| {
            r.record_type == "preference" && !r.summary.to_ascii_lowercase().contains("secret")
        });

        Ok(lines.join("\n"))
    }

    pub fn write_project_snapshot(&self) -> Result<PathBuf> {
        let snapshot = self.build_project_snapshot()?;
        let path = self.root.join(".aiplus/memory/MEMORY.md");
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        let temp = path.with_extension(format!("tmp-{}", epoch_millis()));
        fs::write(&temp, snapshot)?;
        fs::rename(&temp, &path)?;
        Ok(path)
    }

    pub fn write_profile_snapshot(
        &self,
        profile_root: &Path,
        profile_name: &str,
    ) -> Result<PathBuf> {
        let snapshot = self.build_profile_snapshot(profile_root)?;
        let path = profile_root
            .join(profile_name)
            .join("profile-memory/USER.md");
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        let temp = path.with_extension(format!("tmp-{}", epoch_millis()));
        fs::write(&temp, snapshot)?;
        fs::rename(&temp, &path)?;
        Ok(path)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::memory::append_jsonl_atomic;
    use tempfile::TempDir;

    fn sample_memory(id: &str, record_type: &str, scope: &str, summary: &str) -> MemoryRecord {
        MemoryRecord {
            id: id.to_string(),
            record_type: record_type.to_string(),
            scope: scope.to_string(),
            summary: summary.to_string(),
            status: "active".to_string(),
            ..MemoryRecord::default()
        }
    }

    #[test]
    fn build_project_snapshot_sections() {
        let tmp = TempDir::new().unwrap();
        let builder = SnapshotBuilder::new(tmp.path());

        let path = tmp.path().join(".aiplus/memory/project-memory.jsonl");
        let records = vec![
            sample_memory("mem_1", "project_fact", "project", "Rust crate layout"),
            sample_memory(
                "mem_2",
                "project_decision",
                "project",
                "Use SQLite for search",
            ),
            sample_memory("mem_3", "risk", "project", "Schema migration complexity"),
            sample_memory(
                "mem_4",
                "workflow_rule",
                "project",
                "Run tests before merge",
            ),
            sample_memory("mem_5", "owner_preference", "project", "Concise output"),
            sample_memory("mem_6", "handoff_note", "project", "Continue on session.rs"),
        ];
        for r in &records {
            let line = serde_json::to_string(r).unwrap();
            append_jsonl_atomic(&path, &line).unwrap();
        }

        let snapshot = builder.build_project_snapshot().unwrap();
        assert!(snapshot.contains("# AiPlus Project Memory Snapshot"));
        assert!(snapshot.contains("## Project Facts"));
        assert!(snapshot.contains("Rust crate layout"));
        assert!(snapshot.contains("## Project Decisions"));
        assert!(snapshot.contains("Use SQLite for search"));
        assert!(snapshot.contains("## Active Risks"));
        assert!(snapshot.contains("## Workflow Rules"));
        assert!(snapshot.contains("## Owner Preferences"));
        assert!(snapshot.contains("## Handoff Notes"));
        assert!(snapshot.contains("## Skill Candidates"));
    }

    #[test]
    fn build_profile_snapshot_sections() {
        let tmp = TempDir::new().unwrap();
        let builder = SnapshotBuilder::new(tmp.path());

        let path = tmp.path().join(".aiplus/memory/project-memory.jsonl");

        let records = vec![
            sample_memory("mem_1", "owner_preference", "profile", "Prefer async"),
            sample_memory("mem_2", "workflow_rule", "profile", "Tag releases"),
            sample_memory(
                "mem_3",
                "owner_gate",
                "profile",
                "Require review for deploy",
            ),
            sample_memory("mem_4", "preference", "profile", "Use Claude 4"),
        ];
        for r in &records {
            let line = serde_json::to_string(r).unwrap();
            append_jsonl_atomic(&path, &line).unwrap();
        }

        let snapshot = builder.build_profile_snapshot(tmp.path()).unwrap();
        assert!(snapshot.contains("# AiPlus User Profile Snapshot"));
        assert!(snapshot.contains("## Communication Preferences"));
        assert!(snapshot.contains("## Default CEO/Advisor Style"));
        assert!(snapshot.contains("## Cross-Project Workflow Preferences"));
        assert!(snapshot.contains("## Risk Posture"));
        assert!(snapshot.contains("## Common Owner Gates"));
        assert!(snapshot.contains("## Model/Tool Preferences"));
        assert!(snapshot.contains("Prefer async"));
        assert!(snapshot.contains("Tag releases"));
    }

    #[test]
    fn build_project_snapshot_redacts_secrets() {
        let tmp = TempDir::new().unwrap();
        let builder = SnapshotBuilder::new(tmp.path());

        let path = tmp.path().join(".aiplus/memory/project-memory.jsonl");
        let mut r = sample_memory("mem_1", "project_fact", "project", "api_key=secret123");
        r.summary = "api_key=secret123".to_string();
        let line = serde_json::to_string(&r).unwrap();
        append_jsonl_atomic(&path, &line).unwrap();

        let snapshot = builder.build_project_snapshot().unwrap();
        assert!(snapshot.contains("[REDACTED]"));
        assert!(!snapshot.contains("api_key=secret123"));
    }

    #[test]
    fn write_profile_snapshot_creates_file() {
        let tmp = TempDir::new().unwrap();
        let builder = SnapshotBuilder::new(tmp.path());

        let profile = tmp.path().join("test-profile/profile-memory");
        fs::create_dir_all(&profile).unwrap();

        let path = builder
            .write_profile_snapshot(tmp.path(), "test-profile")
            .unwrap();
        assert_eq!(path, tmp.path().join("test-profile/profile-memory/USER.md"));
        assert!(path.exists());
    }
}
