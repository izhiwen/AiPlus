use anyhow::{Context, Result};
use rusqlite::{params, Connection};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

pub const SESSION_SCHEMA_VERSION: &str = "0.2.0";

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
#[serde(rename_all = "camelCase", default)]
pub struct SessionRecord {
    pub id: String,
    pub project_id: String,
    pub role: String,
    pub created_at: String,
    pub updated_at: String,
    pub summary: String,
    pub decisions: Vec<String>,
    pub files_changed: Vec<String>,
    pub commands_run: Vec<String>,
    pub tests_run: Vec<String>,
    pub findings_fixed: Vec<String>,
    pub blockers: Vec<String>,
    pub next_action: String,
    pub memory_ids_used: Vec<String>,
    pub skill_candidates_proposed: Vec<String>,
    pub compact_checkpoint_link: Option<String>,
    pub no_secret_marker: bool,
}

#[derive(Debug, Clone)]
pub struct SessionIndex {
    db_path: PathBuf,
}

impl SessionIndex {
    pub fn new(root: &Path) -> Result<Self> {
        let db_path = root.join(".aiplus/memory/sessions.sqlite");
        Ok(Self { db_path })
    }

    pub fn init(&self) -> Result<()> {
        let parent = self.db_path.parent().with_context(|| {
            format!("sessions db path has no parent: {}", self.db_path.display())
        })?;
        std::fs::create_dir_all(parent)?;
        let conn = Connection::open(&self.db_path)
            .with_context(|| format!("open sessions db: {}", self.db_path.display()))?;
        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS sessions (
                id TEXT PRIMARY KEY,
                project_id TEXT NOT NULL,
                role TEXT NOT NULL,
                created_at TEXT NOT NULL,
                updated_at TEXT NOT NULL,
                summary TEXT NOT NULL,
                decisions TEXT NOT NULL DEFAULT '[]',
                files_changed TEXT NOT NULL DEFAULT '[]',
                commands_run TEXT NOT NULL DEFAULT '[]',
                tests_run TEXT NOT NULL DEFAULT '[]',
                findings_fixed TEXT NOT NULL DEFAULT '[]',
                blockers TEXT NOT NULL DEFAULT '[]',
                next_action TEXT NOT NULL,
                memory_ids_used TEXT NOT NULL DEFAULT '[]',
                skill_candidates_proposed TEXT NOT NULL DEFAULT '[]',
                compact_checkpoint_link TEXT,
                no_secret_marker INTEGER NOT NULL DEFAULT 0
            );
            CREATE TABLE IF NOT EXISTS session_meta (
                key TEXT PRIMARY KEY,
                value TEXT NOT NULL
            );
            INSERT OR IGNORE INTO session_meta (key, value) VALUES ('schema_version', '0.2.0');
            CREATE VIRTUAL TABLE IF NOT EXISTS sessions_fts USING fts5(
                session_id UNINDEXED,
                summary,
                decisions,
                next_action
            );",
        )
        .with_context(|| "create sessions schema")?;
        Ok(())
    }

    pub fn add_session(&self, session: &SessionRecord) -> Result<()> {
        if !session.no_secret_marker {
            return Err(anyhow::anyhow!(
                "SessionRecord must have no_secret_marker=true"
            ));
        }
        let conn = Connection::open(&self.db_path)
            .with_context(|| format!("open sessions db: {}", self.db_path.display()))?;
        let decisions = serde_json::to_string(&session.decisions)?;
        let files_changed = serde_json::to_string(&session.files_changed)?;
        let commands_run = serde_json::to_string(&session.commands_run)?;
        let tests_run = serde_json::to_string(&session.tests_run)?;
        let findings_fixed = serde_json::to_string(&session.findings_fixed)?;
        let blockers = serde_json::to_string(&session.blockers)?;
        let memory_ids_used = serde_json::to_string(&session.memory_ids_used)?;
        let skill_candidates_proposed = serde_json::to_string(&session.skill_candidates_proposed)?;

        conn.execute(
            "INSERT OR REPLACE INTO sessions (
                id, project_id, role, created_at, updated_at,
                summary, decisions, files_changed, commands_run, tests_run,
                findings_fixed, blockers, next_action, memory_ids_used,
                skill_candidates_proposed, compact_checkpoint_link, no_secret_marker
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17)",
            params![
                session.id,
                session.project_id,
                session.role,
                session.created_at,
                session.updated_at,
                session.summary,
                decisions,
                files_changed,
                commands_run,
                tests_run,
                findings_fixed,
                blockers,
                session.next_action,
                memory_ids_used,
                skill_candidates_proposed,
                session.compact_checkpoint_link,
                1i32,
            ],
        )
        .with_context(|| "insert session")?;

        conn.execute(
            "INSERT OR REPLACE INTO sessions_fts (session_id, summary, decisions, next_action)
             VALUES (?1, ?2, ?3, ?4)",
            params![
                session.id,
                session.summary,
                session.decisions.join(" "),
                session.next_action,
            ],
        )
        .with_context(|| "insert session fts")?;

        Ok(())
    }

    pub fn search(&self, query: &str, limit: usize) -> Result<Vec<SessionRecord>> {
        let conn = Connection::open(&self.db_path)
            .with_context(|| format!("open sessions db: {}", self.db_path.display()))?;
        let mut stmt = conn.prepare(
            "SELECT s.id FROM sessions_fts fts
             JOIN sessions s ON s.id = fts.session_id
             WHERE sessions_fts MATCH ?1
             ORDER BY rank
             LIMIT ?2",
        )?;
        let ids: Vec<String> = stmt
            .query_map(params![query, limit as i64], |row| row.get(0))?
            .collect::<Result<Vec<String>, _>>()?;
        drop(stmt);

        let mut results = Vec::new();
        for id in ids {
            if let Some(record) = self.get_session_conn(&conn, &id)? {
                results.push(record);
            }
        }
        Ok(results)
    }

    pub fn list_recent(&self, limit: usize) -> Result<Vec<SessionRecord>> {
        let conn = Connection::open(&self.db_path)
            .with_context(|| format!("open sessions db: {}", self.db_path.display()))?;
        let mut stmt = conn.prepare(
            "SELECT id FROM sessions
             ORDER BY updated_at DESC
             LIMIT ?1",
        )?;
        let ids: Vec<String> = stmt
            .query_map(params![limit as i64], |row| row.get(0))?
            .collect::<Result<Vec<String>, _>>()?;
        drop(stmt);

        let mut results = Vec::new();
        for id in ids {
            if let Some(record) = self.get_session_conn(&conn, &id)? {
                results.push(record);
            }
        }
        Ok(results)
    }

    pub fn get_session(&self, id: &str) -> Result<Option<SessionRecord>> {
        let conn = Connection::open(&self.db_path)
            .with_context(|| format!("open sessions db: {}", self.db_path.display()))?;
        self.get_session_conn(&conn, id)
    }

    fn get_session_conn(&self, conn: &Connection, id: &str) -> Result<Option<SessionRecord>> {
        let mut stmt = conn.prepare(
            "SELECT
                id, project_id, role, created_at, updated_at,
                summary, decisions, files_changed, commands_run, tests_run,
                findings_fixed, blockers, next_action, memory_ids_used,
                skill_candidates_proposed, compact_checkpoint_link, no_secret_marker
             FROM sessions WHERE id = ?1",
        )?;
        let mut rows = stmt.query(params![id])?;
        if let Some(row) = rows.next()? {
            let no_secret_marker: i32 = row.get(16)?;
            Ok(Some(SessionRecord {
                id: row.get(0)?,
                project_id: row.get(1)?,
                role: row.get(2)?,
                created_at: row.get(3)?,
                updated_at: row.get(4)?,
                summary: row.get(5)?,
                decisions: serde_json::from_str(&row.get::<_, String>(6)?)?,
                files_changed: serde_json::from_str(&row.get::<_, String>(7)?)?,
                commands_run: serde_json::from_str(&row.get::<_, String>(8)?)?,
                tests_run: serde_json::from_str(&row.get::<_, String>(9)?)?,
                findings_fixed: serde_json::from_str(&row.get::<_, String>(10)?)?,
                blockers: serde_json::from_str(&row.get::<_, String>(11)?)?,
                next_action: row.get(12)?,
                memory_ids_used: serde_json::from_str(&row.get::<_, String>(13)?)?,
                skill_candidates_proposed: serde_json::from_str(&row.get::<_, String>(14)?)?,
                compact_checkpoint_link: row.get(15)?,
                no_secret_marker: no_secret_marker != 0,
            }))
        } else {
            Ok(None)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn sample_session(id: &str, summary: &str) -> SessionRecord {
        SessionRecord {
            id: id.to_string(),
            project_id: "proj_1".to_string(),
            role: "ceo".to_string(),
            created_at: "1000.000".to_string(),
            updated_at: "1000.000".to_string(),
            summary: summary.to_string(),
            decisions: vec!["decide_a".to_string()],
            files_changed: vec!["src/main.rs".to_string()],
            commands_run: vec!["cargo test".to_string()],
            tests_run: vec!["unit".to_string()],
            findings_fixed: vec!["fix_1".to_string()],
            blockers: vec![],
            next_action: "merge".to_string(),
            memory_ids_used: vec!["mem_1".to_string()],
            skill_candidates_proposed: vec![],
            compact_checkpoint_link: None,
            no_secret_marker: true,
        }
    }

    #[test]
    fn session_index_init_and_add() {
        let tmp = TempDir::new().unwrap();
        let index = SessionIndex::new(tmp.path()).unwrap();
        index.init().unwrap();

        let session = sample_session("sess_1", "First session");
        index.add_session(&session).unwrap();

        let found = index.get_session("sess_1").unwrap().unwrap();
        assert_eq!(found.id, "sess_1");
        assert_eq!(found.summary, "First session");
        assert!(found.no_secret_marker);
    }

    #[test]
    fn session_index_add_rejects_missing_marker() {
        let tmp = TempDir::new().unwrap();
        let index = SessionIndex::new(tmp.path()).unwrap();
        index.init().unwrap();

        let mut session = sample_session("sess_1", "First session");
        session.no_secret_marker = false;
        assert!(index.add_session(&session).is_err());
    }

    #[test]
    fn session_index_search() {
        let tmp = TempDir::new().unwrap();
        let index = SessionIndex::new(tmp.path()).unwrap();
        index.init().unwrap();

        let s1 = sample_session("sess_1", "Implement auth module");
        let mut s2 = sample_session("sess_2", "Fix login bug");
        s2.decisions = vec!["use jwt".to_string()];
        let mut s3 = sample_session("sess_3", "Update docs");
        s3.next_action = "publish docs".to_string();

        index.add_session(&s1).unwrap();
        index.add_session(&s2).unwrap();
        index.add_session(&s3).unwrap();

        let results = index.search("auth", 10).unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].id, "sess_1");

        let results = index.search("jwt", 10).unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].id, "sess_2");

        let results = index.search("publish", 10).unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].id, "sess_3");
    }

    #[test]
    fn session_index_list_recent() {
        let tmp = TempDir::new().unwrap();
        let index = SessionIndex::new(tmp.path()).unwrap();
        index.init().unwrap();

        let mut s1 = sample_session("sess_1", "Older");
        s1.updated_at = "1000.000".to_string();
        let mut s2 = sample_session("sess_2", "Newer");
        s2.updated_at = "2000.000".to_string();

        index.add_session(&s1).unwrap();
        index.add_session(&s2).unwrap();

        let results = index.list_recent(10).unwrap();
        assert_eq!(results.len(), 2);
        assert_eq!(results[0].id, "sess_2");
        assert_eq!(results[1].id, "sess_1");
    }

    #[test]
    fn session_index_get_missing() {
        let tmp = TempDir::new().unwrap();
        let index = SessionIndex::new(tmp.path()).unwrap();
        index.init().unwrap();

        assert!(index.get_session("sess_missing").unwrap().is_none());
    }

    #[test]
    fn session_record_default() {
        let r = SessionRecord::default();
        assert!(r.id.is_empty());
        assert!(!r.no_secret_marker);
    }
}
