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
fn btreemap_remove<K: Ord, V>(m: &mut BTreeMap<K, V>, k: &K) -> Option<V> {
    m.remove(k)
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
    pub const fn new(capacity: usize) -> Self {
        let result = Self {
            entries: BTreeMap::new(),
            counter: 0,
            capacity,
        };
        proof! {
            Self::lemma_new_view(&result, capacity);
        }
        result
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
    #[verus_spec(
        requires
            old(self)@.inv(),
        ensures
            self@ == old(self)@.spec_put(key, value),
            self@.inv(),
    )]
    pub fn put(&mut self, key: K, value: V) {
        // A zero-capacity cache cannot store entries.
        if self.capacity == 0 {
            return;
        }

        // VERUS REWRITE: replace get_mut with remove+insert
        // Avoids &mut reference from get_mut (unsupported by Verus).
        let existed = btreemap_remove(&mut self.entries, &key);

        proof_decl! {
            let ghost mut entries_after_remove;
            let ghost mut pre_insert_entries;
        }

        proof! {
            entries_after_remove = self.entries;
        }

        if existed.is_none() {
            // Key was absent — need to prove inv holds for evict's precondition.
            proof! {
                reveal(<Cache<_, _> as View>::view);
                reveal(cache_contents_of);
                reveal(cache_lru_of);
                broadcast use axiom_spec_btree_map_len, axiom_btree_map_view_finite_dom,
                    vstd::set::group_set_axioms, vstd::map::group_map_axioms;

                // Map identity: removing absent key doesn't change the map.
                assert(btreemap_view_spec(self.entries)
                    =~= btreemap_view_spec(old(self).entries));

                assert(cache_contents_of(self.entries)
                    =~= cache_contents_of(old(self).entries));

                // Key not in LRU sequence (from inv: to_set == contents.dom(),
                // and key not in dom because existed.is_none()).
                let old_lru = cache_lru_of(old(self).entries);
                assert(!old(self)@.contents.dom().contains(key));
                assert(!old_lru.to_set().contains(key));
                assert(!old_lru.contains(key));

                // cache_lru_of: axiom gives filter, filter identity for absent key.
                axiom_cache_lru_of_remove(old(self).entries, self.entries, key);
                lemma_filter_neq_absent(old_lru, key);
            }

            // New key — may need eviction.
            if self.entries.len() >= self.capacity {
                proof! {
                    reveal(<Cache<_, _> as View>::view);
                    reveal(cache_contents_of);
                    broadcast use axiom_spec_btree_map_len, axiom_btree_map_view_finite_dom;
                    // entries.len() >= capacity > 0 ⟹ contents non-empty.
                    assert(cache_contents_of(self.entries).dom()
                        =~= btreemap_view_spec(self.entries).dom());
                    assert(self@.contents.dom().len() > 0);
                }
                self.evict();
            }
        }

        proof! {
            pre_insert_entries = self.entries;
        }

        self.counter = if self.counter < u64::MAX {
            self.counter + 1
        } else {
            self.counter
        };
        self.entries.insert(
            key,
            CacheEntry {
                value,
                last_used: self.counter,
            },
        );

        proof! {
            broadcast use vstd::set::group_set_axioms, vstd::map::group_map_axioms,
                vstd::seq_lib::seq_to_set_is_finite,
                axiom_spec_btree_map_len, axiom_btree_map_view_finite_dom;

            reveal(<Cache<_, _> as View>::view);
            reveal(cache_contents_of);
            reveal(cache_lru_of);

            // Apply remove axiom on old(self).entries -> entries_after_remove.
            axiom_cache_lru_of_remove(old(self).entries, entries_after_remove, key);

            // Apply insert axiom on pre_insert_entries -> self.entries.
            axiom_cache_lru_of_insert(
                pre_insert_entries,
                self.entries,
                key,
                CacheEntry { value, last_used: self.counter },
            );

            // Key intermediate: map_values distributes over insert, so
            // cache_contents_of(inserted) =~= cache_contents_of(before).insert(key, value).
            assert(cache_contents_of(self.entries)
                =~= cache_contents_of(pre_insert_entries).insert(key, value));

            let old_view = old(self)@;

            if existed.is_some() {
                // pre_insert_entries = entries_after_remove (key removed).
                assert(cache_contents_of(pre_insert_entries)
                    =~= old_view.contents.remove(key));
                // remove(key).insert(key, value) =~= insert(key, value)
                assert(cache_contents_of(self.entries)
                    =~= old_view.contents.insert(key, value));
            } else if old_view.contents.dom().len() >= old_view.capacity {
                // Evict case: pre_insert_entries = entries after evict.
                let victim = old_view.lru_order[0];

                // Re-establish identity: entries_after_remove == old entries (absent key).
                assert(cache_contents_of(entries_after_remove)
                    =~= old_view.contents);
                assert(cache_lru_of(entries_after_remove) == old_view.lru_order);

                // Evict postcondition unfolded through ghost variables:
                // post_evict_self@.contents == pre_evict_self@.contents.remove(victim)
                // <=> cache_contents_of(pre_insert_entries)
                //     == cache_contents_of(entries_after_remove).remove(
                //          cache_lru_of(entries_after_remove)[0])
                assert(cache_contents_of(pre_insert_entries)
                    == cache_contents_of(entries_after_remove).remove(
                        cache_lru_of(entries_after_remove)[0]));

                // Substitute identities:
                assert(cache_contents_of(entries_after_remove).remove(victim)
                    =~= old_view.contents.remove(victim));
                assert(cache_contents_of(pre_insert_entries)
                    =~= old_view.contents.remove(victim));

                assert(cache_contents_of(self.entries)
                    =~= old_view.contents.remove(victim).insert(key, value));
            } else {
                // Simple insert: btreemap_remove was identity (absent key).
                assert(cache_contents_of(pre_insert_entries)
                    =~= old_view.contents);
                assert(cache_contents_of(self.entries)
                    =~= old_view.contents.insert(key, value));
            }

            // spec_put preserves inv.
            lemma_spec_put_inv(old(self)@, key, value);
        }
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
    #[verus_spec(
        requires
            old(self)@.inv(),
        ensures
            self@ == old(self)@.spec_remove(*key),
            self@.inv(),
    )]
    pub fn remove(&mut self, key: &K) {
        // VERUS REWRITE: originally self.entries.remove(key);
        // Wrapper needed because BTreeMap::remove's full generic signature
        // (Borrow<Q>, Allocator) cannot be expressed with btreemap_view_spec.
        btreemap_remove(&mut self.entries, key);
        proof! {
            Self::lemma_remove_view(self, *key, old(self).entries, old(self).capacity);
        }
    }

    ///
    /// # Description
    ///
    /// Removes all entries.
    ///
    #[verus_spec(
        requires
            old(self)@.inv(),
        ensures
            self@ == old(self)@.spec_clear(),
            self@.inv(),
    )]
    pub fn clear(&mut self) {
        self.entries.clear();
        self.counter = 0;
        proof! {
            Self::lemma_clear_view(self, old(self).capacity);
        }
    }

    /// Finds the LRU victim (entry with smallest last_used counter).
    /// Only the iterator chain is unverifiable; isolated as external_body.
    #[verus_verify(external_body)]
    #[verus_spec(ret =>
        ensures
            btreemap_view_spec(*entries).dom().len() > 0 ==> {
                &&& ret is Some
                &&& cache_lru_of(*entries).len() > 0
                &&& ret->Some_0 == cache_lru_of(*entries)[0]
            },
            btreemap_view_spec(*entries).dom().len() == 0 ==> ret is None,
    )]
    fn find_lru_victim(entries: &BTreeMap<K, CacheEntry<V>>) -> Option<K> {
        // VERUS REWRITE: originally inlined in evict as iterator chain
        entries
            .iter()
            .min_by_key(|(_, e)| e.last_used)
            .map(|(k, _)| k.clone())
    }

    ///
    /// # Description
    ///
    /// Evicts the entry with the smallest `last_used` counter.
    ///
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
    fn evict(&mut self) {
        // VERUS REWRITE: extracted iterator chain into find_lru_victim
        if let Some(key) = Self::find_lru_victim(&self.entries) {
            // VERUS REWRITE: originally self.entries.remove(&key)
            btreemap_remove(&mut self.entries, &key);
            proof! {
                Self::lemma_evict_view(self, key, old(self).entries, old(self).capacity);
            }
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
