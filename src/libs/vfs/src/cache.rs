// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//! VFS metadata cache for accelerating repeated path lookups.
//!
//! CPython's import system performs hundreds of `stat()` and `open()` calls
//! during interpreter startup. Each call traverses the FAT32 cluster chain
//! to resolve the path. This module provides caches that eliminate redundant
//! FAT32 traversals:
//!
//! - **Stat cache**: Maps absolute paths to cached `Stat` results.
//! - **Negative cache**: Tracks paths known to not exist (ENOENT).
//! - **Raw region cache**: Maps paths to `(ptr, size)` for zero-copy direct
//!   read handles.
//!
//! # Thread Safety
//!
//! All caches are protected by `spin::Mutex` for safe concurrent access.
//!
//! # Invalidation
//!
//! Caches are invalidated on write operations (`mkdir`, `rmdir`, `unlink`,
//! `rename`, `create`). For read-only filesystems (e.g., ramfs in standalone
//! mode), invalidation never fires and the caches persist for the VM's
//! lifetime.

//==================================================================================================
// Imports
//==================================================================================================

use crate::file::Stat;
use ::alloc::{
    collections::BTreeMap,
    string::String,
};
use ::spin::Mutex;

//==================================================================================================
// Cache State
//==================================================================================================

/// Cached stat result for a path.
struct StatCache {
    entries: BTreeMap<String, Stat>,
}

/// Set of paths known to not exist.
struct NegativeCache {
    entries: BTreeMap<String, ()>,
}

/// Cached raw region pointers for zero-copy reads.
struct RawRegionCache {
    entries: BTreeMap<String, Option<(*const u8, usize)>>,
}

// SAFETY: The raw pointer in RawRegionCache points into the FAT image memory
// which lives for the entire program lifetime. Access is serialized through
// the spin::Mutex.
unsafe impl Send for RawRegionCache {}

static STAT_CACHE: Mutex<StatCache> = Mutex::new(StatCache {
    entries: BTreeMap::new(),
});

static NEGATIVE_CACHE: Mutex<NegativeCache> = Mutex::new(NegativeCache {
    entries: BTreeMap::new(),
});

static RAW_REGION_CACHE: Mutex<RawRegionCache> = Mutex::new(RawRegionCache {
    entries: BTreeMap::new(),
});

//==================================================================================================
// Public API
//==================================================================================================

/// Looks up a cached stat result.
///
/// Returns `Some(stat)` if the path has been cached, `None` on cache miss.
pub fn get_stat(path: &str) -> Option<Stat> {
    STAT_CACHE.lock().entries.get(path).copied()
}

/// Inserts a stat result into the cache.
pub fn put_stat(path: &str, stat: Stat) {
    STAT_CACHE.lock().entries.insert(String::from(path), stat);
}

/// Returns `true` if the path is in the negative cache (known ENOENT).
pub fn is_negative(path: &str) -> bool {
    NEGATIVE_CACHE.lock().entries.contains_key(path)
}

/// Adds a path to the negative cache.
pub fn put_negative(path: &str) {
    NEGATIVE_CACHE
        .lock()
        .entries
        .insert(String::from(path), ());
}

/// Looks up a cached raw region result.
///
/// Returns `Some(Some((ptr, size)))` if contiguous region was cached,
/// `Some(None)` if we cached that the file is not contiguous, or
/// `None` on cache miss.
pub fn get_raw_region(path: &str) -> Option<Option<(*const u8, usize)>> {
    RAW_REGION_CACHE.lock().entries.get(path).copied()
}

/// Inserts a raw region result into the cache.
pub fn put_raw_region(path: &str, region: Option<(*const u8, usize)>) {
    RAW_REGION_CACHE
        .lock()
        .entries
        .insert(String::from(path), region);
}

/// Invalidates all cached entries for a specific path.
///
/// Called when a write operation modifies the filesystem.
pub fn invalidate_path(path: &str) {
    STAT_CACHE.lock().entries.remove(path);
    NEGATIVE_CACHE.lock().entries.remove(path);
    RAW_REGION_CACHE.lock().entries.remove(path);
}

/// Invalidates the entire cache.
///
/// Called when a bulk filesystem modification occurs (e.g., rename across dirs).
pub fn invalidate_all() {
    STAT_CACHE.lock().entries.clear();
    NEGATIVE_CACHE.lock().entries.clear();
    RAW_REGION_CACHE.lock().entries.clear();
}
