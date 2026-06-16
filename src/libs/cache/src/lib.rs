// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//! Bounded LRU cache backed by `BTreeMap`.
//!
//! Entries are stored in a `BTreeMap` keyed by a caller-chosen type `K` that
//! implements `Ord + Clone`. A monotonic counter tracks access order; on
//! eviction the entry with the smallest (oldest) counter value is removed.
//! Lookups bump the counter to mark the entry as recently used.
//!
//! # Capacity
//!
//! The cache is bounded by a capacity given at construction time. When the
//! cache is full, the least-recently-used entry is evicted before inserting a
//! new one.
//!
//! # Example
//!
//! ```
//! use cache::Cache;
//!
//! let mut cache: Cache<&str, i32> = Cache::new(2);
//! cache.put("a", 1);
//! cache.put("b", 2);
//! assert_eq!(*cache.get(&"a").unwrap(), 1);
//!
//! // Inserting a third entry evicts "b" (least recently used).
//! cache.put("c", 3);
//! assert!(cache.get(&"b").is_none());
//! assert_eq!(*cache.get(&"c").unwrap(), 3);
//! ```

#![cfg_attr(not(feature = "std"), no_std)]

//==================================================================================================
// Imports
//==================================================================================================

extern crate alloc;

use ::alloc::collections::BTreeMap;
use ::core::ops::{
    Deref,
    DerefMut,
};

//==================================================================================================
// Cache Entry
//==================================================================================================

/// A single entry in the cache.
#[derive(Clone)]
struct CacheEntry<V> {
    /// Cached value.
    value: V,
    /// Last-access counter (higher = more recently used).
    last_used: u64,
}

//==================================================================================================
// Cache Guard
//==================================================================================================

///
/// # Description
///
/// RAII guard providing a reference to a cached entry.
///
/// Created by [`Cache::get()`]. Dereferences to `&V` for read access
/// and `&mut V` for write access. The LRU counter is bumped when the guard
/// is created.
///
pub struct CacheGuard<'a, V> {
    /// Mutable reference to the cached value.
    value: &'a mut V,
}

impl<V> Deref for CacheGuard<'_, V> {
    type Target = V;

    fn deref(&self) -> &V {
        self.value
    }
}

impl<V> DerefMut for CacheGuard<'_, V> {
    fn deref_mut(&mut self) -> &mut V {
        self.value
    }
}

//==================================================================================================
// Cache
//==================================================================================================

///
/// # Description
///
/// A bounded cache using access-counter LRU approximation.
///
/// Entries are stored in a `BTreeMap` keyed by `K`. A monotonic counter tracks access order;
/// on eviction the entry with the smallest (oldest) counter value is removed. Lookups bump the
/// counter to mark the entry as recently used.
///
pub struct Cache<K, V> {
    /// Cached entries.
    entries: BTreeMap<K, CacheEntry<V>>,
    /// Monotonically increasing counter for LRU ordering.
    counter: u64,
    /// Maximum number of entries.
    capacity: usize,
}

impl<K: Ord + Clone, V> Cache<K, V> {
    ///
    /// # Description
    ///
    /// Creates a new empty cache with the given capacity.
    ///
    /// # Parameters
    ///
    /// - `capacity`: Maximum number of entries the cache can hold.
    ///
    pub const fn new(capacity: usize) -> Self {
        Self {
            entries: BTreeMap::new(),
            counter: 0,
            capacity,
        }
    }

