use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;

pub const IDENTITY_SCHEMA_VERSION_V2: &str = "0.2.0";

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
#[serde(rename_all = "snake_case", default)]
pub struct RoleIdentity {
    pub id: String,
    pub role: String,
    #[serde(alias = "version")]
    pub schema_version: String,
    pub scope: String,
    pub activation: Vec<String>,
    pub output_contract: String,
    pub owner_gates: Vec<String>,
    pub inherits: Vec<String>,
    // v2 fields
    pub role_contract: Option<String>,
    pub scope_boundaries: Vec<String>,
    pub current_responsibilities: Vec<String>,
    pub allowed_actions: Vec<String>,
    pub forbidden_actions: Vec<String>,
    pub memory_retrieval_policy: Option<String>,
    pub owner_gate_policy: Option<String>,
    pub result_packet_expectations: Vec<String>,
}

pub fn read_identity(root: &Path, role: &str) -> Result<RoleIdentity> {
    let path = root.join(format!(".aiplus/identities/{}.identity.toml", role));
    let text = fs::read_to_string(&path)
        .with_context(|| format!("read identity file for role {}", role))?;

    let mut identity: RoleIdentity =
        toml::from_str(&text).with_context(|| format!("parse identity TOML for role {}", role))?;

    identity.role = role.to_string();
    if identity.schema_version.is_empty() {
        identity.schema_version = IDENTITY_SCHEMA_VERSION_V2.to_string();
    }

    Ok(identity)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn read_identity_parses_v1_template() {
        let tmp = TempDir::new().unwrap();
        fs::create_dir_all(tmp.path().join(".aiplus/identities")).unwrap();
        let content = r#"
id = "aiplus.advisor.default"
role = "Advisor"
version = "0.1.0"
scope = "project"
activation = ["advice", "brainstorm"]
output_contract = "concise options"
owner_gates = ["publish", "deploy"]
inherits = ["aiplus-work-with-me when linked and available"]
"#;
        fs::write(
            tmp.path().join(".aiplus/identities/advisor.identity.toml"),
            content,
        )
        .unwrap();

        let identity = read_identity(tmp.path(), "advisor").unwrap();
        assert_eq!(identity.id, "aiplus.advisor.default");
        assert_eq!(identity.role, "advisor");
        assert_eq!(identity.scope, "project");
        assert!(identity.activation.contains(&"advice".to_string()));
        assert!(identity.owner_gates.contains(&"publish".to_string()));
    }

    #[test]
    fn read_identity_v2_fields_default() {
        let tmp = TempDir::new().unwrap();
        fs::create_dir_all(tmp.path().join(".aiplus/identities")).unwrap();
        let content = r#"
id = "aiplus.ceo.default"
role = "CEO"
scope = "project"
activation = []
output_contract = ""
owner_gates = []
inherits = []
"#;
        fs::write(
            tmp.path().join(".aiplus/identities/ceo.identity.toml"),
            content,
        )
        .unwrap();

        let identity = read_identity(tmp.path(), "ceo").unwrap();
        assert!(identity.role_contract.is_none());
        assert!(identity.scope_boundaries.is_empty());
        assert!(identity.current_responsibilities.is_empty());
    }
}
