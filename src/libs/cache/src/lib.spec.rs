// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

// Cache - Specifications
//
// This file contains the CacheView type, invariant, spec transition functions,
// and View trait implementations for Cache and CacheGuard.

verus! {

//==================================================================================================
// External Type Specifications
//==================================================================================================

// BTreeMap is from alloc::collections. vstd provides specs in vstd::std_specs::btree,
// but they require cfg(std) which is incompatible with this crate's no_std kernel target.
// We declare it as an external type so Verus can reference it.
#[verifier::reject_recursive_types(K)]
#[verifier::reject_recursive_types(V)]
#[verifier::reject_recursive_types(A)]
#[verifier::external_type_specification]
#[verifier::external_body]
pub struct ExBTreeMap<K, V, A>(alloc::collections::BTreeMap<K, V, A>)
    where A: core::alloc::Allocator + core::clone::Clone;

#[verifier::external_type_specification]
pub struct ExGlobal(alloc::alloc::Global);

//==================================================================================================
// BTreeMap abstract view (custom spec fn — orphan rules prevent impl View for BTreeMap)
//==================================================================================================

// Uninterpreted spec function mirroring vstd's View::view for BTreeMap.
// The View trait impl is in vstd::std_specs::btree, gated behind cfg(std) which is
// unavailable on this no_std kernel target. We use a standalone spec function instead.
pub uninterp spec fn btreemap_view_spec<K, V>(m: alloc::collections::BTreeMap<K, V>) -> Map<K, V>;

// assume_specification for BTreeMap::new — matches vstd/std_specs/btree.rs:613-616.
pub assume_specification<K, V>[ alloc::collections::BTreeMap::<K, V>::new ]()
    -> (m: alloc::collections::BTreeMap<K, V>)
    ensures
        btreemap_view_spec(m) == Map::<K, V>::empty(),
;

//==================================================================================================
// Cache abstraction functions (connecting concrete fields to abstract CacheView)
//==================================================================================================

// Project BTreeMap<K, CacheEntry<V>> contents to Map<K, V> by extracting CacheEntry::value.
// Closed: CacheEntry is crate-private, so the body can't appear in pub open functions.
spec fn cache_contents_of<K, V>(entries: alloc::collections::BTreeMap<K, CacheEntry<V>>) -> Map<K, V> {
    Map::new(
        |k: K| btreemap_view_spec(entries).dom().contains(k),
        |k: K| btreemap_view_spec(entries)[k].value,
    )
}

// LRU ordering from entries: sorted by last_used ascending.
// For empty entries, definitionally Seq::empty(); otherwise uninterpreted.
spec fn cache_lru_of<K, V>(entries: alloc::collections::BTreeMap<K, CacheEntry<V>>) -> Seq<K> {
    if btreemap_view_spec(entries).dom().len() == 0 {
        Seq::empty()
    } else {
        cache_lru_of_nonempty(entries)
    }
}

uninterp spec fn cache_lru_of_nonempty<K, V>(entries: alloc::collections::BTreeMap<K, CacheEntry<V>>) -> Seq<K>;

// CacheEntry is a private internal type.
#[verifier::reject_recursive_types(V)]
#[verifier::external_type_specification]
struct ExCacheEntry<V>(crate::CacheEntry<V>);

// CacheGuard wraps &mut V; Verus cannot handle &mut in struct fields,
// so we declare it as an external type.
#[verifier::reject_recursive_types(V)]
#[verifier::external_type_specification]
#[verifier::external_body]
pub struct ExCacheGuard<'a, V>(crate::CacheGuard<'a, V>);

//==================================================================================================
// CacheView - Abstract Specification Model
//==================================================================================================

/// Abstract view of a bounded LRU cache.
///
/// - `contents`: the key-value mapping of all cached entries.
/// - `capacity`: maximum number of entries.
/// - `lru_order`: keys ordered from LRU (index 0) to MRU (last element).
#[verifier::ext_equal]
#[verifier::reject_recursive_types(K)]
#[verifier::reject_recursive_types(V)]
pub struct CacheView<K, V> {
    pub contents: Map<K, V>,
    pub capacity: nat,
    pub lru_order: Seq<K>,
}

impl<K, V> CacheView<K, V> {

    //----------------------------------------------------------------------------------------------
    // Well-formedness Invariant
    //----------------------------------------------------------------------------------------------