    ///
    /// # Description
    ///
    /// Looks up a cached value and bumps its LRU counter on hit.
    ///
    /// # Parameters
    ///
    /// - `key`: The cache key to look up.
    ///
    /// # Returns
    ///
    /// An RAII guard providing access to the cached value, or `None` on cache miss.
    ///
    pub fn get(&mut self, key: &K) -> Option<CacheGuard<'_, V>> {
        if let Some(entry) = self.entries.get_mut(key) {
            self.counter += 1;
            entry.last_used = self.counter;
            Some(CacheGuard {
                value: &mut entry.value,
            })
        } else {
            None
        }
    }

    ///
    /// # Description
    ///
    /// Inserts a value, evicting the least-recently-used entry if at capacity.
    ///
    /// # Parameters
    ///
    /// - `key`: The cache key to insert or update.
    /// - `value`: The value to store.
    ///
    pub fn put(&mut self, key: K, value: V) {
        // A zero-capacity cache cannot store entries.
        if self.capacity == 0 {
            return;
        }

        // If the key is already present, update in place.
        if let Some(entry) = self.entries.get_mut(&key) {
            self.counter += 1;
            entry.value = value;
            entry.last_used = self.counter;
            return;
        }

        // Evict the LRU entry if at capacity.
        if self.entries.len() >= self.capacity {
            self.evict();
        }

        self.counter += 1;
        self.entries.insert(
            key,
            CacheEntry {
                value,
                last_used: self.counter,
            },
        );
    }

    ///
    /// # Description
    ///
    /// Removes a specific key.
    ///
    /// # Parameters
    ///
    /// - `key`: The cache key to remove.
    ///
    pub fn remove(&mut self, key: &K) {
        self.entries.remove(key);
    }

    ///
    /// # Description
    ///
    /// Removes all entries.
    ///
    pub fn clear(&mut self) {
        self.entries.clear();
        self.counter = 0;
    }

    ///
    /// # Description
    ///
    /// Evicts the entry with the smallest `last_used` counter.
    ///
    fn evict(&mut self) {
        let victim: Option<K> = self
            .entries
            .iter()
            .min_by_key(|(_, e)| e.last_used)
            .map(|(k, _)| k.clone());
        if let Some(key) = victim {
            self.entries.remove(&key);
        }
    }
}

//==================================================================================================
// Tests
//==================================================================================================

#[cfg(all(test, feature = "std"))]
mod tests {
    use super::*;

    #[test]
    fn get_returns_none_on_miss() {
        let mut cache: Cache<&str, i32> = Cache::new(4);
        assert!(cache.get(&"missing").is_none());
    }

    #[test]
    fn put_then_get() {
        let mut cache: Cache<&str, i32> = Cache::new(4);
        cache.put("a", 1);
        assert_eq!(*cache.get(&"a").unwrap(), 1);
    }

    #[test]
    fn put_overwrites_existing_key() {
        let mut cache: Cache<&str, i32> = Cache::new(4);
        cache.put("a", 1);
        cache.put("a", 2);
        assert_eq!(*cache.get(&"a").unwrap(), 2);
    }

    #[test]
    fn evicts_lru_entry_when_full() {
        let mut cache: Cache<&str, i32> = Cache::new(2);
        cache.put("a", 1);
        cache.put("b", 2);
        // "a" is LRU; inserting "c" should evict it.
        cache.put("c", 3);
        assert!(cache.get(&"a").is_none());
        assert_eq!(*cache.get(&"b").unwrap(), 2);
        assert_eq!(*cache.get(&"c").unwrap(), 3);
    }

    #[test]
    fn get_refreshes_lru_order() {
        let mut cache: Cache<&str, i32> = Cache::new(2);
        cache.put("a", 1);
        cache.put("b", 2);
        // Touch "a" so "b" becomes LRU.
        let _ = cache.get(&"a");
        cache.put("c", 3);
        assert_eq!(*cache.get(&"a").unwrap(), 1);
        assert!(cache.get(&"b").is_none());
        assert_eq!(*cache.get(&"c").unwrap(), 3);
    }

    #[test]
    fn remove_deletes_key() {
        let mut cache: Cache<&str, i32> = Cache::new(4);
        cache.put("a", 1);
        cache.remove(&"a");
        assert!(cache.get(&"a").is_none());
    }

    #[test]
    fn remove_nonexistent_key_is_noop() {
        let mut cache: Cache<&str, i32> = Cache::new(4);
        cache.remove(&"ghost");
        assert!(cache.get(&"ghost").is_none());
    }

    #[test]
    fn clear_removes_all_entries() {
        let mut cache: Cache<&str, i32> = Cache::new(4);
        cache.put("a", 1);
        cache.put("b", 2);
        cache.clear();
        assert!(cache.get(&"a").is_none());
        assert!(cache.get(&"b").is_none());
    }

    #[test]
    fn capacity_one() {
        let mut cache: Cache<&str, i32> = Cache::new(1);
        cache.put("a", 1);
        cache.put("b", 2);
        assert!(cache.get(&"a").is_none());
        assert_eq!(*cache.get(&"b").unwrap(), 2);
    }

    #[test]
    fn overwrite_does_not_evict() {
        let mut cache: Cache<&str, i32> = Cache::new(2);
        cache.put("a", 1);
        cache.put("b", 2);
        // Overwrite "a" — should not trigger eviction.
        cache.put("a", 10);
        assert_eq!(*cache.get(&"a").unwrap(), 10);
        assert_eq!(*cache.get(&"b").unwrap(), 2);
    }
}
