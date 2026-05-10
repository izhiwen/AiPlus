use crate::redaction::reject_sensitive_memory_text;
use anyhow::{anyhow, Context, Result};
use serde::{Deserialize, Serialize};
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::thread;
use std::time::Duration;

pub const MEMORY_SCHEMA_VERSION_V1: &str = "0.1.0";
pub const MEMORY_SCHEMA_VERSION_V2: &str = "0.2.0";

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase", default)]
pub struct MemoryRecord {
    pub schema_version: String,
    pub id: String,
    #[serde(rename = "type", alias = "kind")]
    pub record_type: String,
    pub scope: String,
    pub source: String,
    pub created_at: String,
    pub updated_at: String,
    pub confidence: String,
    pub status: String,
    #[serde(alias = "content")]
    pub summary: String,
    pub evidence: Vec<String>,
    pub tags: Vec<String>,
    pub expires_at: Option<String>,
    #[serde(alias = "reviewAfter")]
    pub stale_after: Option<String>,
    pub supersedes: Vec<String>,
    pub superseded_by: Vec<String>,
    pub conflict_group: Option<String>,
    pub redaction: String,
    // Backward compatibility fields for v0.1.0
    pub subject: Option<String>,
    pub visibility: Option<String>,
    #[serde(alias = "hash")]
    pub content_hash: Option<String>,
}

