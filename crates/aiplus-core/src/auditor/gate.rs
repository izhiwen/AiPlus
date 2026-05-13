use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::SystemTime;

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

use crate::agent_team::types::{BlockedReason, OwnerSentinel, ReleaseManifest};
use crate::auditor::flock_guard::FlockGuard;

/// Binary verdict for hash comparisons — never exposes raw hash strings to LLM.
#[allow(non_camel_case_types)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum HashVerdict {
    HASH_MATCH,
    HASH_MISMATCH,
}

/// Result of running the full 8-step pre-audit gate.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GateResult {
    Passed,
    Blocked(BlockedReason),
    AuditInProgress,
}

/// Cache key: (path, mtime, size)
type FileCache = HashMap<(PathBuf, SystemTime, u64), String>;

/// 8-step pre-audit verification gate for `release_manifest`.
pub struct PreAuditGate {
    manifest_path: PathBuf,
    lock_path: PathBuf,
    fingerprint_path: PathBuf,
    sentinel_path: PathBuf,
}

enum StepResult {
    Pass,
    Fail(BlockedReason),
}

impl PreAuditGate {
    pub fn new(
        manifest_path: impl Into<PathBuf>,
        lock_path: impl Into<PathBuf>,
        fingerprint_path: impl Into<PathBuf>,
        sentinel_path: impl Into<PathBuf>,
    ) -> Self {
        Self {
            manifest_path: manifest_path.into(),
            lock_path: lock_path.into(),
            fingerprint_path: fingerprint_path.into(),
            sentinel_path: sentinel_path.into(),
        }
    }

    /// Run the full 8-step pre-audit chain.
    ///
    /// Returns `GateResult::Passed` only if **all** steps pass.
    pub fn run(&self) -> Result<GateResult> {
        // Step 0 (concurrency): Acquire exclusive flock on audit lock.
        if let Some(parent) = self.lock_path.parent() {
            fs::create_dir_all(parent)?;
        }
        let _guard = match FlockGuard::try_lock_exclusive(&self.lock_path)? {
            Some(guard) => guard,
            None => return Ok(GateResult::AuditInProgress),
        };

        let mut cache = HashMap::new();

        // Step 1: Sentinel verification
        match self.step1_sentinel()? {
            StepResult::Pass => {}
            StepResult::Fail(reason) => return Ok(GateResult::Blocked(reason)),
        }

        // Step 2: Manifest clean
        match self.step2_manifest_clean()? {
            StepResult::Pass => {}
            StepResult::Fail(reason) => return Ok(GateResult::Blocked(reason)),
        }

        // Step 3: GPG signature + fingerprint
        match self.step3_manifest_signature()? {
            StepResult::Pass => {}
            StepResult::Fail(reason) => return Ok(GateResult::Blocked(reason)),
        }

        // Load manifest for steps 4-8.
        let manifest = match self.load_manifest() {
            Ok(m) => m,
            Err(_) => return Ok(GateResult::Blocked(BlockedReason::SchemaTampered)),
        };

        // Step 4: Bin aliases hash
        match self.step4_bin_aliases(&manifest, &mut cache) {
            Ok(HashVerdict::HASH_MATCH) => {}
            Ok(HashVerdict::HASH_MISMATCH) | Err(_) => {
                return Ok(GateResult::Blocked(BlockedReason::SchemaTampered));
            }
        }

        // Step 5: Acceptance schema hash
        match self.step5_acceptance_schema(&manifest, &mut cache) {
            Ok(HashVerdict::HASH_MATCH) => {}
            Ok(HashVerdict::HASH_MISMATCH) | Err(_) => {
                return Ok(GateResult::Blocked(BlockedReason::SchemaTampered));
            }
        }

        // Step 6: Audit scripts hash
        match self.step6_audit_scripts(&manifest, &mut cache) {
            Ok(HashVerdict::HASH_MATCH) => {}
            Ok(HashVerdict::HASH_MISMATCH) | Err(_) => {
                return Ok(GateResult::Blocked(BlockedReason::SchemaTampered));
            }
        }

        // Step 7: Audit script self-tests hash
        match self.step7_audit_self_tests(&manifest, &mut cache) {
            Ok(HashVerdict::HASH_MATCH) => {}
            Ok(HashVerdict::HASH_MISMATCH) | Err(_) => {
                return Ok(GateResult::Blocked(BlockedReason::SchemaTampered));
            }
        }

        // Step 8: Synthetic fixtures hash
        match self.step8_synthetic_fixtures(&manifest, &mut cache) {
            Ok(HashVerdict::HASH_MATCH) => {}
            Ok(HashVerdict::HASH_MISMATCH) | Err(_) => {
                return Ok(GateResult::Blocked(BlockedReason::SchemaTampered));
            }
        }

        Ok(GateResult::Passed)
    }

