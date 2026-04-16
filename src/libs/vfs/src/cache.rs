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
//! - **Raw region cache**: Maps paths to `(ptr, size)` for zero-copy direct
//!   read handles.
//!
//! # Capacity
//!
//! Each cache is bounded to [`config::vfs::CACHE_CAPACITY`] entries. When
//! full, the least-recently-used entry (approximated by access counter) is
//! evicted before inserting a new one.
//!
//! # Thread Safety
//!
//! All caches are protected by `spin::Mutex` for safe concurrent access.
//!
//! # Invalidation
//!
//! Caches are invalidated when the directory structure changes (`mkdir`,
//! `rmdir`, `unlink`, `rename`). File creations, truncations, and writes do
//! not currently trigger automatic invalidation of cached entries, so this
//! cache is intended primarily for read-only or read-mostly workloads unless
//! callers perform explicit invalidation on such write paths. For read-only
//! filesystems (e.g., ramfs in standalone mode), invalidation never fires and
//! the caches persist for the VM's lifetime. All caches are also bulk-
//! invalidated on mount and unmount operations.

//==================================================================================================
// Imports
//==================================================================================================

use crate::file::Stat;
use ::alloc::string::String;
use ::cache::Cache;
use ::config::vfs::CACHE_CAPACITY;
use ::spin::Mutex;

//==================================================================================================
// Raw Region Pointer Wrapper
//==================================================================================================

/// A `Copy` wrapper around an optional `(*const u8, usize)` pair.
///
/// This exists solely to confine the `unsafe impl Send` to the raw-pointer
/// type rather than applying a blanket impl over all `Cache<V>`.
#[derive(Clone, Copy)]
pub(crate) struct RawRegion(pub Option<(*const u8, usize)>);

// SAFETY: The wrapped pointers reference FAT image memory that either lives
// for the program lifetime (host-provided mounts) or is invalidated via
// `cache::invalidate_all()` before deallocation (guest-created mounts via
// `state::unmount`). Access is serialized through `spin::Mutex`.
//
// NOTE: A TOCTOU window exists between reading a cached pointer and a
// concurrent `unmount` that frees the underlying memory. This is the same
// class of risk that `DirectReadHandle` already carries (it holds a raw
// pointer outside the VFS lock). The `unmount` path mitigates this by
// checking `has_open_files()` and returning `FileLocked` if any FDs remain
// open, which prevents deallocation while the data is still reachable.
unsafe impl Send for RawRegion {}

//==================================================================================================
// Cache State
//==================================================================================================

/// Cached stat results for paths.
static STAT_CACHE: Mutex<Cache<String, Stat>> = Mutex::new(Cache::new(CACHE_CAPACITY));

/// Cached raw region pointers for zero-copy reads.
static RAW_REGION_CACHE: Mutex<Cache<String, RawRegion>> = Mutex::new(Cache::new(CACHE_CAPACITY));

//==================================================================================================
// Public API
//==================================================================================================

///
/// # Description
///
/// Looks up a cached stat result.
///
/// # Parameters
///
/// - `path`: The normalized absolute path to look up.
///
/// # Returns
///
/// `Some(stat)` if the path has been cached, `None` on cache miss.
///
pub(crate) fn get_stat(path: &str) -> Option<Stat> {
    STAT_CACHE.lock().get(path).map(|g| *g)
}

///
/// # Description
///
/// Inserts a stat result into the cache.
///
/// # Parameters
///
/// - `path`: The normalized absolute path to cache.
/// - `stat`: The metadata to store.
///
pub(crate) fn put_stat(path: &str, stat: Stat) {
    STAT_CACHE.lock().put(String::from(path), stat);
}

///
/// # Description
///
/// Looks up a cached raw region result.
///
/// Only positive results are cached; negative lookups always go to FAT.
///
/// # Parameters
///
/// - `path`: The normalized absolute path to look up.
///
/// # Returns
///
/// `Some((ptr, size))` on cache hit, or `None` on cache miss.
///
pub(crate) fn get_raw_region(path: &str) -> Option<(*const u8, usize)> {
    RAW_REGION_CACHE.lock().get(path).and_then(|g| g.0)
}

///
/// # Description
///
/// Inserts a raw region result into the cache.
///
/// # Parameters
///
/// - `path`: The normalized absolute path to cache.
/// - `region`: The raw pointer and size pair, or `None` if no contiguous
///   region exists.
///
pub(crate) fn put_raw_region(path: &str, region: Option<(*const u8, usize)>) {
    RAW_REGION_CACHE
        .lock()
        .put(String::from(path), RawRegion(region));
}

///
/// # Description
///
/// Invalidates all cached entries for a specific path.
///
/// Called when a write operation modifies the filesystem (e.g., `mkdir`,
/// `rmdir`, `unlink`, `rename`).
///
/// # Parameters
///
/// - `path`: The normalized absolute path to invalidate.
///
pub(crate) fn invalidate_path(path: &str) {
    STAT_CACHE.lock().remove(path);
    RAW_REGION_CACHE.lock().remove(path);
}

///
/// # Description
///
/// Invalidates the entire cache.
///
/// Called when a bulk filesystem modification occurs (e.g., mount/unmount).
///
pub(crate) fn invalidate_all() {
    STAT_CACHE.lock().clear();
    RAW_REGION_CACHE.lock().clear();
}
