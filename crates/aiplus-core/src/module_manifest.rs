use crate::assets::{embedded_asset_paths, embedded_asset_text};
use crate::paths::project_local_path;
use anyhow::{anyhow, Result};
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;
use std::path::Path;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ModuleSpec {
    pub name: &'static str,
    pub vendor_name: &'static str,
    pub version: &'static str,
    pub path: &'static str,
    pub required_files: &'static [&'static str],
    /// `true` = installed by default on `aiplus install`. `false` = opt-in
    /// only, requires explicit `aiplus add <name>` from the user. Niche
    /// or audience-specific modules should set this to `false` to avoid
    /// polluting every AiPlus install with files most users do not need.
    pub auto_install: bool,
}

pub const MODULES: &[ModuleSpec] = &[
    ModuleSpec {
        name: "compact-reminder",
        vendor_name: "aiplus-compact-reminder",
        version: "0.4.6",
        path: ".aiplus/modules/aiplus-compact-reminder",
        required_files: &[
            "LICENSE",
            "core/templates/current-handoff.md",
            "core/templates/compact-policy.json",
        ],
        auto_install: true,
    },
    ModuleSpec {
        name: "auto-team-consultant",
        vendor_name: "aiplus-auto-team-consultant",
        version: "0.4.6",
        path: ".aiplus/modules/aiplus-auto-team-consultant",
        required_files: &[
            "LICENSE",
            "adapters/codex/skills/auto-team-consultant/SKILL.md",
            "core/templates/TEMPLATE_INDEX.md",
        ],
        auto_install: true,
    },
    ModuleSpec {
        name: "agent-memory",
        vendor_name: "aiplus-agent-memory",
        version: "0.5.1",
        path: ".aiplus/modules/aiplus-agent-memory",
        required_files: &[
            "README.md",
            "core/schemas/memory-record.schema.json",
            "core/templates/memory-context.md",
            "adapters/codex/skills/agent-memory/SKILL.md",
        ],
        auto_install: true,
    },
    ModuleSpec {
        name: "agent-team",
        vendor_name: "aiplus-agent-team",
        version: "0.2.0",
        path: ".aiplus/modules/aiplus-agent-team",
        required_files: &[
            "core/templates/advisor.toml",
            "core/templates/ceo.toml",
            "core/templates/agent-team.toml",
            "core/templates/personas/advisor.md",
            "adapters/claude-code/subagents.toml",
            "adapters/claude-code/claude-md-block.md",
        ],
        auto_install: true,
    },
    ModuleSpec {
        name: "agent-key",
        vendor_name: "aiplus-agent-key",
        version: "0.1.0",
        path: ".aiplus/modules/aiplus-agent-key",
        required_files: &[
            "README.md",
            "DESIGN.md",
            "core/alias-conventions.md",
            "core/example-aliases.md",
            "core/example-aliases.tsv",
        ],
        // Auto-installed: agent-key documents the built-in `aiplus
        // secret-broker` subcommand and ships alias conventions + the
        // example TSV layout. Project-local docs let runtimes reference
        // them without round-tripping to GitHub.
        auto_install: true,
    },
    ModuleSpec {
        name: "agent-velocity",
        vendor_name: "aiplus-agent-velocity",
        version: "0.1.0",
        path: ".aiplus/modules/aiplus-agent-velocity",
        required_files: &[
            "README.md",
            "DESIGN.md",
            "core/schemas/run-record.schema.json",
            "core/schemas/estimate-record.schema.json",
            "core/schemas/config.schema.json",
            "core/schemas/rare-case-record.schema.json",
        ],
        // Auto-installed: agent-velocity documents the built-in
        // `aiplus velocity` subcommand and ships the JSONL schemas
        // (run-record, estimate-record, config, rare-case-record).
        auto_install: true,
    },
    ModuleSpec {
        name: "aieconlab",
        vendor_name: "aieconlab",
        version: "0.2.0",
        path: ".aiplus/modules/aieconlab",
        required_files: &[
            "core/templates/advisor.toml",
            "core/templates/pi.toml",
            "core/templates/econ-team.toml",
            "core/templates/personas/advisor.md",
            "adapters/claude-code/subagents.toml",
            "adapters/claude-code/claude-md-block.md",
        ],
        // Opt-in by design: AiEconLab is for applied-economics
        // research, which is a niche audience relative to the
        // software-engineering majority of AiPlus users. Auto-installing
        // it would put 32+ research roles into every AiPlus project's
        // .aiplus/agents/ directory — left-handed scissors in every
        // kitchen drawer. Users who want AEL run `aiplus add aieconlab`.
        auto_install: false,
    },
];

