// TODO(v0.2): WarmBenchCache scaffolding for cross-session role-state
// caching. Wired into `aiplus agent route` once we add the warm-bench
// fast-path. Currently unused — keep as scaffolding rather than rewrite.
#![allow(dead_code)]

use aiplus_core::agent_team::RoleId;
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Condvar, Mutex, OnceLock};
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant};

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