impl Default for MemoryRecord {
    fn default() -> Self {
        Self {
            schema_version: MEMORY_SCHEMA_VERSION_V2.to_string(),
            id: String::new(),
            record_type: String::new(),
            scope: String::new(),
            source: String::new(),
            created_at: String::new(),
            updated_at: String::new(),
            confidence: String::new(),
            status: String::new(),
            summary: String::new(),
            evidence: Vec::new(),
            tags: Vec::new(),
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
}

impl MemoryRecord {
    pub fn is_expired(&self) -> bool {
        if let Some(ref expires) = self.expires_at {
            if let Ok(expires_millis) = expires.parse::<u128>() {
                let now = epoch_millis();
                return now > expires_millis;
            }
        }
        false
    }

    pub fn is_stale(&self) -> bool {
        if self.confidence == "stale" {
            return true;
        }
        if let Some(ref stale) = self.stale_after {
            if let Ok(stale_millis) = stale.parse::<u128>() {
                let now = epoch_millis();
                return now > stale_millis;
            }
        }
        self.is_expired()
    }
}

#[derive(Debug, Clone, Default)]
pub struct MemoryStore {
    pub root: PathBuf,
}

impl MemoryStore {
    pub fn new(root: &Path) -> Self {
        Self {
            root: root.to_path_buf(),
        }
    }

    pub fn read_all(&self) -> Result<Vec<MemoryRecord>> {
        read_all(&self.root)
    }

    pub fn read_active(&self) -> Result<Vec<MemoryRecord>> {
        read_active(&self.root)
    }

    pub fn read_all_including_rejected(&self) -> Result<Vec<MemoryRecord>> {
        read_all_including_rejected(&self.root)
    }

    pub fn append(&self, record: &MemoryRecord) -> Result<()> {
        append(&self.root, record)
    }

    pub fn rewrite(&self, records: &[MemoryRecord]) -> Result<()> {
        rewrite(&self.root, records)
    }

    pub fn find_by_id(&self, id: &str) -> Result<Option<MemoryRecord>> {
        let records = self.read_active()?;
        Ok(records.into_iter().find(|r| r.id == id))
    }

    pub fn find_by_query(&self, query: &str) -> Result<Vec<MemoryRecord>> {
        let records = self.read_active()?;
        let needle = query.to_ascii_lowercase();
        Ok(records
            .into_iter()
            .filter(|r| {
                r.id.to_ascii_lowercase().contains(&needle)
                    || r.record_type.to_ascii_lowercase().contains(&needle)
                    || r.scope.to_ascii_lowercase().contains(&needle)
                    || r.summary.to_ascii_lowercase().contains(&needle)
                    || r.source.to_ascii_lowercase().contains(&needle)
                    || r.tags
                        .iter()
                        .any(|tag| tag.to_ascii_lowercase().contains(&needle))
            })
            .collect())
    }
}

pub fn read_all(root: &Path) -> Result<Vec<MemoryRecord>> {
    let mut records = Vec::new();
    for rel in [
        ".aiplus/memory/project-memory.jsonl",
        ".aiplus/memory/decisions.jsonl",
        ".aiplus/memory/facts.jsonl",
    ] {
        let path = root.join(rel);
        if !path.exists() {
            continue;
        }
        for line in fs::read_to_string(&path)
            .with_context(|| format!("read memory file {}", path.display()))?
            .lines()
            .filter(|line| !line.trim().is_empty())
        {
            let record: MemoryRecord = serde_json::from_str(line)
                .with_context(|| format!("parse memory record in {}", path.display()))?;
            records.push(record);
        }
    }
    Ok(records)
}

pub fn read_active(root: &Path) -> Result<Vec<MemoryRecord>> {
    let all = read_all(root)?;
    Ok(all
        .into_iter()
        .filter(|r| r.status != "rejected" && r.status != "forgotten")
        .collect())
}

pub fn read_all_including_rejected(root: &Path) -> Result<Vec<MemoryRecord>> {
    read_all(root)
}

pub fn append(root: &Path, record: &MemoryRecord) -> Result<()> {
    reject_sensitive_memory_text(&record.summary)?;
    let path = root.join(".aiplus/memory/project-memory.jsonl");
    let line = serde_json::to_string(record)?;
    append_jsonl_atomic(&path, &line)
}

pub fn rewrite(root: &Path, records: &[MemoryRecord]) -> Result<()> {
    for record in records {
        reject_sensitive_memory_text(&record.summary)?;
    }
    let path = root.join(".aiplus/memory/project-memory.jsonl");
    rewrite_jsonl_atomic(&path, records)
}

pub fn find_by_id(root: &Path, id: &str) -> Result<Option<MemoryRecord>> {
    let records = read_active(root)?;
    Ok(records.into_iter().find(|r| r.id == id))
}

pub fn find_by_query(root: &Path, query: &str) -> Result<Vec<MemoryRecord>> {
    let records = read_active(root)?;
    let needle = query.to_ascii_lowercase();
    Ok(records
        .into_iter()
        .filter(|r| {
            r.id.to_ascii_lowercase().contains(&needle)
                || r.record_type.to_ascii_lowercase().contains(&needle)
                || r.scope.to_ascii_lowercase().contains(&needle)
                || r.summary.to_ascii_lowercase().contains(&needle)
                || r.source.to_ascii_lowercase().contains(&needle)
                || r.tags
                    .iter()
                    .any(|tag| tag.to_ascii_lowercase().contains(&needle))
        })
        .collect())
}

pub fn append_jsonl_atomic(path: &Path, line: &str) -> Result<()> {
    let _lock = FileLock::acquire(&path.with_extension("lock"))?;
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let mut current = fs::read_to_string(path).unwrap_or_default();
    if !current.is_empty() && !current.ends_with('\n') {
        current.push('\n');
    }
    current.push_str(line);
    current.push('\n');
    write_file_atomic(path, current.as_bytes())
}

pub fn rewrite_jsonl_atomic<T: Serialize>(path: &Path, rows: &[T]) -> Result<()> {
    let _lock = FileLock::acquire(&path.with_extension("lock"))?;
    let mut body = String::new();
    for row in rows {
        body.push_str(&serde_json::to_string(row)?);
        body.push('\n');
    }
    write_file_atomic(path, body.as_bytes())
}

pub struct FileLock {
    path: PathBuf,
}

impl FileLock {
    pub fn acquire(path: &Path) -> Result<Self> {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        for _ in 0..200 {
            match fs::create_dir(path) {
                Ok(()) => {
                    return Ok(Self {
                        path: path.to_path_buf(),
                    });
                }
                Err(error) if error.kind() == io::ErrorKind::AlreadyExists => {
                    thread::sleep(Duration::from_millis(10));
                }
                Err(error) => return Err(error.into()),
            }
        }
        Err(anyhow!("lock timeout: {}", path.display()))
    }
}

impl Drop for FileLock {
    fn drop(&mut self) {
        let _ = fs::remove_dir(&self.path);
    }
}

pub fn write_file_atomic(path: &Path, bytes: &[u8]) -> Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let temp = path.with_extension(format!("tmp-{}", epoch_millis()));
    fs::write(&temp, bytes)?;
    fs::rename(temp, path)?;
    Ok(())
}

pub fn epoch_millis() -> u128 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|duration| duration.as_millis())
        .unwrap_or(0)
}

