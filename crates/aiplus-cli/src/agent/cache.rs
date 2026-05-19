// TODO(v0.2): WarmBenchCache scaffolding for cross-session role-state
// caching. Wired into `aiplus agent route` once we add the warm-bench
// fast-path. Currently unused — keep as scaffolding rather than rewrite.
#![allow(dead_code)]

use crate::agent::core::AgentConfig;
use aiplus_core::agent_team::RoleId;
use anyhow::{anyhow, Context, Result};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, HashMap};
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Condvar, Mutex, OnceLock};
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

const DISK_CACHE_SCHEMA_VERSION: &str = "0.2.0";
const DISK_CACHE_CONFIG_PATH: &str = ".aiplus/agent-team.toml";
const DISK_CACHE_WARNING_PATH: &str = ".aiplus/agent-team/cache-warnings.jsonl";
const MAX_ROLE_CACHE_BYTES: u64 = 5 * 1024 * 1024;
const MAX_PROJECT_CACHE_BYTES: u64 = 50 * 1024 * 1024;

/// Cached state payload for a role.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CachedState {
    pub data: String,
}

impl CachedState {
    pub fn new(data: String) -> Self {
        Self { data }
    }
}

/// Reason a cache entry was invalidated.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InvalidationReason {
    RoleDismissed,
    RoleRouteCalled,
    RoleIntegrateCompleted,
    GitStateChanged,
    CacheExplicitlyCleared,
    TtlExpired,
}

