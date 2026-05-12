use std::fs::File;
use std::path::Path;

use anyhow::Result;
use fs2::FileExt;

/// A RAII guard that holds an advisory file lock.
pub struct FlockGuard {
    file: File,
}

impl FlockGuard {
    /// Acquire an exclusive lock on the given path.
    pub fn lock_exclusive<P: AsRef<Path>>(path: P) -> Result<Self> {
        let file = File::create(path)?;
        file.lock_exclusive()?;
        Ok(Self { file })
    }

    /// Try to acquire an exclusive lock without blocking.
    /// Returns `Ok(None)` if the lock is already held by another process.
    pub fn try_lock_exclusive<P: AsRef<Path>>(path: P) -> Result<Option<Self>> {
        let file = File::create(path)?;
        match file.try_lock_exclusive() {
            Ok(()) => Ok(Some(Self { file })),
            Err(_) => Ok(None),
        }
    }

    /// Acquire a shared lock on the given path.
    pub fn lock_shared<P: AsRef<Path>>(path: P) -> Result<Self> {
        let file = File::create(path)?;
        file.lock_shared()?;
        Ok(Self { file })
    }
}

impl Drop for FlockGuard {
    fn drop(&mut self) {
        let _ = self.file.unlock();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    #[test]
    fn test_flock_guard() {
        let temp_dir = std::env::temp_dir();
        let lock_path = temp_dir.join("aiplus_test_flock.lock");

        {
            let guard = FlockGuard::lock_exclusive(&lock_path).unwrap();
            let mut file = &guard.file;
            writeln!(file, "locked").unwrap();
            // Lock is released when guard is dropped
        }

        // Clean up
        let _ = std::fs::remove_file(&lock_path);
    }
}
