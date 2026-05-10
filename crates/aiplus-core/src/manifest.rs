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
    pub source: Option<String>,
    pub path: Option<String>,
    pub installed_at: Option<String>,
    pub updated_at: Option<String>,
}