    /// Well-formedness invariant for the abstract cache state.
    pub open spec fn inv(&self) -> bool {
        // Size never exceeds capacity.
        &&& self.contents.dom().len() <= self.capacity
        // LRU order has no duplicates.
        &&& self.lru_order.no_duplicates()
        // LRU order contains exactly the stored keys.
        &&& self.lru_order.to_set() == self.contents.dom()
        // Explicit cardinality link (helps the solver).
        &&& self.lru_order.len() == self.contents.dom().len()
    }

    //----------------------------------------------------------------------------------------------
    // Helper Spec Functions
    //----------------------------------------------------------------------------------------------

    /// Move `key` to the most-recently-used position (end of the sequence).
    /// Preserves all other elements and their relative order.
    pub open spec fn move_to_mru(self, key: K) -> Seq<K> {
        self.lru_order.filter(|k: K| k != key).push(key)
    }

    //----------------------------------------------------------------------------------------------
    // Spec Transition Functions
    //----------------------------------------------------------------------------------------------

    /// Spec transition for `Cache::new`.
    pub open spec fn spec_new(capacity: nat) -> CacheView<K, V> {
        CacheView {
            contents: Map::empty(),
            capacity,
            lru_order: Seq::empty(),
        }
    }

    /// Spec transition for `Cache::get`.
    /// Returns the updated cache state and the lookup result.
    pub open spec fn spec_get(self, key: K) -> (CacheView<K, V>, Option<V>) {
        if self.contents.dom().contains(key) {
            (CacheView {
                lru_order: self.move_to_mru(key),
                ..self
            }, Some(self.contents[key]))
        } else {
            (self, None)
        }
    }

    /// Spec transition for `Cache::put`.
    pub open spec fn spec_put(self, key: K, value: V) -> CacheView<K, V> {
        if self.capacity == 0 {
            // Zero-capacity cache: no-op.
            self
        } else if self.contents.dom().contains(key) {
            // Overwrite existing key: replace value, refresh recency.
            CacheView {
                contents: self.contents.insert(key, value),
                lru_order: self.move_to_mru(key),
                ..self
            }
        } else if self.contents.dom().len() >= self.capacity {
            // At capacity with new key: evict LRU victim, then insert.
            let victim = self.lru_order[0];
            CacheView {
                contents: self.contents.remove(victim).insert(key, value),
                lru_order: self.lru_order.subrange(1, self.lru_order.len() as int).push(key),
                ..self
            }
        } else {
            // Below capacity with new key: insert directly.
            CacheView {
                contents: self.contents.insert(key, value),
                lru_order: self.lru_order.push(key),
                ..self
            }
        }
    }

    /// Spec transition for `Cache::remove`.
    pub open spec fn spec_remove(self, key: K) -> CacheView<K, V> {
        if self.contents.dom().contains(key) {
            CacheView {
                contents: self.contents.remove(key),
                lru_order: self.lru_order.filter(|k: K| k != key),
                ..self
            }
        } else {
            // Key absent: no-op.
            self
        }
    }

    /// Spec transition for `Cache::clear`.
    pub open spec fn spec_clear(self) -> CacheView<K, V> {
        CacheView {
            contents: Map::empty(),
            lru_order: Seq::empty(),
            ..self  // capacity preserved
        }
    }
}

//==================================================================================================
// View Implementation for Cache
//==================================================================================================

// Cache View — interpreted via btreemap_view_spec + abstraction helpers.
// Closed: CacheEntry is private, so the body can't be pub open.
// Within this crate, use reveal(Cache::view) to expose the structure in proofs.
impl<K: Ord + Clone, V> View for Cache<K, V> {
    type V = CacheView<K, V>;

    closed spec fn view(&self) -> CacheView<K, V> {
        CacheView {
            contents: cache_contents_of(self.entries),
            capacity: self.capacity as nat,
            lru_order: cache_lru_of(self.entries),
        }
    }
}

//==================================================================================================
// View Implementation for CacheGuard
//==================================================================================================

// CacheGuard is external_body (Verus limitation: &mut in struct fields),
// so the view is uninterpreted. Constraints come from get/deref ensures.
impl<'a, V> View for CacheGuard<'a, V> {
    type V = V;

    uninterp spec fn view(&self) -> V;
}

} // verus!

