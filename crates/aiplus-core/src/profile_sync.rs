use crate::memory::{MemoryRecord, MemoryStore};
use crate::redaction::reject_sensitive_memory_text;
use anyhow::{anyhow, Context, Result};
use std::path::{Path, PathBuf};

pub struct ProfileSync {
    profile_root: PathBuf,
    profile_name: String,
}

#[derive(Debug, Clone)]
pub struct SyncResult {
    pub profile_records_read: usize,
    pub project_records_updated: usize,
    pub conflicts: Vec<String>,
    pub status: String,
}

impl ProfileSync {
    pub fn new(profile_root: &Path, profile_name: &str) -> Self {
        Self {
            profile_root: profile_root.to_path_buf(),
            profile_name: profile_name.to_string(),
        }
    }

    fn profile_memory_dir(&self) -> PathBuf {
        self.profile_root
            .join(&self.profile_name)
            .join("profile-memory")
    }

    fn profile_memory_path(&self) -> PathBuf {
        self.profile_memory_dir().join("memories.jsonl")
    }

    pub fn read_profile_memories(&self) -> Result<Vec<MemoryRecord>> {
        let path = self.profile_memory_path();
        if !path.exists() {
            return Ok(Vec::new());
        }
        let mut records = Vec::new();
        for line in std::fs::read_to_string(&path)
            .with_context(|| format!("read profile memory file {}", path.display()))?
            .lines()
            .filter(|line| !line.trim().is_empty())
        {
            let record: MemoryRecord = serde_json::from_str(line)
                .with_context(|| format!("parse profile memory record in {}", path.display()))?;
            records.push(record);
        }
        Ok(records)
    }

    pub fn write_profile_memory(&self, record: &MemoryRecord) -> Result<()> {
        reject_sensitive_memory_text(&record.summary)?;
        let dir = self.profile_memory_dir();
        std::fs::create_dir_all(&dir)
            .with_context(|| format!("create profile memory dir {}", dir.display()))?;
        let path = self.profile_memory_path();
        let line = serde_json::to_string(record)?;
        crate::memory::append_jsonl_atomic(&path, &line)
    }

    pub fn sync_to_project(&self, project_root: &Path) -> Result<SyncResult> {
        let profile_records = self.read_profile_memories()?;
        let project_store = MemoryStore::new(project_root);
        let existing_project_records = project_store.read_all().unwrap_or_default();

        let mut project_records_updated = 0;
        let mut conflicts = Vec::new();

        for profile_record in &profile_records {
            if reject_sensitive_memory_text(&profile_record.summary).is_err() {
                conflicts.push(format!(
                    "BLOCKED secret in profile record {}",
                    profile_record.id
                ));
                continue;
            }

            let is_preference = profile_record
                .record_type
                .to_ascii_lowercase()
                .contains("preference")
                || profile_record
                    .tags
                    .iter()
                    .any(|t| t.to_ascii_lowercase().contains("preference"));

            if !is_preference {
                continue;
            }

            let already_exists = existing_project_records.iter().any(|r| {
                r.id == profile_record.id
                    || (r.summary == profile_record.summary
                        && r.record_type == profile_record.record_type)
            });

            if already_exists {
                continue;
            }

            let mut synced = profile_record.clone();
            synced.source = "profile_sync".to_string();
            synced.tags.push("profile_sync".to_string());

            if let Err(e) = project_store.append(&synced) {
                conflicts.push(format!(
                    "Failed to sync profile record {}: {}",
                    profile_record.id, e
                ));
            } else {
                project_records_updated += 1;
            }
        }

        let status = if conflicts.is_empty() {
            "success".to_string()
        } else {
            format!(
                "partial ({}/{} updated)",
                project_records_updated,
                profile_records.len()
            )
        };

        Ok(SyncResult {
            profile_records_read: profile_records.len(),
            project_records_updated,
            conflicts,
            status,
        })
    }

    pub fn is_global_preference_trigger(text: &str) -> bool {
        let lower = text.to_ascii_lowercase();
        let triggers = [
            "以后所有项目都",
            "以后都",
            "我的通用偏好是",
            "所有 ceo 都应该",
            "所有 advisor 都应该",
            "记到我的全局偏好里",
            "all projects should",
            "my global preference is",
            "always remember",
        ];
        triggers
            .iter()
            .any(|t| lower.contains(&t.to_ascii_lowercase()))
    }

