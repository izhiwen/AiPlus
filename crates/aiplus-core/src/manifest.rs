use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Serialize, Deserialize, Default, Clone, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ProjectManifest {
    pub schema_version: Option<String>,
    pub installer: Option<String>,
    pub installer_version: Option<String>,
    pub installed_at: Option<String>,
    pub updated_at: Option<String>,
    pub target_root: Option<String>,
    pub runtime_adapters: Option<Vec<String>>,
    pub modules: Option<BTreeMap<String, ProjectManifestModule>>,
    pub managed_files: Option<Vec<String>>,
}

#[derive(Serialize, Deserialize, Default, Clone, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ProjectManifestModule {
    pub version: Option<String>,
    /// `bundled` (default, ships with the AiPlus CLI binary) or
    /// `external` (installed dynamically via `aiplus add --from-git`).
    pub source: Option<String>,
    pub path: Option<String>,
    pub installed_at: Option<String>,
    pub updated_at: Option<String>,
    /// Origin git URL for external modules. None for bundled.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source_url: Option<String>,
    /// Pinned git tag/branch/commit for external modules. None for bundled.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source_ref: Option<String>,
}
