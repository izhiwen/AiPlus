use serde::{Deserialize, Serialize};

pub const ROLLBACK_SCHEMA_VERSION: &str = "0.1.0";

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct RollbackPlan {
    pub schema_version: String,
    pub id: String,
    pub created_at: String,
    pub entries: Vec<RollbackEntry>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct RollbackEntry {
    pub action: String,
    pub original_path: String,
    pub backup_path: String,
    pub managed_file: bool,
}

impl RollbackPlan {
    pub fn single_restore(
        id: impl Into<String>,
        created_at: impl Into<String>,
        original_path: impl Into<String>,
        backup_path: impl Into<String>,
    ) -> Self {
        Self {
            schema_version: ROLLBACK_SCHEMA_VERSION.to_string(),
            id: id.into(),
            created_at: created_at.into(),
            entries: vec![RollbackEntry {
                action: "restore".to_string(),
                original_path: original_path.into(),
                backup_path: backup_path.into(),
                managed_file: true,
            }],
        }
    }

    pub fn add_restore(
        &mut self,
        original_path: impl Into<String>,
        backup_path: impl Into<String>,
    ) {
        self.entries.push(RollbackEntry {
            action: "restore".to_string(),
            original_path: original_path.into(),
            backup_path: backup_path.into(),
            managed_file: true,
        });
    }
}
