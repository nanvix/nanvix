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
#![cfg_attr(verus_keep_ghost, feature(proc_macro_hygiene))]
#![cfg_attr(verus_keep_ghost, feature(allocator_api))]

//==================================================================================================
// Imports
//==================================================================================================

extern crate alloc;

use ::alloc::collections::BTreeMap;
use ::core::ops::{
    Deref,
    DerefMut,
};

use vstd::prelude::*;
#[cfg(verus_keep_ghost)]
include!("lib.vstd_btree.rs");
#[cfg(verus_keep_ghost)]
include!("lib.spec.rs");
#[cfg(verus_keep_ghost)]
include!("lib.proof.rs");


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

#[verus_verify]
impl<V> Deref for CacheGuard<'_, V> {
    type Target = V;

    #[verus_verify(external_body)]
    #[verus_spec(ret =>
        ensures *ret == self@,
    )]
    fn deref(&self) -> &V { ... }
}

impl<V> DerefMut for CacheGuard<'_, V> {
    fn deref_mut(&mut self) -> &mut V { ... }
}

//==================================================================================================
// Stdlib Wrappers
//==================================================================================================

/// Stdlib wrapper for `BTreeMap::remove`. Needed because `BTreeMap::remove`'s full
/// generic signature (`Borrow<Q>`, `Allocator`) is complex. This wrapper fixes Q=K.
#[verus_verify(external_body)]
#[verus_spec(ret =>
    ensures
        btreemap_view_spec(*m) == btreemap_view_spec(*old(m)).remove(*k),
        ret.is_some() <==> btreemap_view_spec(*old(m)).dom().contains(*k),
        ret.is_some() ==> ret == Some(btreemap_view_spec(*old(m))[*k]),
)]
fn btreemap_remove<K: Ord, V>(m: &mut BTreeMap<K, V>, k: &K) -> Option<V> { ... }

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
#[verus_verify(reject_recursive_types(K))]
#[verus_verify(reject_recursive_types(V))]
pub struct Cache<K, V> {
    /// Cached entries.
    entries: BTreeMap<K, CacheEntry<V>>,
    /// Monotonically increasing counter for LRU ordering.
    counter: u64,
    /// Maximum number of entries.
    capacity: usize,
}

#[verus_verify]
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
    #[verus_spec(result =>
        ensures
            result@ == CacheView::<K, V>::spec_new(capacity as nat),
            result@.inv(),
    )]
    pub const fn new(capacity: usize) -> Self { ... }

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
    #[verus_verify(external_body)]
    #[verus_spec(result =>
        requires
            old(self)@.inv(),
        ensures
            // Hit: key is present.
            old(self)@.contents.dom().contains(*key) ==> {
                &&& result is Some
                &&& result->Some_0@ == old(self)@.spec_get(*key).1.unwrap()
                &&& self@ == old(self)@.spec_get(*key).0
                &&& self@.inv()
            },
            // Miss: key is absent.
            !old(self)@.contents.dom().contains(*key) ==> {
                &&& result is None
                &&& self@ == old(self)@
            },
    )]
    pub fn get(&mut self, key: &K) -> Option<CacheGuard<'_, V>> { ... }

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
    #[verus_verify(external_body)]
    #[verus_spec(
        requires
            old(self)@.inv(),
        ensures
            self@ == old(self)@.spec_put(key, value),
            self@.inv(),
    )]
    pub fn put(&mut self, key: K, value: V) { ... }

    ///
    /// # Description
    ///
    /// Removes a specific key.
    ///
    /// # Parameters
    ///
    /// - `key`: The cache key to remove.
    ///
    #[verus_spec(
        requires
            old(self)@.inv(),
        ensures
            self@ == old(self)@.spec_remove(*key),
            self@.inv(),
    )]
    pub fn remove(&mut self, key: &K) { ... }

    ///
    /// # Description
    ///
    /// Removes all entries.
    ///
    #[verus_verify(external_body)]
    #[verus_spec(
        requires
            old(self)@.inv(),
        ensures
            self@ == old(self)@.spec_clear(),
            self@.inv(),
    )]
    pub fn clear(&mut self) { ... }

    ///
    /// # Description
    ///
    /// Evicts the entry with the smallest `last_used` counter.
    ///
    #[verus_verify(external_body)]
    #[verus_spec(
        requires
            old(self)@.inv(),
            old(self)@.contents.dom().len() > 0,
        ensures
            // The LRU victim (index 0) is evicted.
            !self@.contents.dom().contains(old(self)@.lru_order[0]),
            self@.contents == old(self)@.contents.remove(old(self)@.lru_order[0]),
            self@.contents.dom().len() == old(self)@.contents.dom().len() - 1,
            self@.lru_order == old(self)@.lru_order.subrange(1, old(self)@.lru_order.len() as int),
            self@.capacity == old(self)@.capacity,
            self@.inv(),
    )]
    fn evict(&mut self) { ... }
}

//==================================================================================================
// Tests
//==================================================================================================

#[cfg(all(test, feature = "std"))]
mod tests {
    use super::*;

    #[test]
    fn get_returns_none_on_miss() { ... }

    #[test]
    fn put_then_get() { ... }

    #[test]
    fn put_overwrites_existing_key() { ... }

    #[test]
    fn evicts_lru_entry_when_full() { ... }

    #[test]
    fn get_refreshes_lru_order() { ... }

    #[test]
    fn remove_deletes_key() { ... }

    #[test]
    fn remove_nonexistent_key_is_noop() { ... }

    #[test]
    fn clear_removes_all_entries() { ... }

    #[test]
    fn capacity_one() { ... }

    #[test]
    fn overwrite_does_not_evict() { ... }
}
