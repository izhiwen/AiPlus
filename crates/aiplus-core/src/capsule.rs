use serde::{Deserialize, Serialize};
use std::collections::HashMap;

pub const CAPSULE_SCHEMA_VERSION: u32 = 1;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ContextCapsule {
    pub schema_version: u32,
    pub project_id: String,
    pub created_at: String,
    pub updated_at: String,
    pub objective: String,
    pub current_state: String,
    pub hot: CapsuleTier,
    pub warm: CapsuleTier,
    pub cold: CapsuleTier,
    pub decisions: Vec<CapsuleDecision>,
    pub changed_files: Vec<String>,
    pub owner_gates: Vec<CapsuleOwnerGate>,
    pub risks: Vec<CapsuleRisk>,
    pub verification: Vec<CapsuleVerification>,
    pub next_safe_action: String,
    pub resume_prompt: String,
    pub redaction: RedactionSummary,
    pub checksums: HashMap<String, String>,
}

impl Default for ContextCapsule {
    fn default() -> Self {
        Self {
            schema_version: CAPSULE_SCHEMA_VERSION,
            project_id: String::new(),
            created_at: String::new(),
            updated_at: String::new(),
            objective: String::new(),
            current_state: String::new(),
            hot: CapsuleTier::default(),
            warm: CapsuleTier::default(),
            cold: CapsuleTier::default(),
            decisions: Vec::new(),
            changed_files: Vec::new(),
            owner_gates: Vec::new(),
            risks: Vec::new(),
            verification: Vec::new(),
            next_safe_action: String::new(),
            resume_prompt: String::new(),
            redaction: RedactionSummary::default(),
            checksums: HashMap::new(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct CapsuleTier {
    pub tier: String,
    pub items: Vec<CapsuleItem>,
    pub max_items: usize,
    pub truncation_reason: Option<String>,
}

impl Default for CapsuleTier {
    fn default() -> Self {
        Self {
            tier: "unknown".to_string(),
            items: Vec::new(),
            max_items: 100,
            truncation_reason: None,
        }
    }
}

impl CapsuleTier {
    pub fn new(tier: &str, max_items: usize) -> Self {
        Self {
            tier: tier.to_string(),
            max_items,
            ..Self::default()
        }
    }

    pub fn add_item(&mut self, item: CapsuleItem) {
        if self.items.len() < self.max_items {
            self.items.push(item);
        } else if self.truncation_reason.is_none() {
            self.truncation_reason =
                Some(format!("truncated: max {} items exceeded", self.max_items));
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct CapsuleItem {
    pub id: String,
    pub content: String,
    pub importance: u8,
    pub source: String,
    pub created_at: String,
    pub expires_at: Option<String>,
    pub checksum: Option<String>,
}

impl CapsuleItem {
    pub fn score_importance(&self) -> u8 {
        let base = self.importance;
        let penalty = if self.content.is_empty() { 50 } else { 0 };
        let decay = if self.is_expired() { 30 } else { 0 };
        base.saturating_sub(penalty).saturating_sub(decay)
    }

    pub fn is_expired(&self) -> bool {
        if let Some(ref expires) = self.expires_at {
            if let Ok(expires_millis) = expires.parse::<u128>() {
                let now = std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .map(|d| d.as_millis())
                    .unwrap_or(0);
                return now > expires_millis;
            }
        }
        false
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct CapsuleDecision {
    pub id: String,
    pub description: String,
    pub status: String,
    pub decided_at: String,
    pub verified: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct CapsuleOwnerGate {
    pub label: String,
    pub status: String,
    pub required: bool,
    pub approved_by: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct CapsuleRisk {
    pub label: String,
    pub severity: String,
    pub mitigation: String,
    pub accepted: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct CapsuleVerification {
    pub label: String,
    pub status: String,
    pub verified_at: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
#[derive(Default)]
pub struct RedactionSummary {
    pub secret_values_printed: bool,
    pub raw_transcript_captured: bool,
    pub private_paths_included: bool,
    pub redaction_count: u64,
}

pub fn compute_capsule_checksum(content: &str) -> String {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};
    let mut hasher = DefaultHasher::new();
    content.hash(&mut hasher);
    format!("{:x}", hasher.finish())
}

pub fn build_capsule_from_compact_state(
    project_id: &str,
    objective: &str,
    current_state: &str,
    next_safe_action: &str,
) -> ContextCapsule {
    let now = timestamp();
    let mut capsule = ContextCapsule {
        schema_version: CAPSULE_SCHEMA_VERSION,
        project_id: project_id.to_string(),
        created_at: now.clone(),
        updated_at: now.clone(),
        objective: objective.to_string(),
        current_state: current_state.to_string(),
        hot: CapsuleTier::new("hot", 20),
        warm: CapsuleTier::new("warm", 50),
        cold: CapsuleTier::new("cold", 100),
        resume_prompt: format!(
            "Resume work on: {}. Current state: {}. Next action: {}.",
            objective, current_state, next_safe_action
        ),
        ..ContextCapsule::default()
    };

    capsule.redaction = RedactionSummary {
        secret_values_printed: false,
        raw_transcript_captured: false,
        private_paths_included: false,
        redaction_count: 0,
    };

    let checksum =
        compute_capsule_checksum(&format!("{}{}{}", project_id, objective, current_state));
    capsule.checksums.insert("capsule_v1".to_string(), checksum);

    capsule
}

pub fn timestamp() -> String {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| format!("{}.{:03}", d.as_secs(), d.subsec_millis()))
        .unwrap_or_else(|_| "0".to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn capsule_default_safe() {
        let capsule = ContextCapsule::default();
        assert_eq!(capsule.schema_version, 1);
        assert!(!capsule.redaction.secret_values_printed);
        assert!(!capsule.redaction.raw_transcript_captured);
        assert!(capsule.checksums.is_empty());
    }

    #[test]
    fn tier_truncation() {
        let mut tier = CapsuleTier::new("hot", 2);
        for i in 0..5 {
            tier.add_item(CapsuleItem {
                id: format!("item-{i}"),
                content: format!("content-{i}"),
                importance: 50,
                source: "test".to_string(),
                created_at: timestamp(),
                expires_at: None,
                checksum: None,
            });
        }
        assert_eq!(tier.items.len(), 2);
        assert!(tier.truncation_reason.is_some());
    }

    #[test]
    fn item_importance_scoring() {
        let item = CapsuleItem {
            id: "test".to_string(),
            content: "content".to_string(),
            importance: 80,
            source: "test".to_string(),
            created_at: timestamp(),
            expires_at: None,
            checksum: None,
        };
        assert_eq!(item.score_importance(), 80);

        let empty = CapsuleItem {
            id: "empty".to_string(),
            content: "".to_string(),
            importance: 80,
            source: "test".to_string(),
            created_at: timestamp(),
            expires_at: None,
            checksum: None,
        };
        assert_eq!(empty.score_importance(), 30);
    }

    #[test]
    fn item_expiry() {
        let past = (std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_millis()
            - 1000)
            .to_string();
        let expired = CapsuleItem {
            id: "expired".to_string(),
            content: "test".to_string(),
            importance: 80,
            source: "test".to_string(),
            created_at: timestamp(),
            expires_at: Some(past),
            checksum: None,
        };
        assert!(expired.is_expired());
        assert_eq!(expired.score_importance(), 50);

        let future = (std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_millis()
            + 1000)
            .to_string();
        let fresh = CapsuleItem {
            id: "fresh".to_string(),
            content: "test".to_string(),
            importance: 80,
            source: "test".to_string(),
            created_at: timestamp(),
            expires_at: Some(future),
            checksum: None,
        };
        assert!(!fresh.is_expired());
    }

    #[test]
    fn checksum_deterministic() {
        let a = compute_capsule_checksum("hello");
        let b = compute_capsule_checksum("hello");
        let c = compute_capsule_checksum("world");
        assert_eq!(a, b);
        assert_ne!(a, c);
    }

    #[test]
    fn build_capsule_creates_checksum() {
        let capsule = build_capsule_from_compact_state(
            "test-project",
            "Build v2",
            "In progress",
            "Continue implementation",
        );
        assert_eq!(capsule.project_id, "test-project");
        assert_eq!(capsule.schema_version, 1);
        assert!(capsule.checksums.contains_key("capsule_v1"));
        assert!(!capsule.redaction.secret_values_printed);
        assert!(!capsule.redaction.raw_transcript_captured);
    }
}