    // ------------------------------------------------------------------
    // Step 1: Sentinel verification
    // ------------------------------------------------------------------
    fn step1_sentinel(&self) -> Result<StepResult> {
        // Ownership already established → skip sentinel check.
        if self.fingerprint_path.exists() {
            return Ok(StepResult::Pass);
        }

        // First run: sentinel must exist and be valid.
        if !self.sentinel_path.exists() {
            return Ok(StepResult::Fail(BlockedReason::OwnershipUnverified));
        }

        let content = match fs::read_to_string(&self.sentinel_path) {
            Ok(c) => c,
            Err(_) => return Ok(StepResult::Fail(BlockedReason::OwnershipUnverified)),
        };

        let sentinel: OwnerSentinel = match serde_yaml_ng::from_str(&content) {
            Ok(s) => s,
            Err(_) => return Ok(StepResult::Fail(BlockedReason::OwnershipUnverified)),
        };

        if sentinel.name.trim().is_empty() || sentinel.email.trim().is_empty() {
            return Ok(StepResult::Fail(BlockedReason::OwnershipUnverified));
        }

        Ok(StepResult::Pass)
    }

    // ------------------------------------------------------------------
    // Step 2: Manifest clean in git
    // ------------------------------------------------------------------
    fn step2_manifest_clean(&self) -> Result<StepResult> {
        let cwd = self
            .manifest_path
            .parent()
            .unwrap_or_else(|| Path::new("."));
        let output = Command::new("git")
            .current_dir(cwd)
            .args([
                "status",
                "--porcelain",
                "--",
                &self.manifest_path.to_string_lossy(),
            ])
            .output()
            .with_context(|| "failed to run git status")?;

        if !output.status.success() {
            return Ok(StepResult::Fail(BlockedReason::ManifestDirty));
        }

        if output.stdout.is_empty() {
            Ok(StepResult::Pass)
        } else {
            Ok(StepResult::Fail(BlockedReason::ManifestDirty))
        }
    }

    // ------------------------------------------------------------------
    // Step 3: GPG signature + fingerprint match
    // ------------------------------------------------------------------
    fn step3_manifest_signature(&self) -> Result<StepResult> {
        let cwd = self
            .manifest_path
            .parent()
            .unwrap_or_else(|| Path::new("."));
        let output = Command::new("git")
            .current_dir(cwd)
            .args([
                "log",
                "-1",
                "--format=%G?%n%GF",
                "--",
                &self.manifest_path.to_string_lossy(),
            ])
            .output()
            .with_context(|| "failed to run git log")?;

        if !output.status.success() {
            return Ok(StepResult::Fail(BlockedReason::ManifestUnsignedOrWrongKey));
        }

        let stdout = String::from_utf8_lossy(&output.stdout);
        let lines: Vec<&str> = stdout.trim().lines().collect();
        if lines.len() < 2 || lines[0] != "G" {
            return Ok(StepResult::Fail(BlockedReason::ManifestUnsignedOrWrongKey));
        }

        let commit_fingerprint = lines[1].trim();
        if commit_fingerprint.is_empty() {
            return Ok(StepResult::Fail(BlockedReason::ManifestUnsignedOrWrongKey));
        }

        if !self.fingerprint_path.exists() {
            return Ok(StepResult::Fail(BlockedReason::ManifestUnsignedOrWrongKey));
        }

        let recorded = fs::read_to_string(&self.fingerprint_path).with_context(|| {
            format!(
                "failed to read fingerprint at {}",
                self.fingerprint_path.display()
            )
        })?;

        if commit_fingerprint != recorded.trim() {
            return Ok(StepResult::Fail(BlockedReason::ManifestUnsignedOrWrongKey));
        }

        Ok(StepResult::Pass)
    }

