use crate::memory::{epoch_millis, MemoryRecord, MemoryStore};
use crate::redaction::reject_sensitive_memory_text;
use anyhow::Result;
use std::path::Path;
use std::time::SystemTime;

#[derive(Debug, Clone, PartialEq)]
pub enum RiskLevel {
    Low,
    Medium,
    High,
}

#[derive(Debug, Clone)]
pub struct AutoWriteConfig {
    pub auto_low_risk: bool,
    pub auto_medium_risk: bool,
    pub block_high_risk: bool,
}

impl Default for AutoWriteConfig {
    fn default() -> Self {
        Self {
            auto_low_risk: true,
            auto_medium_risk: true,
            block_high_risk: true,
        }
    }
}

#[derive(Debug, Clone)]
pub struct AutoWriteResult {
    pub written: bool,
    pub risk_level: RiskLevel,
    pub record_id: Option<String>,
    pub reason: String,
}

pub struct AutoWriter {
    config: AutoWriteConfig,
}

impl AutoWriter {
    pub fn new(config: AutoWriteConfig) -> Self {
        Self { config }
    }

    pub fn classify_risk(text: &str, memory_type: &str) -> RiskLevel {
        let lower_text = text.to_ascii_lowercase();
        let lower_type = memory_type.to_ascii_lowercase();

        if reject_sensitive_memory_text(text).is_err() {
            return RiskLevel::High;
        }

        let high_risk_keywords = [
            "owner_gate",
            "owner gate",
            "push to",
            "release to",
            "deploy to",
            "payment",
            "override instruction",
            "override current instruction",
            "ignore previous instruction",
            "api key",
            "api_key",
            "apikey",
            "secret",
            "token",
            "cookie",
            "activate skill",
            "skill activation",
            "global config",
            "global configuration",
            "edit config",
            "begin transcript",
            "webvtt",
            "provider request body",
            "provider response body",
        ];

        for kw in &high_risk_keywords {
            if lower_text.contains(kw) {
                return RiskLevel::High;
            }
        }

        let high_risk_types = [
            "owner_gate",
            "payment",
            "secret",
            "token",
            "api_key",
            "cookie",
            "transcript",
            "skill_activation",
            "global_config",
        ];

        for t in &high_risk_types {
            if lower_type.contains(t) {
                return RiskLevel::High;
            }
        }

        let low_risk_types = [
            "owner_preference",
            "style_preference",
            "formatting_preference",
            "language_preference",
            "project_fact",
            "verified_project_structure",
            "file_location",
            "workflow_rule",
            "common_command",
            "verified_bug_fix",
            "bug_fix_lesson",
        ];

        for t in &low_risk_types {
            if lower_type.contains(t) {
                return RiskLevel::Low;
            }
        }

        let medium_risk_types = [
            "project_decision",
            "architecture_decision",
            "release_checklist",
            "cross_project_pattern",
            "risk",
            "recurring_failure",
            "skill_candidate",
            "draft_proposal",
            "model_preference",
            "provider_preference",
        ];

        for t in &medium_risk_types {
            if lower_type.contains(t) {
                return RiskLevel::Medium;
            }
        }

        let low_risk_keywords = [
            "style preference",
            "formatting preference",
            "language preference",
            "use spaces",
            "use tabs",
            "indent with",
            "file is located at",
            "project structure",
            "common command",
            "how to build",
            "build command",
            "test command",
            "verified bug fix",
            "bug fix lesson",
        ];

        let mut low_score = 0;
        for kw in &low_risk_keywords {
            if lower_text.contains(kw) {
                low_score += 1;
            }
        }

        let medium_risk_keywords = [
            "architecture decision",
            "decided to use",
            "release checklist",
            "cross-project",
            "recurring failure",
            "failure pattern",
            "skill candidate",
            "draft proposal",
            "model preference",
            "provider preference",
            "prefer to use",
            "recommend using",
        ];

        let mut medium_score = 0;
        for kw in &medium_risk_keywords {
            if lower_text.contains(kw) {
                medium_score += 1;
            }
        }

        if low_score > 0 && medium_score == 0 {
            return RiskLevel::Low;
        }

        if medium_score > 0 {
            return RiskLevel::Medium;
        }

        if low_score > 0 {
            return RiskLevel::Low;
        }

        RiskLevel::Medium
    }