impl InvalidationReason {
    pub fn as_str(&self) -> &'static str {
        match self {
            InvalidationReason::RoleDismissed => "role_dismissed",
            InvalidationReason::RoleRouteCalled => "role_route_called",
            InvalidationReason::RoleIntegrateCompleted => "role_integrate_completed",
            InvalidationReason::GitStateChanged => "git_state_changed",
            InvalidationReason::CacheExplicitlyCleared => "cache_explicitly_cleared",
            InvalidationReason::TtlExpired => "ttl_expired",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CacheSource {
    Disabled,
    BypassedOwnerFacing,
    ColdStart,
    DiskWarm,
}

impl CacheSource {
    pub fn as_str(self) -> &'static str {
        match self {
            CacheSource::Disabled => "disabled",
            CacheSource::BypassedOwnerFacing => "bypassed_owner_facing",
            CacheSource::ColdStart => "cold_start",
            CacheSource::DiskWarm => "disk_warm",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DiskCacheWarningKind {
    Corrupt,
    ChecksumMismatch,
    Stale,
    RedactionBlocked,
    WriteFailed,
}

impl DiskCacheWarningKind {
    fn as_str(self) -> &'static str {
        match self {
            DiskCacheWarningKind::Corrupt => "corrupt",
            DiskCacheWarningKind::ChecksumMismatch => "checksum_mismatch",
            DiskCacheWarningKind::Stale => "stale",
            DiskCacheWarningKind::RedactionBlocked => "redaction_blocked",
            DiskCacheWarningKind::WriteFailed => "write_failed",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct DiskCacheConfig {
    pub cache: DiskCacheConfigSection,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct DiskCacheConfigSection {
    pub enable_disk: bool,
    pub enforce_ttl: bool,
}

impl Default for DiskCacheConfigSection {
    fn default() -> Self {
        Self {
            enable_disk: false,
            enforce_ttl: false,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DiskCacheSnapshot {
    pub schema_version: String,
    pub project: String,
    pub role: String,
    pub written_at_ms: u128,
    pub ttl_seconds: u64,
    pub persona: String,
    pub memory_bundle: String,
    pub workspace_head: String,
    pub workspace_dirty: bool,
    pub adapter_session_mapping_ref: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase", default)]
pub struct DiskCacheMeta {
    pub schema_version: String,
    pub project: String,
    pub roles: BTreeMap<String, DiskCacheRoleMeta>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DiskCacheRoleMeta {
    pub ttl_seconds: u64,
    pub written_at_ms: u128,
    pub last_used_at_ms: u128,
    pub cache_source: String,
    pub sha256: String,
    pub bytes: u64,
}

#[derive(Debug, Clone)]
pub struct DiskCacheStatus {
    pub enabled: bool,
    pub enforce_ttl: bool,
    pub project: String,
    pub project_dir: PathBuf,
    pub meta: Option<DiskCacheMeta>,
    pub sync_warning: Option<String>,
}

pub fn handle_cache_command(
    enable_disk: bool,
    disable_disk: bool,
    clear: bool,
    status: bool,
) -> Result<()> {
    let project_root = std::env::current_dir()?;
    let selected = [enable_disk, disable_disk, clear, status]
        .into_iter()
        .filter(|v| *v)
        .count();
    if selected == 0 {
        print_cache_status(&project_root)?;
        return Ok(());
    }
    if selected > 1 {
        return Err(anyhow!(
            "choose one cache action: --enable-disk, --disable-disk, --clear, or --status"
        ));
    }

    if enable_disk {
        write_disk_cache_enabled(&project_root, true)?;
        println!("disk_cache=enabled");
        println!("cache_dir={}", project_cache_dir(&project_root)?.display());
    } else if disable_disk {
        write_disk_cache_enabled(&project_root, false)?;
        println!("disk_cache=disabled");
        println!("existing_cache_retained=true");
    } else if clear {
        let dir = project_cache_dir(&project_root)?;
        if dir.exists() {
            fs::remove_dir_all(&dir).with_context(|| format!("remove {}", dir.display()))?;
        }
        println!("disk_cache_cleared=true");
        println!("cache_dir={}", dir.display());
    } else if status {
        print_cache_status(&project_root)?;
    }
    Ok(())
}

pub fn print_cache_status(project_root: &Path) -> Result<()> {
    let status = disk_cache_status(project_root)?;
    println!(
        "disk_cache={}",
        if status.enabled {
            "enabled"
        } else {
            "disabled"
        }
    );
    println!("enforce_ttl={}", status.enforce_ttl);
    println!("project={}", status.project);
    println!("cache_dir={}", status.project_dir.display());
    if let Some(meta) = status.meta {
        for (role, entry) in meta.roles {
            println!(
                "role={} cache_source={} ttl_seconds={} bytes={} last_used_at_ms={}",
                role, entry.cache_source, entry.ttl_seconds, entry.bytes, entry.last_used_at_ms
            );
        }
    }
    if let Some(warning) = status.sync_warning {
        println!("WARNING: {warning}");
    }
    Ok(())
}

pub fn disk_cache_status(project_root: &Path) -> Result<DiskCacheStatus> {
    let config = read_disk_cache_config(project_root)?;
    let enabled = config.cache.enable_disk;
    let enforce_ttl = config.cache.enforce_ttl;
    let project_dir = project_cache_dir(project_root)?;
    let project = project_basename(project_root);
    let meta = read_meta_if_exists(&project_dir)?;
    let sync_warning = cache_sync_warning(&cache_root_dir()?);
    Ok(DiskCacheStatus {
        enabled,
        enforce_ttl,
        project,
        project_dir,
        meta,
        sync_warning,
    })
}

pub fn disk_cache_enabled(project_root: &Path) -> Result<bool> {
    Ok(read_disk_cache_config(project_root)?.cache.enable_disk)
}

pub fn disk_cache_enforce_ttl(project_root: &Path) -> Result<bool> {
    Ok(read_disk_cache_config(project_root)?.cache.enforce_ttl)
}

fn read_disk_cache_config(project_root: &Path) -> Result<DiskCacheConfig> {
    let path = project_root.join(DISK_CACHE_CONFIG_PATH);
    if !path.exists() {
        return Ok(DiskCacheConfig::default());
    }
    let text = fs::read_to_string(&path).with_context(|| format!("read {}", path.display()))?;
    toml::from_str(&text).with_context(|| format!("parse {}", path.display()))
}

pub fn write_disk_cache_enabled(project_root: &Path, enabled: bool) -> Result<()> {
    let path = project_root.join(DISK_CACHE_CONFIG_PATH);
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let mut config = read_disk_cache_config(project_root).unwrap_or_default();
    config.cache.enable_disk = enabled;
    let text = format!(
        "[cache]\nenable_disk = {}\nenforce_ttl = {}\n",
        config.cache.enable_disk, config.cache.enforce_ttl
    );
    aiplus_core::write_file_atomic(&path, text.as_bytes())?;
    Ok(())
}

pub fn lookup_disk_snapshot(
    project_root: &Path,
    role: &str,
    ttl_seconds: u64,
) -> Result<CacheSource> {
    let cache_config = read_disk_cache_config(project_root)?;
    if !cache_config.cache.enable_disk {
        return Ok(CacheSource::Disabled);
    }
    if is_owner_facing_role(role) {
        remember_cache_source(
            project_root,
            role,
            ttl_seconds,
            CacheSource::BypassedOwnerFacing,
        )?;
        return Ok(CacheSource::BypassedOwnerFacing);
    }

    let project_dir = project_cache_dir(project_root)?;
    let snapshot_path = role_snapshot_path(&project_dir, role);
    let checksum_path = role_checksum_path(&project_dir, role);
    if !snapshot_path.exists() || !checksum_path.exists() {
        remember_cache_source(project_root, role, ttl_seconds, CacheSource::ColdStart)?;
        return Ok(CacheSource::ColdStart);
    }

    let bytes =
        fs::read(&snapshot_path).with_context(|| format!("read {}", snapshot_path.display()))?;
    let actual = sha256_hex(&bytes);
    let expected = fs::read_to_string(&checksum_path)
        .with_context(|| format!("read {}", checksum_path.display()))?
        .split_whitespace()
        .next()
        .unwrap_or("")
        .to_string();
    if expected != actual {
        record_cache_warning(
            project_root,
            role,
            DiskCacheWarningKind::ChecksumMismatch,
            "sha256 mismatch; cold-starting and replacing cache",
        );
        remember_cache_source(project_root, role, ttl_seconds, CacheSource::ColdStart)?;
        return Ok(CacheSource::ColdStart);
    }

    let snapshot: DiskCacheSnapshot = match serde_cbor::from_slice(&bytes) {
        Ok(snapshot) => snapshot,
        Err(e) => {
            record_cache_warning(
                project_root,
                role,
                DiskCacheWarningKind::Corrupt,
                &format!("snapshot decode failed: {e}; cold-starting and replacing cache"),
            );
            remember_cache_source(project_root, role, ttl_seconds, CacheSource::ColdStart)?;
            return Ok(CacheSource::ColdStart);
        }
    };

    let now = epoch_millis();
    let ttl_expired = disk_cache_ttl_expired_for_role(project_root, role, ttl_seconds)?
        .unwrap_or_else(|| {
            now.saturating_sub(snapshot.written_at_ms) > (ttl_seconds as u128 * 1000)
        });
    if cache_config.cache.enforce_ttl && ttl_expired {
        remember_cache_source(project_root, role, ttl_seconds, CacheSource::ColdStart)?;
        return Ok(CacheSource::ColdStart);
    }
    if role_config_newer_than(project_root, role, snapshot.written_at_ms)? {
        record_cache_warning(
            project_root,
            role,
            DiskCacheWarningKind::Stale,
            "role config/persona changed after cache write; cold-starting",
        );
        remember_cache_source(project_root, role, ttl_seconds, CacheSource::ColdStart)?;
        return Ok(CacheSource::ColdStart);
    }

    update_role_meta(
        &project_dir,
        role,
        ttl_seconds,
        CacheSource::DiskWarm,
        actual,
        bytes.len() as u64,
        snapshot.written_at_ms,
    )?;
    Ok(CacheSource::DiskWarm)
}

pub fn disk_cache_ttl_expired_for_role(
    project_root: &Path,
    role: &str,
    ttl_seconds: u64,
) -> Result<Option<bool>> {
    let config = read_disk_cache_config(project_root)?;
    if !config.cache.enable_disk || !config.cache.enforce_ttl || is_owner_facing_role(role) {
        return Ok(Some(false));
    }
    let project_dir = project_cache_dir(project_root)?;
    let meta = match read_meta_if_exists(&project_dir)? {
        Some(meta) => meta,
        None => return Ok(Some(false)),
    };
    let entry = match meta.roles.get(role) {
        Some(entry) => entry,
        None => return Ok(Some(false)),
    };
    let snapshot_path = role_snapshot_path(&project_dir, role);
    let checksum_path = role_checksum_path(&project_dir, role);
    if !snapshot_path.exists() || !checksum_path.exists() {
        return Ok(Some(false));
    }
    let ttl_ms = ttl_seconds as u128 * 1000;
    Ok(Some(
        epoch_millis().saturating_sub(entry.last_used_at_ms) > ttl_ms,
    ))
}

pub fn current_epoch_millis() -> u128 {
    epoch_millis()
}

pub fn write_disk_snapshot(
    project_root: &Path,
    role: &str,
    config: &AgentConfig,
    cache_source: CacheSource,
) -> Result<()> {
    if !disk_cache_enabled(project_root)? || is_owner_facing_role(role) {
        return Ok(());
    }

    let ttl_seconds = config.warm_bench_ttl_seconds;
    let project = project_basename(project_root);
    let project_dir = project_cache_dir(project_root)?;
    fs::create_dir_all(&project_dir)?;

    let mut snapshot = DiskCacheSnapshot {
        schema_version: DISK_CACHE_SCHEMA_VERSION.to_string(),
        project,
        role: role.to_string(),
        written_at_ms: epoch_millis(),
        ttl_seconds,
        persona: redact_for_cache(&persona_bundle(project_root, role, config)?),
        memory_bundle: redact_for_cache(&memory_bundle(project_root, role)?),
        workspace_head: workspace_head(project_root),
        workspace_dirty: workspace_dirty(project_root),
        adapter_session_mapping_ref: ".aiplus/agent-team/sessions".to_string(),
    };

    let redaction_probe = serde_json::to_string(&snapshot)?;
    if let Err(e) = aiplus_core::reject_sensitive_memory_text(&redaction_probe) {
        record_cache_warning(
            project_root,
            role,
            DiskCacheWarningKind::RedactionBlocked,
            &format!("redaction pipeline blocked snapshot: {e}"),
        );
        snapshot.persona = "[REDACTED_BY_AIPLUS_CACHE]".to_string();
        snapshot.memory_bundle = "[REDACTED_BY_AIPLUS_CACHE]".to_string();
    }

    let bytes = serde_cbor::to_vec(&snapshot)?;
    if bytes.len() as u64 > MAX_ROLE_CACHE_BYTES {
        record_cache_warning(
            project_root,
            role,
            DiskCacheWarningKind::WriteFailed,
            "snapshot exceeds 5MB role cache cap",
        );
        return Ok(());
    }
    let sha = sha256_hex(&bytes);
    let snapshot_path = role_snapshot_path(&project_dir, role);
    let checksum_path = role_checksum_path(&project_dir, role);
    aiplus_core::write_file_atomic(&snapshot_path, &bytes)?;
    aiplus_core::write_file_atomic(&checksum_path, sha.as_bytes())?;
    update_role_meta(
        &project_dir,
        role,
        ttl_seconds,
        cache_source,
        sha,
        bytes.len() as u64,
        snapshot.written_at_ms,
    )?;
    enforce_project_size_cap(&project_dir)?;
    Ok(())
}

pub fn cache_warnings(project_root: &Path) -> Vec<String> {
    let path = project_root.join(DISK_CACHE_WARNING_PATH);
    let mut warnings = Vec::new();
    if path.exists() {
        if let Ok(text) = fs::read_to_string(&path) {
            for line in text.lines().rev().take(10) {
                if !line.trim().is_empty() {
                    warnings.push(line.to_string());
                }
            }
        }
    }
    if let Ok(root) = cache_root_dir() {
        if let Some(warning) = cache_sync_warning(&root) {
            warnings.push(warning);
        }
    }
    warnings
}

fn read_meta_if_exists(project_dir: &Path) -> Result<Option<DiskCacheMeta>> {
    let path = project_dir.join("_cache_meta.json");
    if !path.exists() {
        return Ok(None);
    }
    let text = fs::read_to_string(&path).with_context(|| format!("read {}", path.display()))?;
    Ok(Some(serde_json::from_str(&text).unwrap_or_default()))
}

fn remember_cache_source(
    project_root: &Path,
    role: &str,
    ttl_seconds: u64,
    source: CacheSource,
) -> Result<()> {
    let project_dir = project_cache_dir(project_root)?;
    let mut meta = read_meta_if_exists(&project_dir)?.unwrap_or_else(|| DiskCacheMeta {
        schema_version: DISK_CACHE_SCHEMA_VERSION.to_string(),
        project: project_basename(project_root),
        roles: BTreeMap::new(),
    });
    let now = epoch_millis();
    let entry = meta
        .roles
        .entry(role.to_string())
        .or_insert(DiskCacheRoleMeta {
            ttl_seconds,
            written_at_ms: now,
            last_used_at_ms: now,
            cache_source: source.as_str().to_string(),
            sha256: String::new(),
            bytes: 0,
        });
    entry.ttl_seconds = ttl_seconds;
    entry.last_used_at_ms = now;
    entry.cache_source = source.as_str().to_string();
    write_meta(&project_dir, &meta)
}

fn update_role_meta(
    project_dir: &Path,
    role: &str,
    ttl_seconds: u64,
    source: CacheSource,
    sha256: String,
    bytes: u64,
    written_at_ms: u128,
) -> Result<()> {
    let mut meta = read_meta_if_exists(project_dir)?.unwrap_or_else(|| DiskCacheMeta {
        schema_version: DISK_CACHE_SCHEMA_VERSION.to_string(),
        project: project_dir
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("unknown")
            .to_string(),
        roles: BTreeMap::new(),
    });
    meta.schema_version = DISK_CACHE_SCHEMA_VERSION.to_string();
    meta.roles.insert(
        role.to_string(),
        DiskCacheRoleMeta {
            ttl_seconds,
            written_at_ms,
            last_used_at_ms: epoch_millis(),
            cache_source: source.as_str().to_string(),
            sha256,
            bytes,
        },
    );
    write_meta(project_dir, &meta)
}

fn write_meta(project_dir: &Path, meta: &DiskCacheMeta) -> Result<()> {
    fs::create_dir_all(project_dir)?;
    let path = project_dir.join("_cache_meta.json");
    let text = serde_json::to_string_pretty(meta)?;
    aiplus_core::write_file_atomic(&path, text.as_bytes())?;
    Ok(())
}

fn project_cache_dir(project_root: &Path) -> Result<PathBuf> {
    Ok(cache_root_dir()?.join(project_basename(project_root)))
}

fn cache_root_dir() -> Result<PathBuf> {
    if let Ok(root) = std::env::var("AIPLUS_AGENT_CACHE_ROOT") {
        return Ok(PathBuf::from(root));
    }
    Ok(home_dir()?.join(".cache").join("aiplus-agent-team"))
}

fn home_dir() -> Result<PathBuf> {
    std::env::var_os("HOME")
        .map(PathBuf::from)
        .ok_or_else(|| anyhow!("HOME is not set"))
}

fn project_basename(project_root: &Path) -> String {
    project_root
        .file_name()
        .and_then(|n| n.to_str())
        .filter(|s| !s.trim().is_empty())
        .unwrap_or("project")
        .to_string()
}

fn role_snapshot_path(project_dir: &Path, role: &str) -> PathBuf {
    project_dir.join(format!("{role}.cbor"))
}

fn role_checksum_path(project_dir: &Path, role: &str) -> PathBuf {
    project_dir.join(format!("{role}.cbor.sha256"))
}

fn sha256_hex(bytes: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    hex::encode(hasher.finalize())
}

fn epoch_millis() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis()
}

fn is_owner_facing_role(role: &str) -> bool {
    matches!(role, "advisor" | "ceo")
}

fn persona_bundle(project_root: &Path, role: &str, config: &AgentConfig) -> Result<String> {
    let config_path = project_root
        .join(".aiplus")
        .join("agents")
        .join(format!("{role}.toml"));
    let config_text = fs::read_to_string(&config_path).unwrap_or_default();
    let persona_text = config
        .persona
        .as_ref()
        .map(|persona| persona.system_prompt_file.trim())
        .filter(|path| !path.is_empty())
        .and_then(|rel| {
            fs::read_to_string(project_root.join(".aiplus").join("agents").join(rel)).ok()
        })
        .unwrap_or_default();
    Ok(format!(
        "role={role}\nconfig_toml:\n{config_text}\npersona:\n{persona_text}"
    ))
}

fn memory_bundle(project_root: &Path, role: &str) -> Result<String> {
    let dir = project_root.join(".aiplus").join("agent-memory").join(role);
    if !dir.exists() {
        return Ok("memory_source=missing".to_string());
    }
    let mut chunks = Vec::new();
    for entry in fs::read_dir(&dir)? {
        let entry = entry?;
        let path = entry.path();
        if path.is_file() {
            if let Ok(text) = fs::read_to_string(&path) {
                let name = path
                    .file_name()
                    .and_then(|n| n.to_str())
                    .unwrap_or("memory");
                chunks.push(format!("file={name}\n{}", redact_for_cache(&text)));
            }
        }
    }
    if chunks.is_empty() {
        Ok("memory_source=empty".to_string())
    } else {
        Ok(chunks.join("\n---\n"))
    }
}

fn redact_for_cache(text: &str) -> String {
    text.lines()
        .map(|line| {
            if aiplus_core::reject_sensitive_memory_text(line).is_err() {
                "[REDACTED_BY_AIPLUS_CACHE]"
            } else {
                line
            }
        })
        .collect::<Vec<_>>()
        .join("\n")
}

fn workspace_head(project_root: &Path) -> String {
    std::process::Command::new("git")
        .args(["-C"])
        .arg(project_root)
        .args(["rev-parse", "HEAD"])
        .output()
        .ok()
        .filter(|output| output.status.success())
        .map(|output| String::from_utf8_lossy(&output.stdout).trim().to_string())
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| "unknown".to_string())
}

fn workspace_dirty(project_root: &Path) -> bool {
    std::process::Command::new("git")
        .args(["-C"])
        .arg(project_root)
        .args(["status", "--porcelain"])
        .output()
        .ok()
        .map(|output| !String::from_utf8_lossy(&output.stdout).trim().is_empty())
        .unwrap_or(false)
}

fn role_config_newer_than(project_root: &Path, role: &str, written_at_ms: u128) -> Result<bool> {
    let mut paths = vec![
        project_root
            .join(".aiplus")
            .join("agents")
            .join(format!("{role}.toml")),
        project_root
            .join(".aiplus")
            .join("agents")
            .join("personas")
            .join(format!("{role}.md")),
    ];
    paths.retain(|path| path.exists());
    for path in paths {
        let modified = fs::metadata(&path)?
            .modified()?
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis();
        if modified > written_at_ms {
            return Ok(true);
        }
    }
    Ok(false)
}

fn record_cache_warning(project_root: &Path, role: &str, kind: DiskCacheWarningKind, detail: &str) {
    let path = project_root.join(DISK_CACHE_WARNING_PATH);
    if let Some(parent) = path.parent() {
        let _ = fs::create_dir_all(parent);
    }
    let line = serde_json::json!({
        "schemaVersion": DISK_CACHE_SCHEMA_VERSION,
        "event": "disk_cache_warning",
        "kind": kind.as_str(),
        "role": role,
        "detail": detail,
        "timestamp": aiplus_core::timestamp(),
        "secretValues": "none"
    });
    let _ = aiplus_core::append_jsonl_atomic(&path, &line.to_string());
}

fn cache_sync_warning(cache_root: &Path) -> Option<String> {
    let lower = cache_root.to_string_lossy().to_ascii_lowercase();
    ["icloud", "dropbox", "onedrive"]
        .iter()
        .any(|needle| lower.contains(needle))
        .then(|| {
            format!(
                "disk cache root appears to live under a sync folder: {}; recommend `aiplus agent cache --clear` and a non-synced HOME/cache root",
                cache_root.display()
            )
        })
}

fn enforce_project_size_cap(project_dir: &Path) -> Result<()> {
    let mut files = Vec::new();
    let mut total = 0_u64;
    if !project_dir.exists() {
        return Ok(());
    }
    for entry in fs::read_dir(project_dir)? {
        let entry = entry?;
        let path = entry.path();
        if path.extension().and_then(|e| e.to_str()) == Some("cbor") {
            let meta = fs::metadata(&path)?;
            let modified = meta.modified().unwrap_or(UNIX_EPOCH);
            let size = meta.len();
            total += size;
            files.push((path, modified, size));
        }
    }
    if total <= MAX_PROJECT_CACHE_BYTES {
        return Ok(());
    }
    files.sort_by_key(|(_, modified, _)| *modified);
    for (path, _, size) in files {
        let _ = fs::remove_file(&path);
        let _ = fs::remove_file(path.with_extension("cbor.sha256"));
        total = total.saturating_sub(size);
        if total <= MAX_PROJECT_CACHE_BYTES {
            break;
        }
    }
    Ok(())
}

/// In-process warm-bench cache for AiPlus Agent Team v0.1.
///
/// DESIGN.md §6:
/// - std-only HashMap + manual TTL sweep (no lru, moka, cached, dashmap)
/// - TTL values from each role's TOML `[agent] warm_bench_ttl_seconds`
/// - 6 invalidation triggers as discrete events (no debounce)
pub struct WarmBenchCache {
    inner: Arc<Mutex<HashMap<RoleId, (Instant, CachedState)>>>,
    shutdown: Arc<(Mutex<bool>, Condvar)>,
    ttl: Duration,
    bg_thread: Option<JoinHandle<()>>,
    audit_log_path: Option<PathBuf>,
}

impl WarmBenchCache {
    /// Create a new cache with the given TTL.
    /// Spawns a background thread that periodically purges expired entries.
    pub fn new(ttl_seconds: u64) -> Self {
        let inner = Arc::new(Mutex::new(HashMap::new()));
        let shutdown = Arc::new((Mutex::new(false), Condvar::new()));
        let ttl = Duration::from_secs(ttl_seconds);
        let audit_log_path = Self::default_audit_log_path();

        let bg_thread = {
            let inner = Arc::clone(&inner);
            let shutdown = Arc::clone(&shutdown);
            let audit_log_path = audit_log_path.clone();
            let ttl = ttl;
            Some(thread::spawn(move || {
                Self::background_purge_loop(inner, shutdown, ttl, audit_log_path);
            }))
        };

        Self {
            inner,
            shutdown,
            ttl,
            bg_thread,
            audit_log_path,
        }
    }

    /// Create a new cache without a background thread (for testing).
    #[cfg(test)]
    pub fn new_without_bg(ttl_seconds: u64) -> Self {
        let inner = Arc::new(Mutex::new(HashMap::new()));
        let shutdown = Arc::new((Mutex::new(false), Condvar::new()));
        let ttl = Duration::from_secs(ttl_seconds);

        Self {
            inner,
            shutdown,
            ttl,
            bg_thread: None,
            audit_log_path: None,
        }
    }

    fn default_audit_log_path() -> Option<PathBuf> {
        std::env::current_dir().ok().map(|root| {
            root.join(".aiplus")
                .join("agent-team")
                .join("audit-trail")
                .join("cache-invalidations.log")
        })
    }

    fn log_invalidation(role: Option<&str>, reason: InvalidationReason, path: Option<&Path>) {
        if let Some(path) = path {
            if let Some(parent) = path.parent() {
                let _ = fs::create_dir_all(parent);
            }
            let line = serde_json::json!({
                "schemaVersion": "0.1.0",
                "event": "cache_invalidation",
                "reason": reason.as_str(),
                "role": role.unwrap_or("*"),
                "timestamp": aiplus_core::timestamp(),
                "secretValues": "none"
            });
            let _ = aiplus_core::append_jsonl_atomic(path, &line.to_string());
        }
    }

    fn background_purge_loop(
        inner: Arc<Mutex<HashMap<RoleId, (Instant, CachedState)>>>,
        shutdown: Arc<(Mutex<bool>, Condvar)>,
        ttl: Duration,
        audit_log_path: Option<PathBuf>,
    ) {
        let (lock, cvar) = &*shutdown;
        let interval = if ttl.is_zero() {
            Duration::from_millis(100)
        } else {
            ttl / 2
        };
        loop {
            let guard = lock.lock().unwrap();
            let result = cvar.wait_timeout(guard, interval).unwrap();
            let should_shutdown = *result.0;
            drop(result);

            if should_shutdown {
                break;
            }

            Self::purge_expired(&inner, ttl, audit_log_path.as_deref());
        }
    }

    fn purge_expired(
        inner: &Arc<Mutex<HashMap<RoleId, (Instant, CachedState)>>>,
        ttl: Duration,
        audit_log_path: Option<&Path>,
    ) {
        let now = Instant::now();
        let mut map = inner.lock().unwrap();
        let expired: Vec<String> = map
            .iter()
            .filter(|(_, (instant, _))| now.duration_since(*instant) > ttl)
            .map(|(role, _)| role.clone())
            .collect();

        for role in expired {
            map.remove(&role);
            drop(map);
            Self::log_invalidation(Some(&role), InvalidationReason::TtlExpired, audit_log_path);
            map = inner.lock().unwrap();
        }
    }

    /// Look up a cached state for `role_id`. Returns `Some(state)` if the entry
    /// exists and has not expired. Returns `None` and evicts if expired.
    pub fn get(&self, role_id: &str) -> Option<CachedState> {
        let now = Instant::now();
        let mut map = self.inner.lock().unwrap();

        if let Some((instant, state)) = map.get(role_id) {
            if now.duration_since(*instant) <= self.ttl {
                return Some(state.clone());
            }
            // Expired — evict on access
            map.remove(role_id);
            drop(map);
            Self::log_invalidation(
                Some(role_id),
                InvalidationReason::TtlExpired,
                self.audit_log_path.as_deref(),
            );
            return None;
        }

        None
    }

    /// Insert or replace the cached state for `role_id`.
    pub fn set(&self, role_id: RoleId, state: CachedState) {
        let mut map = self.inner.lock().unwrap();
        map.insert(role_id, (Instant::now(), state));
    }

    /// Remove the entry for a specific role, logging the reason.
    pub fn invalidate(&self, role_id: &str, reason: InvalidationReason) {
        let mut map = self.inner.lock().unwrap();
        if map.remove(role_id).is_some() {
            drop(map);
            Self::log_invalidation(Some(role_id), reason, self.audit_log_path.as_deref());
        }
    }

    /// Remove all entries, logging the reason.
    pub fn clear(&self, reason: InvalidationReason) {
        let mut map = self.inner.lock().unwrap();
        if !map.is_empty() {
            map.clear();
            drop(map);
            Self::log_invalidation(None, reason, self.audit_log_path.as_deref());
        }
    }

    /// Backward-compatible invalidate without explicit reason.
    pub fn invalidate_role(&self, role_id: &str) {
        self.invalidate(role_id, InvalidationReason::CacheExplicitlyCleared);
    }

    /// Backward-compatible clear without explicit reason.
    pub fn invalidate_all(&self) {
        self.clear(InvalidationReason::CacheExplicitlyCleared);
    }

    /// Explicitly sweep expired entries. Useful for testing or diagnostics.
    pub fn sweep_expired(&self) {
        Self::purge_expired(&self.inner, self.ttl, self.audit_log_path.as_deref());
    }

    /// Signal the background thread to shut down and wait for it.
    pub fn shutdown(mut self) {
        let (lock, cvar) = &*self.shutdown;
        {
            let mut guard = lock.lock().unwrap();
            *guard = true;
        }
        cvar.notify_all();
        if let Some(handle) = self.bg_thread.take() {
            let _ = handle.join();
        }
    }
}

impl Drop for WarmBenchCache {
    fn drop(&mut self) {
        let (lock, cvar) = &*self.shutdown;
        {
            let mut guard = lock.lock().unwrap();
            *guard = true;
        }
        cvar.notify_all();
        if let Some(handle) = self.bg_thread.take() {
            let _ = handle.join();
        }
    }
}

static CACHE: OnceLock<Mutex<WarmBenchCache>> = OnceLock::new();

/// Access the process-global warm-bench cache.
pub fn global_cache() -> &'static Mutex<WarmBenchCache> {
    CACHE.get_or_init(|| Mutex::new(WarmBenchCache::new(1800)))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::thread;
    use std::time::Duration;

    #[test]
    fn cache_hit_returns_correct_state() {
        let cache = WarmBenchCache::new_without_bg(3600);
        let state = CachedState::new(r#"{"key":"value","nested":{"a":1}}"#.to_string());
        cache.set("test".to_string(), state.clone());
        let result = cache.get("test");
        assert_eq!(result, Some(state));
    }

    #[test]
    fn cache_miss_after_ttl_expiration() {
        let cache = WarmBenchCache::new_without_bg(1);
        cache.set("expiring".to_string(), CachedState::new("data".to_string()));
        assert!(cache.get("expiring").is_some());
        thread::sleep(Duration::from_secs(2));
        assert!(cache.get("expiring").is_none());
    }

    #[test]
    fn invalidate_removes_entry() {
        let cache = WarmBenchCache::new_without_bg(3600);
        cache.set(
            "pm".to_string(),
            CachedState::new(r#"{"state":"planning"}"#.to_string()),
        );
        assert!(cache.get("pm").is_some());
        cache.invalidate("pm", InvalidationReason::RoleDismissed);
        assert!(cache.get("pm").is_none());
    }

    #[test]
    fn clear_removes_all_entries() {
        let cache = WarmBenchCache::new_without_bg(3600);
        cache.set(
            "ceo".to_string(),
            CachedState::new(r#"{"state":"reviewing"}"#.to_string()),
        );
        cache.set(
            "architect".to_string(),
            CachedState::new(r#"{"state":"designing"}"#.to_string()),
        );
        cache.clear(InvalidationReason::CacheExplicitlyCleared);
        assert!(cache.get("ceo").is_none());
        assert!(cache.get("architect").is_none());
    }

    #[test]
    fn concurrent_access_is_safe() {
        let cache = Arc::new(WarmBenchCache::new_without_bg(3600));
        let mut handles = vec![];

        // Writers
        for i in 0..5 {
            let cache = Arc::clone(&cache);
            handles.push(thread::spawn(move || {
                cache.set(
                    format!("role-{}", i),
                    CachedState::new(format!("state-{}", i)),
                );
            }));
        }

        // Readers
        for i in 0..5 {
            let cache = Arc::clone(&cache);
            handles.push(thread::spawn(move || {
                for _ in 0..100 {
                    let _ = cache.get(&format!("role-{}", i));
                }
            }));
        }

        // Invalidators
        for i in 0..5 {
            let cache = Arc::clone(&cache);
            handles.push(thread::spawn(move || {
                cache.invalidate(
                    &format!("role-{}", i),
                    InvalidationReason::CacheExplicitlyCleared,
                );
            }));
        }

        for handle in handles {
            handle.join().unwrap();
        }
    }

    #[test]
    fn background_thread_purges_expired() {
        let cache = WarmBenchCache::new(0); // TTL=0, bg thread will purge quickly
        cache.set(
            "test-role".to_string(),
            CachedState::new("data".to_string()),
        );

        // Wait for background thread to run at least once
        thread::sleep(Duration::from_millis(300));

        // Should be purged by background thread
        assert!(cache.get("test-role").is_none());
    }

    #[test]
    fn sweep_expired_removes_stale_entries() {
        let cache = WarmBenchCache::new_without_bg(0);
        cache.set(
            "engineer-a".to_string(),
            CachedState::new(r#"{"state":"coding"}"#.to_string()),
        );
        thread::sleep(Duration::from_millis(10));
        cache.set(
            "engineer-b".to_string(),
            CachedState::new(r#"{"state":"reviewing"}"#.to_string()),
        );
        // With TTL=0, everything is expired immediately
        cache.sweep_expired();
        assert!(cache.get("engineer-a").is_none());
        assert!(cache.get("engineer-b").is_none());
    }

    #[test]
    fn invalidation_reason_is_tracked() {
        let cache = WarmBenchCache::new_without_bg(3600);
        cache.set(
            "reviewer".to_string(),
            CachedState::new(r#"{"state":"idle"}"#.to_string()),
        );
        cache.invalidate("reviewer", InvalidationReason::RoleRouteCalled);
        // Should be gone regardless of reason
        assert!(cache.get("reviewer").is_none());
    }
}
