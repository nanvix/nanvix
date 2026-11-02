// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

use super::*;

impl PathCache {
    /// Returns the number of entries currently held by the cache.
    ///
    /// Test-only helper used to assert caching and eviction behavior.
    pub(crate) fn entry_count(&self) -> usize {
        self.entries.len()
    }
}

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
    assert_eq!(cache.get("/a"), Some((1, String::from("new"))), "the most recent insert must win");
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
        cache.insert(::alloc::format!("/data/file{}.txt", i), i, ::alloc::format!("file{}.txt", i));
        assert!(cache.entry_count() <= PATH_CACHE_CAPACITY, "cache must never exceed its capacity");
    }
    assert_eq!(cache.entry_count(), PATH_CACHE_CAPACITY, "cache should be saturated at capacity");

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
