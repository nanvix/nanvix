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

// BTreeMap is from alloc::collections and has no vstd specs.
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

// BTreeMap has no vstd specs, so the view is uninterpreted.
// All constraints on the view come from the external_body method ensures clauses.
impl<K: Ord + Clone, V> View for Cache<K, V> {
    type V = CacheView<K, V>;

    uninterp spec fn view(&self) -> CacheView<K, V>;
}

} // verus!

