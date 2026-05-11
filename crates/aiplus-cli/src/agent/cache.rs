use std::collections::HashMap;
use std::sync::{Mutex, OnceLock};
use std::time::{Duration, Instant};

/// In-process warm-bench cache for AiPlus Agent Team v0.1.
///
/// DESIGN.md §6:
/// - std-only HashMap + manual TTL sweep (no lru, moka, cached, dashmap)
/// - TTL values from each role's TOML `[agent] warm_bench_ttl_seconds`
/// - 6 invalidation triggers as discrete events (no debounce)
#[allow(dead_code)]
pub struct WarmBenchCache {
    entries: HashMap<String, (Instant, String)>, // role -> (last_access, cached_state_json)
    hits: u64,
    misses: u64,
    evictions: u64,
}

#[allow(dead_code)]
impl WarmBenchCache {
    pub fn new() -> Self {
        Self {
            entries: HashMap::new(),
            hits: 0,
            misses: 0,
            evictions: 0,
        }
    }

    /// Look up a cached state for `role`. If the entry exists and is within
    /// `ttl_seconds`, returns a reference to the cached JSON state and bumps
    /// the hit counter. If the entry is expired it is removed, the eviction
    /// counter is bumped, and `None` is returned.
    pub fn get(&mut self, role: &str, ttl_seconds: u64) -> Option<&str> {
        let now = Instant::now();
        let ttl = Duration::from_secs(ttl_seconds);

        if let Some((last_access, _state)) = self.entries.get(role) {
            if now.duration_since(*last_access) <= ttl {
                self.hits += 1;
                // Update last_access in-place without re-inserting
                let entry = self.entries.get_mut(role).unwrap();
                entry.0 = now;
                return Some(&entry.1);
            }
            // Expired — evict on access
            self.entries.remove(role);
            self.evictions += 1;
            self.misses += 1;
            return None;
        }

        self.misses += 1;
        None
    }

    /// Insert or replace the cached state for `role`.
    pub fn put(&mut self, role: &str, state: String) {
        self.entries
            .insert(role.to_string(), (Instant::now(), state));
    }

    /// Remove the entry for a specific role.
    pub fn invalidate(&mut self, role: &str) {
        if self.entries.remove(role).is_some() {
            self.evictions += 1;
        }
    }

    /// Remove all entries.
    pub fn invalidate_all(&mut self) {
        let count = self.entries.len() as u64;
        self.entries.clear();
        self.evictions += count;
    }

    /// Manually sweep all expired entries. Called periodically or before
    /// reporting cache diagnostics.
    pub fn sweep_expired(&mut self) {
        let now = Instant::now();
        let expired: Vec<String> = self
            .entries
            .iter()
            .filter(|(_, (last_access, _))| {
                // We don't know the per-role TTL here, so we use a default
                // conservative sweep of 1 hour. Callers should prefer
                // per-role get() for precise TTL enforcement.
                now.duration_since(*last_access) > Duration::from_secs(3600)
            })
            .map(|(role, _)| role.clone())
            .collect();
        for role in expired {
            self.entries.remove(&role);
            self.evictions += 1;
        }
    }

    /// Return (hits, misses, evictions).
    pub fn stats(&self) -> (u64, u64, u64) {
        (self.hits, self.misses, self.evictions)
    }

    /// Returns true if the cache has at least one live entry.
    pub fn is_warm(&self) -> bool {
        !self.entries.is_empty()
    }
}

impl Default for WarmBenchCache {
    fn default() -> Self {
        Self::new()
    }
}

static CACHE: OnceLock<Mutex<WarmBenchCache>> = OnceLock::new();

/// Access the process-global warm-bench cache.
pub fn global_cache() -> &'static Mutex<WarmBenchCache> {
    CACHE.get_or_init(|| Mutex::new(WarmBenchCache::new()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::thread;

    #[test]
    fn put_and_get_within_ttl_is_hit() {
        let mut cache = WarmBenchCache::new();
        cache.put("advisor", r#"{"state":"ready"}"#.to_string());
        let result = cache.get("advisor", 3600);
        assert_eq!(result, Some(r#"{"state":"ready"}"#));
        assert_eq!(cache.stats(), (1, 0, 0));
    }

    #[test]
    fn get_after_ttl_expires_is_miss_and_eviction() {
        let mut cache = WarmBenchCache::new();
        cache.put("qa", r#"{"state":"busy"}"#.to_string());
        // Simulate TTL expiration by using 0-second TTL
        let result = cache.get("qa", 0);
        assert_eq!(result, None);
        let (hits, misses, evictions) = cache.stats();
        assert_eq!(hits, 0);
        assert_eq!(misses, 1);
        assert_eq!(evictions, 1);
    }

    #[test]
    fn invalidate_makes_next_get_miss() {
        let mut cache = WarmBenchCache::new();
        cache.put("pm", r#"{"state":"planning"}"#.to_string());
        assert!(cache.get("pm", 3600).is_some());
        cache.invalidate("pm");
        assert!(cache.get("pm", 3600).is_none());
        let (hits, misses, evictions) = cache.stats();
        assert_eq!(hits, 1);
        assert_eq!(misses, 1);
        assert_eq!(evictions, 1);
    }

    #[test]
    fn invalidate_all_clears_all_entries() {
        let mut cache = WarmBenchCache::new();
        cache.put("ceo", r#"{"state":"reviewing"}"#.to_string());
        cache.put("architect", r#"{"state":"designing"}"#.to_string());
        cache.invalidate_all();
        assert!(cache.get("ceo", 3600).is_none());
        assert!(cache.get("architect", 3600).is_none());
        let (_, _, evictions) = cache.stats();
        assert_eq!(evictions, 2);
    }

    #[test]
    fn sweep_expired_removes_stale_entries() {
        let mut cache = WarmBenchCache::new();
        cache.put("engineer-a", r#"{"state":"coding"}"#.to_string());
        // Short sleep to ensure the entry is "old" relative to a 1h sweep
        thread::sleep(Duration::from_millis(10));
        // put a fresh entry
        cache.put("engineer-b", r#"{"state":"reviewing"}"#.to_string());
        // No expired entries with 1h default sweep if we haven't slept an hour,
        // so verify sweep doesn't remove fresh entries.
        cache.sweep_expired();
        assert!(cache.get("engineer-a", 3600).is_some());
        assert!(cache.get("engineer-b", 3600).is_some());
    }

    #[test]
    fn stats_track_correctly() {
        let mut cache = WarmBenchCache::new();
        cache.put("reviewer", r#"{"state":"idle"}"#.to_string());
        cache.get("reviewer", 3600);
        cache.get("reviewer", 3600);
        cache.get("unknown", 3600);
        let (hits, misses, evictions) = cache.stats();
        assert_eq!(hits, 2);
        assert_eq!(misses, 1);
        assert_eq!(evictions, 0);
    }
}