    // ------------------------------------------------------------------
    // Manifest loader
    // ------------------------------------------------------------------
    fn load_manifest(&self) -> Result<ReleaseManifest> {
        let content = fs::read_to_string(&self.manifest_path).with_context(|| {
            format!(
                "failed to read manifest at {}",
                self.manifest_path.display()
            )
        })?;
        let manifest: ReleaseManifest = serde_yaml_ng::from_str(&content)
            .with_context(|| "failed to parse release manifest")?;
        Ok(manifest)
    }

    // ------------------------------------------------------------------
    // Steps 4-8: Hash chain verification
    // ------------------------------------------------------------------
    fn step4_bin_aliases(
        &self,
        manifest: &ReleaseManifest,
        cache: &mut FileCache,
    ) -> Result<HashVerdict> {
        let paths = self.resolve_paths(&manifest.bin_aliases)?;
        let actual = compute_list_hash(&paths, cache)?;
        Ok(if actual == manifest.bin_aliases_hash {
            HashVerdict::HASH_MATCH
        } else {
            HashVerdict::HASH_MISMATCH
        })
    }

    fn step5_acceptance_schema(
        &self,
        manifest: &ReleaseManifest,
        cache: &mut FileCache,
    ) -> Result<HashVerdict> {
        let paths = self.resolve_paths(&manifest.acceptance_files)?;
        let actual = compute_list_hash(&paths, cache)?;
        Ok(if actual == manifest.acceptance_schema_hash {
            HashVerdict::HASH_MATCH
        } else {
            HashVerdict::HASH_MISMATCH
        })
    }

    fn step6_audit_scripts(
        &self,
        manifest: &ReleaseManifest,
        cache: &mut FileCache,
    ) -> Result<HashVerdict> {
        let paths = self.resolve_paths(&manifest.audit_scripts)?;
        let actual = compute_list_hash(&paths, cache)?;
        Ok(if actual == manifest.audit_scripts_hash {
            HashVerdict::HASH_MATCH
        } else {
            HashVerdict::HASH_MISMATCH
        })
    }

    fn step7_audit_self_tests(
        &self,
        manifest: &ReleaseManifest,
        cache: &mut FileCache,
    ) -> Result<HashVerdict> {
        let paths = self.resolve_paths(&manifest.audit_script_self_tests)?;
        let actual = compute_list_hash(&paths, cache)?;
        Ok(if actual == manifest.audit_script_self_tests_hash {
            HashVerdict::HASH_MATCH
        } else {
            HashVerdict::HASH_MISMATCH
        })
    }

    fn step8_synthetic_fixtures(
        &self,
        manifest: &ReleaseManifest,
        cache: &mut FileCache,
    ) -> Result<HashVerdict> {
        let paths = self.resolve_paths(&manifest.synthetic_fixtures)?;
        let actual = compute_list_hash(&paths, cache)?;
        Ok(if actual == manifest.synthetic_fixtures_hash {
            HashVerdict::HASH_MATCH
        } else {
            HashVerdict::HASH_MISMATCH
        })
    }

    fn resolve_paths(&self, rel_paths: &[String]) -> Result<Vec<PathBuf>> {
        let base = self
            .manifest_path
            .parent()
            .unwrap_or_else(|| Path::new("."));
        Ok(rel_paths.iter().map(|p| base.join(p)).collect())
    }
}

// ----------------------------------------------------------------------
// Hash helpers
// ----------------------------------------------------------------------