pub fn single_line(value: &str) -> String {
    value.split_whitespace().collect::<Vec<_>>().join(" ")
}

pub fn stable_hash(value: &str) -> String {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};
    let mut hasher = DefaultHasher::new();
    value.hash(&mut hasher);
    format!("{:x}", hasher.finish())
}

pub fn slugify(value: &str) -> String {
    let mut slug = value
        .chars()
        .map(|ch| {
            if ch.is_ascii_alphanumeric() {
                ch.to_ascii_lowercase()
            } else {
                '-'
            }
        })
        .collect::<String>();
    while slug.contains("--") {
        slug = slug.replace("--", "-");
    }
    slug.trim_matches('-').chars().take(64).collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn sample_record(id: &str, record_type: &str, summary: &str) -> MemoryRecord {
        MemoryRecord {
            id: id.to_string(),
            record_type: record_type.to_string(),
            summary: summary.to_string(),
            status: "active".to_string(),
            ..MemoryRecord::default()
        }
    }

    #[test]
    fn memory_record_default() {
        let r = MemoryRecord::default();
        assert_eq!(r.schema_version, MEMORY_SCHEMA_VERSION_V2);
        assert_eq!(r.redaction, "none");
    }

    #[test]
    fn memory_record_backward_compat_v1() {
        let json = r#"{"schemaVersion":"0.1.0","id":"mem_1","kind":"preference","scope":"project","subject":"workflow","content":"test content","source":"manual","confidence":"owner_asserted","visibility":"project-local","status":"active","createdAt":"1234567890.000","updatedAt":"1234567890.000","expiresAt":null,"reviewAfter":null,"redaction":"none","hash":"hash:abc"}"#;
        let record: MemoryRecord = serde_json::from_str(json).unwrap();
        assert_eq!(record.schema_version, "0.1.0");
        assert_eq!(record.record_type, "preference");
        assert_eq!(record.summary, "test content");
        assert_eq!(record.stale_after, None);
        assert_eq!(record.content_hash, Some("hash:abc".to_string()));
    }

    #[test]
    fn memory_store_read_write() {
        let tmp = TempDir::new().unwrap();
        let store = MemoryStore::new(tmp.path());

        let record = sample_record("mem_1", "project_fact", "Test fact");
        store.append(&record).unwrap();

        let records = store.read_all().unwrap();
        assert_eq!(records.len(), 1);
        assert_eq!(records[0].id, "mem_1");
    }

    #[test]
    fn memory_store_find_by_id() {
        let tmp = TempDir::new().unwrap();
        let store = MemoryStore::new(tmp.path());

        let r1 = sample_record("mem_1", "project_fact", "Fact one");
        let r2 = sample_record("mem_2", "project_decision", "Decision two");
        store.append(&r1).unwrap();
        store.append(&r2).unwrap();

        assert_eq!(
            store.find_by_id("mem_1").unwrap().unwrap().summary,
            "Fact one"
        );
        assert!(store.find_by_id("mem_999").unwrap().is_none());
    }

    #[test]
    fn memory_store_find_by_query() {
        let tmp = TempDir::new().unwrap();
        let store = MemoryStore::new(tmp.path());

        let r1 = sample_record("mem_1", "project_fact", "Fact one");
        let mut r2 = sample_record("mem_2", "project_decision", "Decision two");
        r2.tags = vec!["urgent".to_string()];
        store.append(&r1).unwrap();
        store.append(&r2).unwrap();

        assert_eq!(store.find_by_query("decision").unwrap().len(), 1);
        assert_eq!(store.find_by_query("urgent").unwrap().len(), 1);
        assert_eq!(store.find_by_query("mem_").unwrap().len(), 2);
    }

    #[test]
    fn memory_store_rewrite() {
        let tmp = TempDir::new().unwrap();
        let store = MemoryStore::new(tmp.path());

        let r1 = sample_record("mem_1", "project_fact", "Fact one");
        store.append(&r1).unwrap();

        let mut r2 = sample_record("mem_1", "project_fact", "Updated fact");
        r2.status = "rejected".to_string();
        store.rewrite(&[r2]).unwrap();

        let records = store.read_all().unwrap();
        assert_eq!(records.len(), 1);
        assert_eq!(records[0].status, "rejected");
    }

    #[test]
    fn memory_store_read_active_excludes_rejected() {
        let tmp = TempDir::new().unwrap();
        let store = MemoryStore::new(tmp.path());

        let mut r1 = sample_record("mem_1", "project_fact", "Fact one");
        r1.status = "active".to_string();
        let mut r2 = sample_record("mem_2", "project_fact", "Fact two");
        r2.status = "rejected".to_string();
        let mut r3 = sample_record("mem_3", "project_fact", "Fact three");
        r3.status = "forgotten".to_string();
        store.append(&r1).unwrap();
        store.append(&r2).unwrap();
        store.append(&r3).unwrap();

        let active = store.read_active().unwrap();
        assert_eq!(active.len(), 1);
        assert_eq!(active[0].id, "mem_1");

        let all = store.read_all_including_rejected().unwrap();
        assert_eq!(all.len(), 3);
    }

    #[test]
    fn memory_store_find_by_id_excludes_rejected() {
        let tmp = TempDir::new().unwrap();
        let store = MemoryStore::new(tmp.path());

        let mut r1 = sample_record("mem_1", "project_fact", "Fact one");
        r1.status = "rejected".to_string();
        store.append(&r1).unwrap();

        assert!(store.find_by_id("mem_1").unwrap().is_none());
        assert!(store
            .read_all_including_rejected()
            .unwrap()
            .iter()
            .any(|r| r.id == "mem_1"));
    }

    #[test]
    fn memory_store_find_by_query_excludes_rejected() {
        let tmp = TempDir::new().unwrap();
        let store = MemoryStore::new(tmp.path());

        let mut r1 = sample_record("mem_1", "project_fact", "Alpha fact");
        r1.status = "active".to_string();
        let mut r2 = sample_record("mem_2", "project_fact", "Alpha fact");
        r2.status = "rejected".to_string();
        store.append(&r1).unwrap();
        store.append(&r2).unwrap();

        assert_eq!(store.find_by_query("alpha").unwrap().len(), 1);
        assert_eq!(store.find_by_query("alpha").unwrap()[0].id, "mem_1");
    }

    #[test]
    fn record_expiry() {
        let past = (epoch_millis() - 1000).to_string();
        let future = (epoch_millis() + 10000).to_string();

        let expired = MemoryRecord {
            expires_at: Some(past),
            ..MemoryRecord::default()
        };
        assert!(expired.is_expired());

        let fresh = MemoryRecord {
            expires_at: Some(future),
            ..MemoryRecord::default()
        };
        assert!(!fresh.is_expired());
    }

    #[test]
    fn record_staleness() {
        let stale = MemoryRecord {
            confidence: "stale".to_string(),
            ..MemoryRecord::default()
        };
        assert!(stale.is_stale());

        let past = (epoch_millis() - 1000).to_string();
        let expired = MemoryRecord {
            stale_after: Some(past),
            ..MemoryRecord::default()
        };
        assert!(expired.is_stale());
    }

    #[test]
    fn stable_hash_deterministic() {
        let a = stable_hash("hello");
        let b = stable_hash("hello");
        let c = stable_hash("world");
        assert_eq!(a, b);
        assert_ne!(a, c);
    }

    #[test]
    fn slugify_basic() {
        assert_eq!(slugify("Hello World"), "hello-world");
        assert_eq!(slugify("Test--Value"), "test-value");
        assert_eq!(slugify("-leading-"), "leading");
    }
}
