use serde::{Deserialize, Serialize};

use crate::agent_team::types::CheckKind;

/// Evidence collected from a single check execution.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CheckEvidence {
    pub check_id: String,
    pub kind: CheckKind,
    pub cmd: Option<String>,
    pub exit_code: Option<i32>,
    pub stdout: Option<String>,
    pub stderr: Option<String>,
    pub file_exists: Option<bool>,
    pub regex_matched: Option<bool>,
    pub duration_ms: u64,
    pub timestamp: String,
}

/// Collector that aggregates evidence from multiple checks.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct EvidenceCollector {
    pub checks: Vec<CheckEvidence>,
}

impl EvidenceCollector {
    pub fn new() -> Self {
        Self { checks: Vec::new() }
    }

    pub fn collect(&mut self, evidence: CheckEvidence) {
        self.checks.push(evidence);
    }

    pub fn to_json(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string_pretty(&self.checks)
    }

    pub fn to_jsonl(&self) -> Result<String, serde_json::Error> {
        let mut lines = Vec::new();
        for evidence in &self.checks {
            lines.push(serde_json::to_string(evidence)?);
        }
        Ok(lines.join("\n"))
    }

    pub fn len(&self) -> usize {
        self.checks.len()
    }

    pub fn is_empty(&self) -> bool {
        self.checks.is_empty()
    }
}
