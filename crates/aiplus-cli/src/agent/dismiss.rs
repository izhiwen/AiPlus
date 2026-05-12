use crate::agent::cache;
use anyhow::Result;

pub fn handle_dismiss(role: &str) -> Result<()> {
    println!("Dismissing {} from the active team...", role);
    if let Ok(cache) = cache::global_cache().lock() {
        cache.invalidate(role, cache::InvalidationReason::RoleDismissed);
    }
    Ok(())
}
