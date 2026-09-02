//! Test-only fixture helper: collision-proof temp directories.
//!
//! `std::env::temp_dir().join("bella_...")` is a fixed name shared by every
//! process that runs this suite. Two concurrent full-suite runs (this repo's
//! isolation policy runs `--worktree`, and every worktree shares one `/tmp`)
//! collide on that name, and the common `remove_dir_all`-before-use idiom
//! makes the collision destructive: whichever run gets there second deletes
//! the fixture the first run is still using mid-test.
//!
//! [`unique_temp_dir`] closes this by keying the directory name on the
//! process id, `SystemTime::now()` nanos, and a process-wide atomic counter
//! (the counter is what keeps two calls in the *same* process, in the same
//! nanosecond, from colliding with each other). It creates the directory and
//! returns it — callers must not `remove_dir_all` it first; there is nothing
//! there to remove, and doing so on a *shared* fixture would defeat the
//! point.
//!
//! Declared `#[cfg(test)]` in `lib.rs` — deliberately absent from the crate's
//! `pub mod` list, so it is never reachable from the shipped binary.
//! Mirrors `core/bastion/src/testsupport.rs::unique_temp_dir`.

use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};

/// Returns a freshly created, collision-proof temp directory under the
/// system temp dir, named `<prefix>-<pid>-<nanos>-<seq>`.
pub fn unique_temp_dir(prefix: &str) -> PathBuf {
    static COUNTER: AtomicU64 = AtomicU64::new(0);
    let seq = COUNTER.fetch_add(1, Ordering::Relaxed);
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    let dir = std::env::temp_dir().join(format!("{prefix}-{}-{nanos}-{seq}", std::process::id()));
    std::fs::create_dir_all(&dir).expect("unique_temp_dir: failed to create fixture dir");
    dir
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unique_temp_dir_never_repeats_within_a_process() {
        let mut seen = std::collections::HashSet::new();
        for _ in 0..500 {
            let dir = unique_temp_dir("bella-uniqueness-probe");
            assert!(
                seen.insert(dir.clone()),
                "unique_temp_dir must never hand out the same path twice"
            );
            let _ = std::fs::remove_dir_all(&dir);
        }
    }

    #[test]
    fn unique_temp_dir_path_contains_the_process_id() {
        let dir = unique_temp_dir("bella-pid-probe");
        let name = dir.file_name().unwrap().to_string_lossy().to_string();
        let pid = std::process::id().to_string();
        assert!(
            name.contains(&pid),
            "expected the process id ({pid}) in the directory name; got {name}"
        );
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn unique_temp_dir_creates_the_directory() {
        let dir = unique_temp_dir("bella-exists-probe");
        assert!(
            dir.is_dir(),
            "unique_temp_dir must create the directory before returning it: {}",
            dir.display()
        );
        let _ = std::fs::remove_dir_all(&dir);
    }
}