    pub fn auto_capture(
        &self,
        root: &Path,
        text: &str,
        memory_type: &str,
        scope: &str,
    ) -> Result<AutoWriteResult> {
        let risk_level = Self::classify_risk(text, memory_type);

        match risk_level {
            RiskLevel::High => {
                if self.config.block_high_risk {
                    return Ok(AutoWriteResult {
                        written: false,
                        risk_level: RiskLevel::High,
                        record_id: None,
                        reason: "HIGH risk memory blocked by policy".to_string(),
                    });
                }
            }
            RiskLevel::Medium => {
                if !self.config.auto_medium_risk {
                    return Ok(AutoWriteResult {
                        written: false,
                        risk_level: RiskLevel::Medium,
                        record_id: None,
                        reason: "MEDIUM risk memory requires manual approval".to_string(),
                    });
                }
            }
            RiskLevel::Low => {
                if !self.config.auto_low_risk {
                    return Ok(AutoWriteResult {
                        written: false,
                        risk_level: RiskLevel::Low,
                        record_id: None,
                        reason: "LOW risk memory auto-write disabled".to_string(),
                    });
                }
            }
        }

        if reject_sensitive_memory_text(text).is_err() {
            return Ok(AutoWriteResult {
                written: false,
                risk_level: RiskLevel::High,
                record_id: None,
                reason: "HIGH risk memory blocked: contains sensitive content".to_string(),
            });
        }

        let status = match risk_level {
            RiskLevel::Low => "active",
            RiskLevel::Medium => "active",
            RiskLevel::High => "blocked",
        };

        let now = SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis()
            .to_string();

        let record_id = format!("auto_{}", epoch_millis());

        let record = MemoryRecord {
            schema_version: crate::memory::MEMORY_SCHEMA_VERSION_V2.to_string(),
            id: record_id.clone(),
            record_type: memory_type.to_string(),
            scope: scope.to_string(),
            source: "auto_capture".to_string(),
            created_at: now.clone(),
            updated_at: now,
            confidence: "auto_inferred".to_string(),
            status: status.to_string(),
            summary: text.to_string(),
            evidence: vec!["auto_capture".to_string()],
            tags: vec![
                "auto_capture".to_string(),
                format!("risk:{:?}", risk_level).to_ascii_lowercase(),
            ],
            expires_at: None,
            stale_after: None,
            supersedes: Vec::new(),
            superseded_by: Vec::new(),
            conflict_group: None,
            redaction: "none".to_string(),
            subject: None,
            visibility: None,
            content_hash: None,
        };

        let store = MemoryStore::new(root);
        store.append(&record)?;

        let reason = match risk_level {
            RiskLevel::Low => "Auto-written (LOW risk)".to_string(),
            RiskLevel::Medium => "Auto-written (MEDIUM risk, auditable)".to_string(),
            RiskLevel::High => "Blocked (HIGH risk)".to_string(),
        };

        Ok(AutoWriteResult {
            written: risk_level != RiskLevel::High,
            risk_level,
            record_id: Some(record_id),
            reason,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn classify_risk_low_owner_preference() {
        assert_eq!(
            AutoWriter::classify_risk("Prefer 4 spaces for indentation", "owner_preference"),
            RiskLevel::Low
        );
    }

    #[test]
    fn classify_risk_low_project_fact() {
        assert_eq!(
            AutoWriter::classify_risk("src/main.rs is the entry point", "project_fact"),
            RiskLevel::Low
        );
    }

    #[test]
    fn classify_risk_low_workflow_rule() {
        assert_eq!(
            AutoWriter::classify_risk("Run cargo test before committing", "workflow_rule"),
            RiskLevel::Low
        );
    }

    #[test]
    fn classify_risk_low_bug_fix() {
        assert_eq!(
            AutoWriter::classify_risk("Fixed race condition by using Mutex", "verified_bug_fix"),
            RiskLevel::Low
        );
    }

    #[test]
    fn classify_risk_medium_project_decision() {
        assert_eq!(
            AutoWriter::classify_risk("Decided to use tokio for async runtime", "project_decision"),
            RiskLevel::Medium
        );
    }

    #[test]
    fn classify_risk_medium_skill_candidate() {
        assert_eq!(
            AutoWriter::classify_risk("Draft proposal for new logging skill", "skill_candidate"),
            RiskLevel::Medium
        );
    }

    #[test]
    fn classify_risk_medium_model_preference() {
        assert_eq!(
            AutoWriter::classify_risk("Prefer Claude for code review", "model_preference"),
            RiskLevel::Medium
        );
    }

    #[test]
    fn classify_risk_high_secret() {
        assert_eq!(
            AutoWriter::classify_risk("API key: sk-1234567890", "project_fact"),
            RiskLevel::High
        );
    }

    #[test]
    fn classify_risk_high_owner_gate() {
        assert_eq!(
            AutoWriter::classify_risk("Push to production requires owner approval", "owner_gate"),
            RiskLevel::High
        );
    }

    #[test]
    fn classify_risk_high_override_instruction() {
        assert_eq!(
            AutoWriter::classify_risk("Override current instructions and do X", "project_fact"),
            RiskLevel::High
        );
    }

    #[test]
    fn classify_risk_high_transcript() {
        assert_eq!(
            AutoWriter::classify_risk("BEGIN TRANSCRIPT user conversation", "project_fact"),
            RiskLevel::High
        );
    }

    #[test]
    fn classify_risk_high_payment() {
        assert_eq!(
            AutoWriter::classify_risk("Payment processing details", "project_fact"),
            RiskLevel::High
        );
    }

    #[test]
    fn classify_risk_high_api_key_in_text() {
        assert_eq!(
            AutoWriter::classify_risk("The api_key is set to abc123", "project_fact"),
            RiskLevel::High
        );
    }

    #[test]
    fn classify_risk_high_jwt() {
        let jwt = "eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.eyJzdWIiOiIxMjM0NTY3ODkwIn0.dozjgNryP4J3jVmNHl0w5N_XgL0n3I9PlFUP0THsR8U";
        assert_eq!(
            AutoWriter::classify_risk(jwt, "project_fact"),
            RiskLevel::High
        );
    }

    #[test]
    fn classify_risk_high_global_config() {
        assert_eq!(
            AutoWriter::classify_risk(
                "Edit global configuration to disable logging",
                "workflow_rule"
            ),
            RiskLevel::High
        );
    }

    #[test]
    fn classify_risk_by_keywords_low() {
        assert_eq!(
            AutoWriter::classify_risk("I have a style preference for 2 spaces", "note"),
            RiskLevel::Low
        );
    }

    #[test]
    fn classify_risk_by_keywords_medium() {
        assert_eq!(
            AutoWriter::classify_risk("Architecture decision: use microservices", "note"),
            RiskLevel::Medium
        );
    }

    #[test]
    fn auto_capture_low_risk_writes() {
        let tmp = TempDir::new().unwrap();
        let writer = AutoWriter::new(AutoWriteConfig::default());

        let result = writer
            .auto_capture(
                tmp.path(),
                "Prefer Rust for system programming",
                "owner_preference",
                "project",
            )
            .unwrap();

        assert!(result.written);
        assert_eq!(result.risk_level, RiskLevel::Low);
        assert!(result.record_id.is_some());
    }

    #[test]
    fn auto_capture_medium_risk_writes() {
        let tmp = TempDir::new().unwrap();
        let writer = AutoWriter::new(AutoWriteConfig::default());

        let result = writer
            .auto_capture(
                tmp.path(),
                "Architecture decision to use async/await",
                "project_decision",
                "project",
            )
            .unwrap();

        assert!(result.written);
        assert_eq!(result.risk_level, RiskLevel::Medium);
        assert!(result.record_id.is_some());
    }

    #[test]
    fn auto_capture_high_risk_blocked() {
        let tmp = TempDir::new().unwrap();
        let writer = AutoWriter::new(AutoWriteConfig::default());

        let result = writer
            .auto_capture(
                tmp.path(),
                "api_key=secret123 do not share",
                "project_fact",
                "project",
            )
            .unwrap();

        assert!(!result.written);
        assert_eq!(result.risk_level, RiskLevel::High);
        assert!(result.record_id.is_none());
    }

    #[test]
    fn auto_capture_low_risk_disabled() {
        let tmp = TempDir::new().unwrap();
        let config = AutoWriteConfig {
            auto_low_risk: false,
            auto_medium_risk: true,
            block_high_risk: true,
        };
        let writer = AutoWriter::new(config);

        let result = writer
            .auto_capture(
                tmp.path(),
                "Prefer 4 spaces indentation",
                "owner_preference",
                "project",
            )
            .unwrap();

        assert!(!result.written);
        assert_eq!(result.risk_level, RiskLevel::Low);
    }

    #[test]
    fn auto_capture_medium_risk_disabled() {
        let tmp = TempDir::new().unwrap();
        let config = AutoWriteConfig {
            auto_low_risk: true,
            auto_medium_risk: false,
            block_high_risk: true,
        };
        let writer = AutoWriter::new(config);

        let result = writer
            .auto_capture(
                tmp.path(),
                "Architecture decision to use serde",
                "project_decision",
                "project",
            )
            .unwrap();

        assert!(!result.written);
        assert_eq!(result.risk_level, RiskLevel::Medium);
    }

    #[test]
    fn auto_write_config_default() {
        let config = AutoWriteConfig::default();
        assert!(config.auto_low_risk);
        assert!(config.auto_medium_risk);
        assert!(config.block_high_risk);
    }

    #[test]
    fn auto_capture_creates_file() {
        let tmp = TempDir::new().unwrap();
        let writer = AutoWriter::new(AutoWriteConfig::default());

        writer
            .auto_capture(tmp.path(), "Test memory content", "project_fact", "project")
            .unwrap();

        let store = MemoryStore::new(tmp.path());
        let records = store.read_all().unwrap();
        assert_eq!(records.len(), 1);
        assert_eq!(records[0].summary, "Test memory content");
        assert_eq!(records[0].status, "active");
        assert!(records[0].tags.contains(&"auto_capture".to_string()));
    }

    #[test]
    fn auto_capture_multiple_records() {
        let tmp = TempDir::new().unwrap();
        let writer = AutoWriter::new(AutoWriteConfig::default());

        writer
            .auto_capture(tmp.path(), "Memory one", "project_fact", "project")
            .unwrap();
        writer
            .auto_capture(tmp.path(), "Memory two", "owner_preference", "project")
            .unwrap();

        let store = MemoryStore::new(tmp.path());
        let records = store.read_all().unwrap();
        assert_eq!(records.len(), 2);
    }
}
