use crate::agent::cache;
use anyhow::Result;

pub fn handle_reset() -> Result<()> {
    println!("Resetting agent team state...");
    if let Ok(cache) = cache::global_cache().lock() {
        cache.clear(cache::InvalidationReason::CacheExplicitlyCleared);
    }
    println!("Warm-bench cache cleared.");
    Ok(())
}