pub const MODULE_SLUG_COMPACT_REMINDER: &str = "compact-reminder";
pub const MODULE_SLUG_COMPACT_REMINDER_LEGACY_ALIAS: &str = "auto-compact";
pub const MODULE_SLUG_AGENT_TEAM: &str = "agent-team";
pub const MODULE_SLUG_AIECONLAB: &str = "aieconlab";

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
#[serde(rename_all = "camelCase")]
pub struct ModuleManifest {
    pub schema_version: String,
    #[serde(alias = "auto-compact")]
    pub name: String,
    pub display_name: String,
    pub version: String,
    pub source: String,
    pub license: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub abbreviation: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub requires: Option<ModuleRequires>,
    pub required_files: Vec<String>,
    pub managed_files: Vec<String>,
    pub runtime_adapters: Vec<String>,
    pub install_hints: Vec<String>,
    pub doctor_checks: Vec<DoctorCheck>,
    pub public_private_boundary: Boundary,
    pub secret_boundary: Boundary,
    pub legacy_status: Option<LegacyStatus>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
#[serde(rename_all = "snake_case")]
pub struct ModuleRequires {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub aiplus_min_version: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub substrate_modules: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
#[serde(rename_all = "camelCase")]
pub struct DoctorCheck {
    pub id: String,
    pub path: Option<String>,
    pub description: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
#[serde(rename_all = "camelCase")]
pub struct Boundary {
    pub status: String,
    pub notes: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
#[serde(rename_all = "camelCase")]
pub struct LegacyStatus {
    pub status: String,
    pub notes: Vec<String>,
}

pub fn bundled_module_specs() -> &'static [ModuleSpec] {
    MODULES
}

pub fn normalize_module(value: Option<&str>) -> Option<&'static str> {
    match value? {
        "compact-reminder"
        | "aiplus-compact-reminder"
        | "auto-compact"
        | "aiplus-auto-compact"
        | "compact" => Some("compact-reminder"),
        "auto-team-consultant" | "aiplus-auto-team-consultant" | "team" => {
            Some("auto-team-consultant")
        }
        "agent-memory" | "aiplus-agent-memory" | "memory" => Some("agent-memory"),
        "agent-team" | "aiplus-agent-team" => Some("agent-team"),
        "aieconlab"
        | "AiEconLab"
        | "ael"
        | "AEL"
        | "econ-team"
        | "econ-agent-team"
        | "aiplus-econ-agent-team" => Some("aieconlab"),
        _ => None,
    }
}

pub fn module_spec(name: &str) -> Option<ModuleSpec> {
    MODULES.iter().copied().find(|spec| spec.name == name)
}

pub fn default_module_names() -> Vec<String> {
    MODULES
        .iter()
        .filter(|spec| spec.auto_install)
        .map(|spec| spec.name.to_string())
        .collect()
}

/// All bundled module names, including opt-in ones. Used by `aiplus add`
/// to look up modules a user has explicitly requested.
pub fn all_module_names() -> Vec<String> {
    MODULES.iter().map(|spec| spec.name.to_string()).collect()
}

pub fn available_modules_text() -> String {
    MODULES
        .iter()
        .map(|spec| spec.name)
        .collect::<Vec<_>>()
        .join(", ")
}

pub fn parse_module_manifest(text: &str) -> Result<ModuleManifest> {
    Ok(serde_json::from_str(text)?)
}

pub fn bundled_module_manifest(spec: ModuleSpec) -> Result<ModuleManifest> {
    let rel = format!("{}/aiplus-module.json", spec.vendor_name);
    parse_module_manifest(&embedded_asset_text(&rel)?)
}

pub fn validate_module_manifest(spec: ModuleSpec, manifest: &ModuleManifest) -> Result<()> {
    if manifest.schema_version != "0.1.0" {
        return Err(anyhow!(
            "module manifest schemaVersion unsupported for {}: {}",
            spec.name,
            manifest.schema_version
        ));
    }
    if manifest.name != spec.name && manifest.name != MODULE_SLUG_COMPACT_REMINDER_LEGACY_ALIAS {
        return Err(anyhow!(
            "module manifest name mismatch: expected {}, got {}",
            spec.name,
            manifest.name
        ));
    }
    if manifest.version != spec.version {
        return Err(anyhow!(
            "module manifest version mismatch for {}: expected {}, got {}",
            spec.name,
            spec.version,
            manifest.version
        ));
    }
    if manifest.source != "bundled" {
        return Err(anyhow!(
            "module manifest source mismatch for {}: expected bundled, got {}",
            spec.name,
            manifest.source
        ));
    }
    if manifest.display_name.trim().is_empty() {
        return Err(anyhow!(
            "module manifest displayName is empty: {}",
            spec.name
        ));
    }
    if manifest.license.trim().is_empty() {
        return Err(anyhow!("module manifest license is empty: {}", spec.name));
    }
    let manifest_required: BTreeSet<&str> =
        manifest.required_files.iter().map(String::as_str).collect();
    for required in spec.required_files {
        if !manifest_required.contains(required) {
            return Err(anyhow!(
                "module manifest missing required file for {}: {}",
                spec.name,
                required
            ));
        }
    }
    for required in &manifest.required_files {
        validate_manifest_relative_path(spec.name, "requiredFiles", required)?;
        validate_embedded_module_path(spec, "requiredFiles", required)?;
    }
    if manifest.runtime_adapters.is_empty() {
        return Err(anyhow!(
            "module manifest has no runtime adapters: {}",
            spec.name
        ));
    }
    for adapter in &manifest.runtime_adapters {
        if !["codex", "claude-code", "opencode"].contains(&adapter.as_str()) {
            return Err(anyhow!(
                "module manifest runtime adapter unsupported for {}: {}",
                spec.name,
                adapter
            ));
        }
    }
    validate_boundary_status(
        spec.name,
        "publicPrivateBoundary",
        &manifest.public_private_boundary,
    )?;
    validate_boundary_status(spec.name, "secretBoundary", &manifest.secret_boundary)?;
    if manifest.managed_files.is_empty() {
        return Err(anyhow!(
            "module manifest has no managed files: {}",
            spec.name
        ));
    }
    for managed in &manifest.managed_files {
        validate_manifest_relative_path(spec.name, "managedFiles", managed)?;
    }
    for check in &manifest.doctor_checks {
        if check.id.trim().is_empty() || check.description.trim().is_empty() {
            return Err(anyhow!(
                "module manifest doctorChecks entry is incomplete for {}",
                spec.name
            ));
        }
        if let Some(path) = check.path.as_deref() {
            validate_manifest_relative_path(spec.name, "doctorChecks.path", path)?;
            validate_embedded_module_path(spec, "doctorChecks.path", path)?;
        }
    }
    Ok(())
}

fn validate_manifest_relative_path(module: &str, field: &str, path: &str) -> Result<()> {
    project_local_path(Path::new("."), path).map_err(|error| {
        anyhow!("module manifest {field} path invalid for {module}: {path}: {error}")
    })?;
    Ok(())
}

fn validate_embedded_module_path(spec: ModuleSpec, field: &str, path: &str) -> Result<()> {
    let asset_path = format!("{}/{}", spec.vendor_name, path);
    if embedded_asset_paths().any(|embedded| embedded == asset_path) {
        Ok(())
    } else {
        Err(anyhow!(
            "module manifest {field} path missing from embedded assets for {}: {}",
            spec.name,
            path
        ))
    }
}

fn validate_boundary_status(module: &str, field: &str, boundary: &Boundary) -> Result<()> {
    if !["public", "private", "metadata-only", "forbidden"].contains(&boundary.status.as_str()) {
        return Err(anyhow!(
            "module manifest {field} status unsupported for {module}: {}",
            boundary.status
        ));
    }
    if boundary.notes.is_empty() {
        return Err(anyhow!(
            "module manifest {field} notes are empty for {module}"
        ));
    }
    Ok(())
}

pub fn validate_bundled_module_manifests() -> Result<Vec<ModuleManifest>> {
    MODULES
        .iter()
        .copied()
        .map(|spec| {
            let manifest = bundled_module_manifest(spec)?;
            validate_module_manifest(spec, &manifest)?;
            Ok(manifest)
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bundled_module_manifests_match_static_registry() {
        let manifests = validate_bundled_module_manifests().unwrap();
        assert_eq!(manifests.len(), MODULES.len());
        assert!(manifests
            .iter()
            .any(|manifest| manifest.name == "compact-reminder"));
        assert!(manifests
            .iter()
            .any(|manifest| manifest.name == "auto-team-consultant"));
        assert!(manifests
            .iter()
            .any(|manifest| manifest.name == "agent-memory"));
        assert!(manifests
            .iter()
            .any(|manifest| manifest.name == "agent-team"));
        assert!(manifests
            .iter()
            .any(|manifest| manifest.name == "aieconlab"));
    }

    #[test]
    fn agent_team_slug_is_unique() {
        let slugs: Vec<&str> = MODULES.iter().map(|m| m.name).collect();
        assert_eq!(
            slugs.iter().filter(|s| s.contains("agent-team")).count(),
            1,
            "agent-team slug must be unique"
        );
        // Verify no prefix collision with agent-memory or auto-team-consultant
        assert!(
            !"agent-team".starts_with("agent-memory"),
            "agent-team must not prefix-match agent-memory"
        );
        assert!(
            !"agent-team".starts_with("auto-team"),
            "agent-team must not prefix-match auto-team-consultant"
        );
        // Word boundary check: ensure "agent-team" is not a substring of any other slug
        for slug in &slugs {
            if *slug != MODULE_SLUG_AGENT_TEAM {
                assert!(
                    !slug.contains(MODULE_SLUG_AGENT_TEAM),
                    "slug '{}' must not contain '{}' as substring",
                    slug,
                    MODULE_SLUG_AGENT_TEAM
                );
                assert!(
                    !MODULE_SLUG_AGENT_TEAM.contains(slug),
                    "'{}' must not contain slug '{}' as substring",
                    MODULE_SLUG_AGENT_TEAM,
                    slug
                );
            }
        }
        // Explicit word boundary check against auto-team-consultant
        assert_ne!(
            MODULE_SLUG_AGENT_TEAM, "auto-team-consultant",
            "agent-team must not conflict with auto-team-consultant"
        );
        assert!(
            !"auto-team-consultant".contains(MODULE_SLUG_AGENT_TEAM),
            "auto-team-consultant must not contain agent-team as substring"
        );
        assert!(
            !MODULE_SLUG_AGENT_TEAM.contains("auto-team-consultant"),
            "agent-team must not contain auto-team-consultant as substring"
        );
    }

    #[test]
    fn aliases_normalize_to_canonical_module_names() {
        assert_eq!(normalize_module(Some("compact")), Some("compact-reminder"));
        assert_eq!(
            normalize_module(Some("auto-compact")),
            Some("compact-reminder")
        );
        assert_eq!(
            normalize_module(Some("aiplus-auto-compact")),
            Some("compact-reminder")
        );
        assert_eq!(
            normalize_module(Some("aiplus-compact-reminder")),
            Some("compact-reminder")
        );
        assert_eq!(normalize_module(Some("team")), Some("auto-team-consultant"));
        assert_eq!(normalize_module(Some("memory")), Some("agent-memory"));
        assert_eq!(normalize_module(Some("agent-team")), Some("agent-team"));
        assert_eq!(
            normalize_module(Some("aiplus-agent-team")),
            Some("agent-team")
        );
        assert_eq!(normalize_module(Some("aieconlab")), Some("aieconlab"));
        assert_eq!(normalize_module(Some("AiEconLab")), Some("aieconlab"));
        assert_eq!(normalize_module(Some("ael")), Some("aieconlab"));
        assert_eq!(normalize_module(Some("AEL")), Some("aieconlab"));
        assert_eq!(normalize_module(Some("econ-team")), Some("aieconlab"));
        assert_eq!(normalize_module(Some("econ-agent-team")), Some("aieconlab"));
        assert_eq!(
            normalize_module(Some("aiplus-econ-agent-team")),
            Some("aieconlab")
        );
        assert_eq!(normalize_module(Some("unknown")), None);
    }

    #[test]
    fn module_manifest_validation_rejects_schema_enum_and_boundary_errors() {
        let spec = module_spec("compact-reminder").unwrap();
        let mut manifest = bundled_module_manifest(spec).unwrap();
        manifest.schema_version = "999.0.0".to_string();
        assert!(validate_module_manifest(spec, &manifest)
            .unwrap_err()
            .to_string()
            .contains("schemaVersion unsupported"));

        let mut manifest = bundled_module_manifest(spec).unwrap();
        manifest.source = "external".to_string();
        assert!(validate_module_manifest(spec, &manifest)
            .unwrap_err()
            .to_string()
            .contains("source mismatch"));

        let mut manifest = bundled_module_manifest(spec).unwrap();
        manifest.secret_boundary.status = "maybe".to_string();
        assert!(validate_module_manifest(spec, &manifest)
            .unwrap_err()
            .to_string()
            .contains("status unsupported"));
    }

    #[test]
    fn module_manifest_validation_rejects_traversal_and_missing_doctor_paths() {
        let spec = module_spec("agent-memory").unwrap();
        let mut manifest = bundled_module_manifest(spec).unwrap();
        manifest.managed_files.push("../outside".to_string());
        assert!(validate_module_manifest(spec, &manifest)
            .unwrap_err()
            .to_string()
            .contains("path invalid"));

        let mut manifest = bundled_module_manifest(spec).unwrap();
        manifest.doctor_checks.push(DoctorCheck {
            id: "missing".to_string(),
            path: Some("adapters/missing/README.md".to_string()),
            description: "Missing adapter doc".to_string(),
        });
        assert!(validate_module_manifest(spec, &manifest)
            .unwrap_err()
            .to_string()
            .contains("missing from embedded assets"));
    }

    #[test]
    fn module_manifest_runtime_adapters_match_embedded_adapter_dirs() {
        for spec in MODULES {
            let manifest = bundled_module_manifest(*spec).unwrap();
            let declared: BTreeSet<&str> = manifest
                .runtime_adapters
                .iter()
                .map(String::as_str)
                .collect();
            let prefix = format!("{}/adapters/", spec.vendor_name);
            let embedded: BTreeSet<&str> = embedded_asset_paths()
                .filter_map(|path| path.strip_prefix(&prefix))
                .filter_map(|stripped| stripped.split('/').next())
                .collect();
            assert_eq!(declared, embedded, "{}", spec.name);
        }
    }
}
