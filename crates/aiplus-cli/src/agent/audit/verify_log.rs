use anyhow::{anyhow, Context, Result};
use serde_json::{Map, Value};
use sha2::{Digest, Sha256};
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::{Mutex, OnceLock};

const DISPATCH_LOG_PATH: &str = ".aiplus/agents/dispatch-log.jsonl";
static DISPATCH_LOG_CHAIN_LOCK: OnceLock<Mutex<()>> = OnceLock::new();

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VerifyLogReport {
    pub status: VerifyLogStatus,
    pub checked_lines: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum VerifyLogStatus {
    Pass,
    Fail { line: usize, reason: String },
}

impl VerifyLogReport {
    pub fn doctor_status(&self) -> String {
        match &self.status {
            VerifyLogStatus::Pass => "valid".to_string(),
            VerifyLogStatus::Fail { line, .. } => format!("BROKEN line={line}"),
        }
    }
}

pub fn handle_verify_log() -> Result<()> {
    let project_root = std::env::current_dir()?;
    let report = verify_dispatch_log(&project_root)?;
    match report.status {
        VerifyLogStatus::Pass => {
            println!("VERIFY_LOG=PASS checked_lines={}", report.checked_lines);
            Ok(())
        }
        VerifyLogStatus::Fail { line, reason } => {
            println!("VERIFY_LOG=FAIL line={line} reason={reason}");
            Err(anyhow!(
                "dispatch log verification failed at line {line}: {reason}"
            ))
        }
    }
}

pub fn verify_dispatch_log(project_root: &Path) -> Result<VerifyLogReport> {
    verify_dispatch_log_path(&project_root.join(DISPATCH_LOG_PATH))
}

pub fn verify_dispatch_log_path(path: &Path) -> Result<VerifyLogReport> {
    if !path.exists() {
        return Ok(VerifyLogReport {
            status: VerifyLogStatus::Pass,
            checked_lines: 0,
        });
    }

    let text = fs::read_to_string(path).with_context(|| format!("read {}", path.display()))?;
    let mut prior_hash: Option<String> = None;
    let mut chain_started = false;
    let mut checked_lines = 0;

    for (idx, line) in text.lines().enumerate() {
        let line_number = idx + 1;
        if line.trim().is_empty() {
            continue;
        }
        let value: Value = match serde_json::from_str(line) {
            Ok(value) => value,
            Err(_) if !chain_started => continue,
            Err(_) => {
                return Ok(fail(line_number, "malformed_entry"));
            }
        };
        let has_chain_marker = value.get("entry_hash").is_some()
            || value.get("prev_hash").is_some()
            || value.get("genesis").and_then(Value::as_bool) == Some(true);
        if !has_chain_marker {
            if chain_started {
                return Ok(fail(line_number, "missing_chain_fields"));
            }
            continue;
        }

        chain_started = true;
        checked_lines += 1;
        let Some(entry_hash) = value.get("entry_hash").and_then(Value::as_str) else {
            return Ok(fail(line_number, "missing_entry_hash"));
        };
        let computed = entry_hash_for_value(&value)?;
        if entry_hash != computed {
            return Ok(fail(line_number, "entry_hash_mismatch"));
        }

        let genesis = value.get("genesis").and_then(Value::as_bool) == Some(true);
        match (&prior_hash, genesis) {
            (None, true) => {}
            (None, false) => return Ok(fail(line_number, "missing_genesis")),
            (Some(_), true) => return Ok(fail(line_number, "unexpected_genesis")),
            (Some(expected), false) => {
                let Some(prev_hash) = value.get("prev_hash").and_then(Value::as_str) else {
                    return Ok(fail(line_number, "missing_prev_hash"));
                };
                if prev_hash != expected {
                    return Ok(fail(line_number, "hash_mismatch"));
                }
            }
        }
        prior_hash = Some(entry_hash.to_string());
    }

    Ok(VerifyLogReport {
        status: VerifyLogStatus::Pass,
        checked_lines,
    })
}

fn fail(line: usize, reason: &str) -> VerifyLogReport {
    VerifyLogReport {
        status: VerifyLogStatus::Fail {
            line,
            reason: reason.to_string(),
        },
        checked_lines: 0,
    }
}

pub fn append_chained_jsonl_value(path: &Path, value: &mut Value) -> Result<()> {
    let _guard = DISPATCH_LOG_CHAIN_LOCK
        .get_or_init(|| Mutex::new(()))
        .lock()
        .map_err(|_| anyhow!("dispatch-log hash-chain lock poisoned"))?;
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    value_remove_chain_fields(value);
    let prior_hash = last_chained_entry_hash(path)?;
    match prior_hash {
        Some(hash) => set_object_field(value, "prev_hash", Value::String(hash))?,
        None => set_object_field(value, "genesis", Value::Bool(true))?,
    }
    let entry_hash = entry_hash_for_value(value)?;
    set_object_field(value, "entry_hash", Value::String(entry_hash))?;
    aiplus_core::append_jsonl_atomic(path, &canonical_json(value))?;
    Ok(())
}

fn last_chained_entry_hash(path: &Path) -> Result<Option<String>> {
    if !path.exists() {
        return Ok(None);
    }
    let text = fs::read_to_string(path).with_context(|| format!("read {}", path.display()))?;
    for line in text.lines().rev() {
        if line.trim().is_empty() {
            continue;
        }
        let Ok(value) = serde_json::from_str::<Value>(line) else {
            continue;
        };
        if let Some(hash) = value.get("entry_hash").and_then(Value::as_str) {
            return Ok(Some(hash.to_string()));
        }
    }
    Ok(None)
}

fn value_remove_chain_fields(value: &mut Value) {
    if let Some(object) = value.as_object_mut() {
        object.remove("genesis");
        object.remove("prev_hash");
        object.remove("entry_hash");
    }
}

pub fn entry_hash_for_value(value: &Value) -> Result<String> {
    let mut hash_value = value.clone();
    if let Some(object) = hash_value.as_object_mut() {
        object.remove("entry_hash");
    }
    Ok(sha256_hex(canonical_json(&hash_value).as_bytes()))
}

pub fn canonical_json(value: &Value) -> String {
    match value {
        Value::Null | Value::Bool(_) | Value::Number(_) | Value::String(_) => {
            serde_json::to_string(value).unwrap_or_else(|_| "null".to_string())
        }
        Value::Array(items) => {
            let body = items
                .iter()
                .map(canonical_json)
                .collect::<Vec<_>>()
                .join(",");
            format!("[{body}]")
        }
        Value::Object(object) => canonical_object(object),
    }
}

fn canonical_object(object: &Map<String, Value>) -> String {
    let mut keys = object.keys().collect::<Vec<_>>();
    keys.sort();
    let body = keys
        .into_iter()
        .map(|key| {
            let key_json = serde_json::to_string(key).unwrap_or_else(|_| "\"\"".to_string());
            let value_json = canonical_json(&object[key]);
            format!("{key_json}:{value_json}")
        })
        .collect::<Vec<_>>()
        .join(",");
    format!("{{{body}}}")
}

fn set_object_field(value: &mut Value, key: &str, field_value: Value) -> Result<()> {
    let Some(object) = value.as_object_mut() else {
        return Err(anyhow!("dispatch-log chain value must be a JSON object"));
    };
    object.insert(key.to_string(), field_value);
    Ok(())
}

fn sha256_hex(bytes: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    hex::encode(hasher.finalize())
}

#[allow(dead_code)]
pub fn dispatch_log_path(project_root: &Path) -> PathBuf {
    project_root.join(DISPATCH_LOG_PATH)
}