    pub fn promote_to_profile(&self, record: &MemoryRecord) -> Result<String> {
        if reject_sensitive_memory_text(&record.summary).is_err() {
            return Err(anyhow!(
                "Cannot promote record {}: contains sensitive content",
                record.id
            ));
        }

        let is_preference = record
            .record_type
            .to_ascii_lowercase()
            .contains("preference")
            || record
                .tags
                .iter()
                .any(|t| t.to_ascii_lowercase().contains("preference"));

        let is_global = record
            .tags
            .iter()
            .any(|t| t.to_ascii_lowercase().contains("global"))
            || Self::is_global_preference_trigger(&record.summary);

        let is_project_fact = record
            .record_type
            .to_ascii_lowercase()
            .contains("project_fact")
            || record
                .tags
                .iter()
                .any(|t| t.to_ascii_lowercase().contains("project_fact"));

        if is_project_fact && !is_global {
            return Err(anyhow!(
                "Cannot promote record {}: project facts stay project-local",
                record.id
            ));
        }

        if !is_preference && !is_global {
            return Err(anyhow!(
                "Cannot promote record {}: not a preference or global trigger",
                record.id
            ));
        }

        let mut promoted = record.clone();
        promoted.source = "profile_sync_promoted".to_string();
        promoted.tags.push("profile_candidate".to_string());
        promoted
            .tags
            .retain(|t| !t.to_ascii_lowercase().contains("project-local"));

        self.write_profile_memory(&promoted)?;
        Ok(promoted.id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn sample_record(id: &str, record_type: &str, summary: &str) -> MemoryRecord {
        MemoryRecord {
            schema_version: crate::memory::MEMORY_SCHEMA_VERSION_V2.to_string(),
            id: id.to_string(),
            record_type: record_type.to_string(),
            scope: "project".to_string(),
            source: "manual".to_string(),
            created_at: "0".to_string(),
            updated_at: "0".to_string(),
            confidence: "owner_asserted".to_string(),
            status: "active".to_string(),
            summary: summary.to_string(),
            evidence: vec![],
            tags: vec![],
            expires_at: None,
            stale_after: None,
            supersedes: Vec::new(),
            superseded_by: Vec::new(),
            conflict_group: None,
            redaction: "none".to_string(),
            subject: None,
            visibility: None,
            content_hash: None,
        }
    }

    #[test]
    fn read_profile_memories_empty() {
        let tmp = TempDir::new().unwrap();
        let sync = ProfileSync::new(tmp.path(), "test-profile");
        let records = sync.read_profile_memories().unwrap();
        assert!(records.is_empty());
    }

    #[test]
    fn write_and_read_profile_memory() {
        let tmp = TempDir::new().unwrap();
        let sync = ProfileSync::new(tmp.path(), "test-profile");

        let record = sample_record("pref_1", "owner_preference", "Use 4 spaces");
        sync.write_profile_memory(&record).unwrap();

        let records = sync.read_profile_memories().unwrap();
        assert_eq!(records.len(), 1);
        assert_eq!(records[0].id, "pref_1");
    }

    #[test]
    fn is_global_preference_trigger_chinese() {
        assert!(ProfileSync::is_global_preference_trigger(
            "以后所有项目都使用 Rust"
        ));
        assert!(ProfileSync::is_global_preference_trigger(
            "我的通用偏好是深色模式"
        ));
        assert!(ProfileSync::is_global_preference_trigger(
            "所有 CEO 都应该先审阅计划"
        ));
        assert!(ProfileSync::is_global_preference_trigger(
            "记到我的全局偏好里"
        ));
    }

    #[test]
    fn is_global_preference_trigger_english() {
        assert!(ProfileSync::is_global_preference_trigger(
            "all projects should use cargo"
        ));
        assert!(ProfileSync::is_global_preference_trigger(
            "my global preference is 2 spaces"
        ));
        assert!(ProfileSync::is_global_preference_trigger(
            "always remember to test first"
        ));
    }

    #[test]
    fn is_global_preference_trigger_negative() {
        assert!(!ProfileSync::is_global_preference_trigger(
            "this project uses Rust"
        ));
        assert!(!ProfileSync::is_global_preference_trigger("local setting"));
    }

    #[test]
    fn promote_to_profile_success() {
        let tmp = TempDir::new().unwrap();
        let sync = ProfileSync::new(tmp.path(), "test-profile");

        let mut record = sample_record("pref_1", "owner_preference", "Use dark mode");
        record.tags.push("preference".to_string());

        let id = sync.promote_to_profile(&record).unwrap();
        assert_eq!(id, "pref_1");

        let profile_records = sync.read_profile_memories().unwrap();
        assert_eq!(profile_records.len(), 1);
        assert!(profile_records[0]
            .tags
            .contains(&"profile_candidate".to_string()));
    }

    #[test]
    fn promote_to_profile_global_trigger() {
        let tmp = TempDir::new().unwrap();
        let sync = ProfileSync::new(tmp.path(), "test-profile");

        let record = sample_record("pref_2", "note", "all projects should use Rust");

        let id = sync.promote_to_profile(&record).unwrap();
        assert_eq!(id, "pref_2");

        let profile_records = sync.read_profile_memories().unwrap();
        assert_eq!(profile_records.len(), 1);
    }

    #[test]
    fn promote_to_profile_blocks_project_fact() {
        let tmp = TempDir::new().unwrap();
        let sync = ProfileSync::new(tmp.path(), "test-profile");

        let record = sample_record("fact_1", "project_fact", "src/main.rs is entry point");

        let result = sync.promote_to_profile(&record);
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("project facts stay project-local"));
    }

    #[test]
    fn promote_to_profile_blocks_secret() {
        let tmp = TempDir::new().unwrap();
        let sync = ProfileSync::new(tmp.path(), "test-profile");

        let record = sample_record("sec_1", "owner_preference", "api_key=secret123");

        let result = sync.promote_to_profile(&record);
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("sensitive content"));
    }