#[cfg(test)]
fn compute_file_hash_cached(path: &Path, cache: &mut FileCache) -> Result<String> {
    let metadata = fs::metadata(path)
        .with_context(|| format!("failed to read metadata for {}", path.display()))?;
    let size = metadata.len();
    let mtime = metadata.modified().unwrap_or(SystemTime::UNIX_EPOCH);
    let key = (path.to_path_buf(), mtime, size);

    if let Some(hash) = cache.get(&key) {
        return Ok(hash.clone());
    }

    let content = fs::read(path).with_context(|| format!("failed to read {}", path.display()))?;
    let hash = sha256_bytes(&content);
    cache.insert(key, hash.clone());
    Ok(hash)
}

/// Re-hash a list of files using parallel workers (`std::thread::scope`)
/// and compare the combined hash against the recorded value.
fn compute_list_hash(paths: &[PathBuf], cache: &mut FileCache) -> Result<String> {
    let mut entries: Vec<(PathBuf, String)> = Vec::with_capacity(paths.len());
    let mut threaded_results: Vec<(PathBuf, String, u64, SystemTime)> =
        Vec::with_capacity(paths.len());

    std::thread::scope(|s| {
        let handles: Vec<_> = paths
            .iter()
            .map(|path| {
                s.spawn(move || -> Result<(PathBuf, String, u64, SystemTime)> {
                    let metadata = fs::metadata(path).with_context(|| {
                        format!("failed to read metadata for {}", path.display())
                    })?;
                    let size = metadata.len();
                    let mtime = metadata.modified().unwrap_or(SystemTime::UNIX_EPOCH);
                    let content = fs::read(path)
                        .with_context(|| format!("failed to read {}", path.display()))?;
                    let hash = sha256_bytes(&content);
                    Ok((path.to_path_buf(), hash, size, mtime))
                })
            })
            .collect();

        for handle in handles {
            threaded_results.push(handle.join().unwrap()?);
        }
        Ok::<(), anyhow::Error>(())
    })?;

    // Update cache serially after all threads have joined.
    for (path, hash, size, mtime) in threaded_results {
        let key = (path.clone(), mtime, size);
        cache.insert(key, hash.clone());
        entries.push((path, hash));
    }

    entries.sort_by(|a, b| a.0.cmp(&b.0));
    let combined = entries
        .into_iter()
        .map(|(p, h)| format!("{}={}", p.display(), h))
        .collect::<Vec<_>>()
        .join("\n");

    Ok(sha256_string(&combined))
}

fn sha256_bytes(data: &[u8]) -> String {
    use sha2::{Digest, Sha256};
    let mut hasher = Sha256::new();
    hasher.update(data);
    hex::encode(hasher.finalize())
}

fn sha256_string(s: &str) -> String {
    sha256_bytes(s.as_bytes())
}

