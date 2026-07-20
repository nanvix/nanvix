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

    /// Returns the number of entries currently held by the cache.
    ///
    /// Test-only helper used to assert caching and eviction behavior.
    #[cfg(test)]
    pub(crate) fn entry_count(&self) -> usize {
        self.entries.len()
    }
}

//==================================================================================================
// Unit Tests
//==================================================================================================

#[cfg(test)]
mod tests {
    use super::*;

    /// Tests that a freshly created cache holds no entries.
    #[test]
    fn new_cache_is_empty() {
        let cache: PathCache = PathCache::new();
        assert_eq!(cache.entry_count(), 0, "a new cache must be empty");
    }

    /// Tests that an inserted resolution can be looked up.
    #[test]
    fn insert_then_get_returns_resolution() {
        let mut cache: PathCache = PathCache::new();
        cache.insert(String::from("/data/file.txt"), 2, String::from("file.txt"));
        assert_eq!(
            cache.get("/data/file.txt"),
            Some((2, String::from("file.txt"))),
            "lookup must return the inserted resolution"
        );
    }

    /// Tests that looking up an absent key returns `None`.
    #[test]
    fn get_miss_returns_none() {
        let mut cache: PathCache = PathCache::new();
        cache.insert(String::from("/data"), 0, String::from(""));
        assert_eq!(cache.get("/other"), None, "absent key must miss");
    }

    /// Tests that a successful lookup promotes the entry to most-recently-used.
    #[test]
    fn get_promotes_to_most_recently_used() {
        let mut cache: PathCache = PathCache::new();
        cache.insert(String::from("/a"), 0, String::from("a"));
        cache.insert(String::from("/b"), 1, String::from("b"));
        cache.insert(String::from("/c"), 2, String::from("c"));

        // Entries are ordered most-recently-used first: [/c, /b, /a].
        // Accessing /a must move it to the front.
        assert!(cache.get("/a").is_some(), "/a should be present");
        assert_eq!(
            cache.entries[0].key.as_str(),
            "/a",
            "the accessed entry must become most-recently-used"
        );
    }

    /// Tests that re-inserting a key refreshes it in place without duplicating.
    #[test]
    fn insert_existing_key_refreshes_without_duplicate() {
        let mut cache: PathCache = PathCache::new();
        cache.insert(String::from("/a"), 0, String::from("old"));
        cache.insert(String::from("/a"), 1, String::from("new"));
        assert_eq!(cache.entry_count(), 1, "re-inserting a key must not duplicate it");
        assert_eq!(
            cache.get("/a"),
            Some((1, String::from("new"))),
            "the most recent insert must win"
        );
    }

    /// Tests that clearing drops all cached entries.
    #[test]
    fn clear_drops_all_entries() {
        let mut cache: PathCache = PathCache::new();
        cache.insert(String::from("/a"), 0, String::from("a"));
        cache.insert(String::from("/b"), 1, String::from("b"));
        cache.clear();
        assert_eq!(cache.entry_count(), 0, "clear must empty the cache");
        assert_eq!(cache.get("/a"), None, "no entry should remain after clear");
    }

    /// Tests that the cache stays within its fixed capacity and that, once
    /// full, the least-recently-used entry is the one evicted.
    #[test]
    fn evicts_least_recently_used_at_capacity() {
        let mut cache: PathCache = PathCache::new();

        // Insert more distinct paths than the cache can hold. Paths are
        // inserted in ascending index order, so lower indices are
        // progressively less-recently-used.
        let total: usize = PATH_CACHE_CAPACITY + 8;
        for i in 0..total {
            cache.insert(
                ::alloc::format!("/data/file{}.txt", i),
                i,
                ::alloc::format!("file{}.txt", i),
            );
            assert!(
                cache.entry_count() <= PATH_CACHE_CAPACITY,
                "cache must never exceed its capacity"
            );
        }
        assert_eq!(
            cache.entry_count(),
            PATH_CACHE_CAPACITY,
            "cache should be saturated at capacity"
        );

        // Only the most recently inserted `PATH_CACHE_CAPACITY` paths survive;
        // the earliest-inserted (least-recently-used) paths were evicted.
        let oldest_retained: usize = total - PATH_CACHE_CAPACITY;
        let newest: usize = total - 1;
        assert_eq!(
            cache.get("/data/file0.txt"),
            None,
            "the least-recently-used entry must have been evicted"
        );
        assert_eq!(
            cache.get(&::alloc::format!("/data/file{}.txt", oldest_retained)),
            Some((oldest_retained, ::alloc::format!("file{}.txt", oldest_retained))),
            "the oldest entry still within capacity must be retained"
        );
        assert_eq!(
            cache.get(&::alloc::format!("/data/file{}.txt", newest)),
            Some((newest, ::alloc::format!("file{}.txt", newest))),
            "the most recently inserted entry must be retained"
        );
    }
}