    #[test]
    fn sync_to_project_success() {
        let profile_tmp = TempDir::new().unwrap();
        let project_tmp = TempDir::new().unwrap();
        let sync = ProfileSync::new(profile_tmp.path(), "test-profile");

        let mut record = sample_record("pref_1", "owner_preference", "Use 4 spaces");
        record.tags.push("preference".to_string());
        sync.write_profile_memory(&record).unwrap();

        let result = sync.sync_to_project(project_tmp.path()).unwrap();
        assert_eq!(result.profile_records_read, 1);
        assert_eq!(result.project_records_updated, 1);
        assert!(result.conflicts.is_empty());
        assert_eq!(result.status, "success");

        let project_store = MemoryStore::new(project_tmp.path());
        let project_records = project_store.read_all().unwrap();
        assert_eq!(project_records.len(), 1);
        assert_eq!(project_records[0].summary, "Use 4 spaces");
        assert!(project_records[0]
            .tags
            .contains(&"profile_sync".to_string()));
    }

    #[test]
    fn sync_to_project_skips_non_preference() {
        let profile_tmp = TempDir::new().unwrap();
        let project_tmp = TempDir::new().unwrap();
        let sync = ProfileSync::new(profile_tmp.path(), "test-profile");

        let record = sample_record("fact_1", "project_fact", "src/main.rs is entry");
        sync.write_profile_memory(&record).unwrap();

        let result = sync.sync_to_project(project_tmp.path()).unwrap();
        assert_eq!(result.profile_records_read, 1);
        assert_eq!(result.project_records_updated, 0);
    }

    #[test]
    fn sync_to_project_blocks_secret() {
        let profile_tmp = TempDir::new().unwrap();
        let project_tmp = TempDir::new().unwrap();
        let sync = ProfileSync::new(profile_tmp.path(), "test-profile");

        let mut record = sample_record("pref_1", "owner_preference", "Use dark mode");
        record.tags.push("preference".to_string());
        sync.write_profile_memory(&record).unwrap();

        let mut secret_record = sample_record("sec_1", "owner_preference", "api_key=secret");
        secret_record.tags.push("preference".to_string());
        // Secret is now blocked at write time by reject_sensitive_memory_text
        assert!(sync.write_profile_memory(&secret_record).is_err());

        let result = sync.sync_to_project(project_tmp.path()).unwrap();
        assert_eq!(result.profile_records_read, 1);
        assert_eq!(result.project_records_updated, 1);
        assert_eq!(result.conflicts.len(), 0);
    }

    #[test]
    fn sync_to_project_does_not_duplicate() {
        let profile_tmp = TempDir::new().unwrap();
        let project_tmp = TempDir::new().unwrap();
        let sync = ProfileSync::new(profile_tmp.path(), "test-profile");

        let mut record = sample_record("pref_1", "owner_preference", "Use 4 spaces");
        record.tags.push("preference".to_string());
        sync.write_profile_memory(&record).unwrap();

        sync.sync_to_project(project_tmp.path()).unwrap();
        let result = sync.sync_to_project(project_tmp.path()).unwrap();
        assert_eq!(result.project_records_updated, 0);
    }

    #[test]
    fn promote_to_profile_removes_project_local_tag() {
        let tmp = TempDir::new().unwrap();
        let sync = ProfileSync::new(tmp.path(), "test-profile");

        let mut record = sample_record("pref_1", "owner_preference", "Use dark mode");
        record.tags.push("preference".to_string());
        record.tags.push("project-local".to_string());

        sync.promote_to_profile(&record).unwrap();

        let profile_records = sync.read_profile_memories().unwrap();
        assert!(!profile_records[0].tags.iter().any(|t| t == "project-local"));
    }

    #[test]
    fn sync_to_project_empty_profile() {
        let profile_tmp = TempDir::new().unwrap();
        let project_tmp = TempDir::new().unwrap();
        let sync = ProfileSync::new(profile_tmp.path(), "test-profile");

        let result = sync.sync_to_project(project_tmp.path()).unwrap();
        assert_eq!(result.profile_records_read, 0);
        assert_eq!(result.project_records_updated, 0);
        assert_eq!(result.status, "success");
    }
}