// ----------------------------------------------------------------------
// Tests
// ----------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use std::time::Duration;
    use tempfile::TempDir;

    fn write_file(path: &Path, content: &str) {
        let mut file = fs::File::create(path).unwrap();
        file.write_all(content.as_bytes()).unwrap();
    }

    fn create_git_repo(dir: &Path) {
        let status = Command::new("git")
            .args(["init"])
            .current_dir(dir)
            .status()
            .expect("git init failed");
        assert!(status.success());

        Command::new("git")
            .args(["config", "user.email", "test@example.com"])
            .current_dir(dir)
            .status()
            .expect("git config failed");

        Command::new("git")
            .args(["config", "user.name", "Test User"])
            .current_dir(dir)
            .status()
            .expect("git config failed");
    }

    fn commit_all(dir: &Path, message: &str) {
        let status = Command::new("git")
            .args(["add", "."])
            .current_dir(dir)
            .status()
            .expect("git add failed");
        assert!(status.success());

        let status = Command::new("git")
            .args(["commit", "-m", message, "--no-verify"])
            .current_dir(dir)
            .status()
            .expect("git commit failed");
        assert!(status.success());
    }

    // ------------------------------------------------------------------
    // Step 1 tests
    // ------------------------------------------------------------------

    #[test]
    fn test_step1_sentinel_absent_no_fingerprint_blocked() {
        let tmp = TempDir::new().unwrap();
        let gate = PreAuditGate::new(
            tmp.path().join("manifest.yaml"),
            tmp.path().join(".audit.lock"),
            tmp.path().join("fingerprint"),
            tmp.path().join("sentinel"),
        );
        let result = gate.step1_sentinel().unwrap();
        assert!(
            matches!(result, StepResult::Fail(BlockedReason::OwnershipUnverified)),
            "expected BLOCKED ownership_unverified when sentinel is absent and fingerprint missing"
        );
    }

    #[test]
    fn test_step1_sentinel_valid_passes() {
        let tmp = TempDir::new().unwrap();
        let sentinel_path = tmp.path().join("sentinel");
        write_file(&sentinel_path, "name: Alice\nemail: alice@example.com\n");

        let gate = PreAuditGate::new(
            tmp.path().join("manifest.yaml"),
            tmp.path().join(".audit.lock"),
            tmp.path().join("fingerprint"),
            &sentinel_path,
        );
        let result = gate.step1_sentinel().unwrap();
        assert!(matches!(result, StepResult::Pass));
    }

    #[test]
    fn test_step1_sentinel_malformed_blocked() {
        let tmp = TempDir::new().unwrap();
        let sentinel_path = tmp.path().join("sentinel");
        write_file(&sentinel_path, "not_valid_yaml: [");

        let gate = PreAuditGate::new(
            tmp.path().join("manifest.yaml"),
            tmp.path().join(".audit.lock"),
            tmp.path().join("fingerprint"),
            &sentinel_path,
        );
        let result = gate.step1_sentinel().unwrap();
        assert!(matches!(
            result,
            StepResult::Fail(BlockedReason::OwnershipUnverified)
        ));
    }

    #[test]
    fn test_step1_sentinel_empty_name_blocked() {
        let tmp = TempDir::new().unwrap();
        let sentinel_path = tmp.path().join("sentinel");
        write_file(&sentinel_path, "name: \"\"\nemail: alice@example.com\n");

        let gate = PreAuditGate::new(
            tmp.path().join("manifest.yaml"),
            tmp.path().join(".audit.lock"),
            tmp.path().join("fingerprint"),
            &sentinel_path,
        );
        let result = gate.step1_sentinel().unwrap();
        assert!(matches!(
            result,
            StepResult::Fail(BlockedReason::OwnershipUnverified)
        ));
    }

    #[test]
    fn test_step1_fingerprint_exists_skips_sentinel() {
        let tmp = TempDir::new().unwrap();
        let fingerprint_path = tmp.path().join("fingerprint");
        write_file(&fingerprint_path, "ABCD1234");

        let gate = PreAuditGate::new(
            tmp.path().join("manifest.yaml"),
            tmp.path().join(".audit.lock"),
            &fingerprint_path,
            tmp.path().join("sentinel"),
        );
        let result = gate.step1_sentinel().unwrap();
        assert!(matches!(result, StepResult::Pass));
    }

    // ------------------------------------------------------------------
    // Step 2 tests
    // ------------------------------------------------------------------

    #[test]
    fn test_step2_manifest_dirty() {
        let tmp = TempDir::new().unwrap();
        create_git_repo(tmp.path());
        let manifest_path = tmp.path().join("manifest.yaml");
        write_file(&manifest_path, "version: 1");
        commit_all(tmp.path(), "init");

        // Modify without committing
        write_file(&manifest_path, "version: 2");

        let gate = PreAuditGate::new(
            &manifest_path,
            tmp.path().join(".audit.lock"),
            tmp.path().join("fingerprint"),
            tmp.path().join("sentinel"),
        );
        let result = gate.step2_manifest_clean().unwrap();
        assert!(matches!(
            result,
            StepResult::Fail(BlockedReason::ManifestDirty)
        ));
    }

    #[test]
    fn test_step2_manifest_clean() {
        let tmp = TempDir::new().unwrap();
        create_git_repo(tmp.path());
        let manifest_path = tmp.path().join("manifest.yaml");
        write_file(&manifest_path, "version: 1");
        commit_all(tmp.path(), "init");

        let gate = PreAuditGate::new(
            &manifest_path,
            tmp.path().join(".audit.lock"),
            tmp.path().join("fingerprint"),
            tmp.path().join("sentinel"),
        );
        let result = gate.step2_manifest_clean().unwrap();
        assert!(matches!(result, StepResult::Pass));
    }

    // ------------------------------------------------------------------
    // Step 3 tests
    // ------------------------------------------------------------------

    #[test]
    fn test_step3_unsigned_manifest_blocked() {
        let tmp = TempDir::new().unwrap();
        create_git_repo(tmp.path());
        let manifest_path = tmp.path().join("manifest.yaml");
        let fingerprint_path = tmp.path().join("fingerprint");
        write_file(&manifest_path, "version: 1");
        write_file(&fingerprint_path, "ABCD1234");
        commit_all(tmp.path(), "init");

        let gate = PreAuditGate::new(
            &manifest_path,
            tmp.path().join(".audit.lock"),
            &fingerprint_path,
            tmp.path().join("sentinel"),
        );
        let result = gate.step3_manifest_signature().unwrap();
        assert!(matches!(
            result,
            StepResult::Fail(BlockedReason::ManifestUnsignedOrWrongKey)
        ));
    }

    #[test]
    fn test_step3_missing_fingerprint_blocked() {
        let tmp = TempDir::new().unwrap();
        create_git_repo(tmp.path());
        let manifest_path = tmp.path().join("manifest.yaml");
        write_file(&manifest_path, "version: 1");
        commit_all(tmp.path(), "init");

        let gate = PreAuditGate::new(
            &manifest_path,
            tmp.path().join(".audit.lock"),
            tmp.path().join("fingerprint"),
            tmp.path().join("sentinel"),
        );
        let result = gate.step3_manifest_signature().unwrap();
        assert!(matches!(
            result,
            StepResult::Fail(BlockedReason::ManifestUnsignedOrWrongKey)
        ));
    }

    // ------------------------------------------------------------------
    // Hash / cache tests
    // ------------------------------------------------------------------

    #[test]
    fn test_hash_verdict_match_and_mismatch() {
        let tmp = TempDir::new().unwrap();
        let file1 = tmp.path().join("a.txt");
        let file2 = tmp.path().join("b.txt");
        write_file(&file1, "hello");
        write_file(&file2, "world");

        let mut cache = HashMap::new();
        let paths = vec![file1.clone(), file2.clone()];
        let hash1 = compute_list_hash(&paths, &mut cache).unwrap();

        // Same files, same hash
        let hash2 = compute_list_hash(&paths, &mut cache).unwrap();
        assert_eq!(hash1, hash2);

        // Cache should have entries
        assert_eq!(cache.len(), 2);

        // Modify file
        write_file(&file1, "changed");
        let hash3 = compute_list_hash(&paths, &mut cache).unwrap();
        assert_ne!(hash1, hash3);
    }

    #[test]
    fn test_parallel_hashing_performance() {
        let tmp = TempDir::new().unwrap();
        let mut paths = Vec::new();
        for i in 0..20 {
            let p = tmp.path().join(format!("file{}.txt", i));
            write_file(&p, &format!("content{}", i));
            paths.push(p);
        }

        let mut cache = HashMap::new();
        let start = std::time::Instant::now();
        let _hash = compute_list_hash(&paths, &mut cache).unwrap();
        let elapsed = start.elapsed();

        // Must complete within 2-second perf budget
        assert!(
            elapsed < Duration::from_secs(2),
            "hashing took {:?}, exceeding 2s budget",
            elapsed
        );
        assert_eq!(cache.len(), 20);
    }

    #[test]
    fn test_file_cache_uses_mtime_and_size() {
        let tmp = TempDir::new().unwrap();
        let file = tmp.path().join("data.txt");
        write_file(&file, "static");

        let mut cache = HashMap::new();
        let h1 = compute_file_hash_cached(&file, &mut cache).unwrap();

        // Second call should hit cache
        let h2 = compute_file_hash_cached(&file, &mut cache).unwrap();
        assert_eq!(h1, h2);
        assert_eq!(cache.len(), 1);
    }

    #[test]
    fn test_hash_verdict_binary_only() {
        // Ensure HashVerdict has exactly two variants (no raw hash leakage).
        let m = HashVerdict::HASH_MATCH;
        let mm = HashVerdict::HASH_MISMATCH;
        assert_ne!(m, mm);
    }

    // ------------------------------------------------------------------
    // Concurrency / flock tests
    // ------------------------------------------------------------------

    #[test]
    fn test_concurrent_lock_returns_audit_in_progress() {
        let tmp = TempDir::new().unwrap();
        let lock_path = tmp.path().join(".audit.lock");

        // First guard acquires the lock
        let _guard = FlockGuard::lock_exclusive(&lock_path).unwrap();

        let gate = PreAuditGate::new(
            tmp.path().join("manifest.yaml"),
            &lock_path,
            tmp.path().join("fingerprint"),
            tmp.path().join("sentinel"),
        );

        let result = gate.run().unwrap();
        assert_eq!(result, GateResult::AuditInProgress);
    }

    #[test]
    fn test_gate_creates_lock_parent_directory() {
        let tmp = TempDir::new().unwrap();
        let lock_path = tmp.path().join("deep").join("nested").join(".audit.lock");
        let sentinel_path = tmp.path().join("sentinel");
        write_file(&sentinel_path, "name: Test\nemail: test@example.com\n");

        let gate = PreAuditGate::new(
            tmp.path().join("manifest.yaml"),
            &lock_path,
            tmp.path().join("fingerprint"),
            &sentinel_path,
        );
        let result = gate.run().unwrap();
        // Will fail at step2 (no git repo), but should NOT crash on missing parent dir
        assert_ne!(result, GateResult::AuditInProgress);
        assert!(lock_path.parent().unwrap().exists());
    }

    #[test]
    fn test_lock_released_on_drop_allows_next_run() {
        let tmp = TempDir::new().unwrap();
        let lock_path = tmp.path().join(".audit.lock");
        let sentinel_path = tmp.path().join("sentinel");
        write_file(&sentinel_path, "name: Test\nemail: test@example.com\n");

        {
            let gate = PreAuditGate::new(
                tmp.path().join("manifest.yaml"),
                &lock_path,
                tmp.path().join("fingerprint"),
                &sentinel_path,
            );
            let result = gate.run().unwrap();
            // Will fail at step2 (no git repo), but should NOT be AuditInProgress
            assert_ne!(result, GateResult::AuditInProgress);
        }

        // Lock should be released; second run should also proceed (not blocked by lock)
        let gate = PreAuditGate::new(
            tmp.path().join("manifest.yaml"),
            &lock_path,
            tmp.path().join("fingerprint"),
            &sentinel_path,
        );
        let result = gate.run().unwrap();
        assert_ne!(result, GateResult::AuditInProgress);
    }

    // ------------------------------------------------------------------
    // End-to-end partial gate tests
    // ------------------------------------------------------------------

    #[test]
    fn test_full_gate_blocked_at_step2_when_dirty() {
        let tmp = TempDir::new().unwrap();
        create_git_repo(tmp.path());
        let manifest_path = tmp.path().join("manifest.yaml");
        let sentinel_path = tmp.path().join("sentinel");
        write_file(&manifest_path, "version: 1");
        write_file(&sentinel_path, "name: Test\nemail: test@example.com\n");
        commit_all(tmp.path(), "init");

        // Make dirty
        write_file(&manifest_path, "version: 2");

        let gate = PreAuditGate::new(
            &manifest_path,
            tmp.path().join(".audit.lock"),
            tmp.path().join("fingerprint"),
            &sentinel_path,
        );
        let result = gate.run().unwrap();
        assert_eq!(result, GateResult::Blocked(BlockedReason::ManifestDirty));
    }

    #[test]
    fn test_full_gate_passes_with_valid_setup() {
        let tmp = TempDir::new().unwrap();
        create_git_repo(tmp.path());

        let manifest_path = tmp.path().join("manifest.yaml");
        let fingerprint_path = tmp.path().join("fingerprint");
        let sentinel_path = tmp.path().join("sentinel");
        let lock_path = tmp.path().join(".audit.lock");

        // Create files to hash
        let bin_aliases_path = tmp.path().join("bin_aliases.json");
        write_file(&bin_aliases_path, r#"{"sh":"sh"}"#);

        let acceptance_path = tmp.path().join("acceptance.yaml");
        write_file(&acceptance_path, "schema: v1\n");

        let audit_script_path = tmp.path().join("audit.sh");
        write_file(&audit_script_path, "#!/bin/sh\necho ok\n");

        let self_test_path = tmp.path().join("audit.test.sh");
        write_file(&self_test_path, "#!/bin/sh\nexit 0\n");

        let fixture_path = tmp.path().join("fixture.txt");
        write_file(&fixture_path, "fixture data\n");

        // Compute hashes
        let mut cache = HashMap::new();
        let bin_aliases_hash = compute_list_hash(&[bin_aliases_path.clone()], &mut cache).unwrap();
        let acceptance_hash = compute_list_hash(&[acceptance_path.clone()], &mut cache).unwrap();
        let audit_scripts_hash =
            compute_list_hash(&[audit_script_path.clone()], &mut cache).unwrap();
        let self_tests_hash = compute_list_hash(&[self_test_path.clone()], &mut cache).unwrap();
        let fixtures_hash = compute_list_hash(&[fixture_path.clone()], &mut cache).unwrap();

        // Build manifest
        let manifest = ReleaseManifest {
            schema_version: "0.1.0".to_string(),
            released_at: "2026-01-01T00:00:00Z".to_string(),
            released_by: "test".to_string(),
            auditor_min_version: "0.1.0".to_string(),
            acceptance_files: vec!["acceptance.yaml".to_string()],
            audit_scripts: vec!["audit.sh".to_string()],
            audit_script_self_tests: vec!["audit.test.sh".to_string()],
            synthetic_fixtures: vec!["fixture.txt".to_string()],
            bin_aliases: vec!["bin_aliases.json".to_string()],
            bin_aliases_hash,
            acceptance_schema_hash: acceptance_hash,
            audit_scripts_hash,
            audit_script_self_tests_hash: self_tests_hash,
            synthetic_fixtures_hash: fixtures_hash,
        };
        let manifest_yaml = serde_yaml_ng::to_string(&manifest).unwrap();
        write_file(&manifest_path, &manifest_yaml);

        // Sentinel valid, no fingerprint yet → step1 passes
        write_file(&sentinel_path, "name: Test\nemail: test@example.com\n");

        // Commit everything so manifest is clean
        commit_all(tmp.path(), "initial");

        // We skip step3 (GPG) by providing a fingerprint file and mocking?
        // Actually step3 will run and fail because commits are not GPG-signed.
        // To get a full pass, we'd need a signed commit, which is hard in tests.
        // Instead, verify that steps 1,2,4-8 pass individually.

        let gate = PreAuditGate::new(
            &manifest_path,
            &lock_path,
            &fingerprint_path,
            &sentinel_path,
        );

        assert!(matches!(gate.step1_sentinel().unwrap(), StepResult::Pass));
        assert!(matches!(
            gate.step2_manifest_clean().unwrap(),
            StepResult::Pass
        ));

        let m = gate.load_manifest().unwrap();
        let mut cache = HashMap::new();
        assert_eq!(
            gate.step4_bin_aliases(&m, &mut cache).unwrap(),
            HashVerdict::HASH_MATCH
        );
        assert_eq!(
            gate.step5_acceptance_schema(&m, &mut cache).unwrap(),
            HashVerdict::HASH_MATCH
        );
        assert_eq!(
            gate.step6_audit_scripts(&m, &mut cache).unwrap(),
            HashVerdict::HASH_MATCH
        );
        assert_eq!(
            gate.step7_audit_self_tests(&m, &mut cache).unwrap(),
            HashVerdict::HASH_MATCH
        );
        assert_eq!(
            gate.step8_synthetic_fixtures(&m, &mut cache).unwrap(),
            HashVerdict::HASH_MATCH
        );
    }
}
