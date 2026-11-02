// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//! Path-resolution cache for the VFS mount table.
//!
//! This module provides a small, fixed-capacity LRU cache that maps a
//! normalized absolute path to the mount that handles it, so that repeated
//! operations on the same path skip the mount-table walk.

//==================================================================================================
// Imports
//==================================================================================================

use ::alloc::{
    string::String,
    vec::Vec,
};

//==================================================================================================
// Constants
//==================================================================================================

/// Maximum number of entries retained by the path-resolution cache.
///
/// Kept small to bound guest memory use; see [`PathCache`].
pub(crate) const PATH_CACHE_CAPACITY: usize = 16;

//==================================================================================================
// Structures
//==================================================================================================

/// A cached path-resolution result.
///
/// Maps a normalized absolute path to the mount that handles it.
struct PathCacheEntry {
    /// Normalized absolute path used as the lookup key.
    key: String,
    /// Index of the mount that handles `key`.
    mount_index: usize,
    /// Path relative to the mount root.
    relative: String,
}

/// Fixed-capacity, array-backed LRU cache for path resolution.
///
/// Maps a normalized absolute path to its `(mount_index, relative_path)`
/// resolution so that repeated operations on the same path (e.g.,
/// `open` → `read` → `stat` → `close`) avoid walking the mount table each
/// time.
///
/// # Design
///
/// `no_std`-friendly: a single [`Vec`] bounded by [`PATH_CACHE_CAPACITY`]
/// with move-to-front recency ordering (index `0` is the most recently used,
/// the tail is the least recently used and is evicted first). No `HashMap`
/// and no per-lookup allocation beyond cloning the stored result.
///
/// # Correctness
///
/// Entries are keyed by the *normalized* absolute path, so the mapping only
/// becomes stale when the mount table itself changes. The cache is therefore
/// cleared whenever a mount is added or removed; changing the working
/// directory does not require invalidation because normalization is applied
/// before the lookup.
pub(crate) struct PathCache {
    /// Cached entries ordered most-recently-used first.
    entries: Vec<PathCacheEntry>,
}

//==================================================================================================
// Implementations
//==================================================================================================

impl PathCache {
    /// Creates a new, empty cache.
    ///
    /// The backing storage is pre-allocated to [`PATH_CACHE_CAPACITY`] so the
    /// cache never reallocates over its lifetime, keeping the memory bound
    /// explicit.
    pub(crate) fn new() -> Self {
        Self {
            entries: Vec::with_capacity(PATH_CACHE_CAPACITY),
        }
    }

    /// Looks up a normalized path, returning its cached resolution.
    ///
    /// On a hit, the entry is promoted to most-recently-used.
    ///
    /// # Parameters
    ///
    /// - `key`: The normalized absolute path to look up.
    ///
    /// # Returns
    ///
    /// `Some((mount_index, relative_path))` on a cache hit, `None` otherwise.
    pub(crate) fn get(&mut self, key: &str) -> Option<(usize, String)> {
        let pos: usize = self.entries.iter().position(|e| e.key == key)?;
        if pos != 0 {
            let entry: PathCacheEntry = self.entries.remove(pos);
            self.entries.insert(0, entry);
        }
        let entry: &PathCacheEntry = &self.entries[0];
        Some((entry.mount_index, entry.relative.clone()))
    }

    /// Inserts (or refreshes) a resolution, evicting the LRU entry if full.
    ///
    /// # Parameters
    ///
    /// - `key`: The normalized absolute path.
    /// - `mount_index`: Index of the mount that handles `key`.
    /// - `relative`: Path relative to the mount root.
    pub(crate) fn insert(&mut self, key: String, mount_index: usize, relative: String) {
        if let Some(pos) = self.entries.iter().position(|e| e.key == key) {
            self.entries.remove(pos);
        } else if self.entries.len() >= PATH_CACHE_CAPACITY {
            // Evict the least-recently-used entry (the tail).
            self.entries.pop();
        }
        self.entries.insert(
            0,
            PathCacheEntry {
                key,
                mount_index,
                relative,
            },
        );
    }

    /// Drops all cached entries.
    pub(crate) fn clear(&mut self) {
        self.entries.clear();
    }
}

//==================================================================================================
// Unit Tests
//==================================================================================================

#[cfg(test)]
mod tests;
