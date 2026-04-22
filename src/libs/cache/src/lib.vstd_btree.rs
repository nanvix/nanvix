// BTreeMap specifications for no_std targets.
//
// Copied from vstd v0.0.0-2026-04-05 (std_specs/btree.rs) and adapted to use
// `alloc::collections::BTreeMap` instead of `std::collections::BTreeMap`.
//
// The upstream specs are gated behind `cfg(all(feature = "alloc", feature = "std"))`,
// making them unavailable on no_std kernel targets. The specifications here are
// semantically identical — only the import path differs.
//
// IMPORTANT: This file should be kept in sync with the upstream vstd btree specs.
// If vstd adds `cfg(alloc)` support for btree specs, this file can be removed.
//
// Source: ~/.cargo/registry/src/.../vstd-0.0.0-2026-04-05-0114/std_specs/btree.rs
// License: MIT / Apache-2.0 (same as vstd)

verus! {

//==================================================================================================
// External Type Specifications
//==================================================================================================

#[verifier::external_type_specification]
#[verifier::external_body]
#[verifier::accept_recursive_types(Key)]
#[verifier::accept_recursive_types(Value)]
#[verifier::reject_recursive_types(A)]
pub struct ExBTreeMap<Key, Value, A: core::alloc::Allocator + Clone>(
    alloc::collections::BTreeMap<Key, Value, A>,
);

#[verifier::external_type_specification]
pub struct ExGlobal(alloc::alloc::Global);

//==================================================================================================
// View for BTreeMap
//==================================================================================================

impl<Key, Value, A: core::alloc::Allocator + Clone> View for alloc::collections::BTreeMap<Key, Value, A> {
    type V = Map<Key, Value>;

    uninterp spec fn view(&self) -> Map<Key, Value>;
}

/// A BTreeMap always has a finite domain.
pub broadcast axiom fn axiom_btree_map_view_finite_dom<K, V>(m: alloc::collections::BTreeMap<K, V>)
    ensures
        #[trigger] m@.dom().finite(),
;

//==================================================================================================
// BTreeMap method specifications
//==================================================================================================

// --- new ---
pub assume_specification<Key, Value>[ alloc::collections::BTreeMap::<Key, Value>::new ]()
    -> (m: alloc::collections::BTreeMap<Key, Value>)
    ensures
        m@ == Map::<Key, Value>::empty(),
;

// --- len ---
pub uninterp spec fn spec_btree_map_len<Key, Value>(
    m: &alloc::collections::BTreeMap<Key, Value>,
) -> usize;

pub broadcast axiom fn axiom_spec_btree_map_len<Key, Value>(
    m: &alloc::collections::BTreeMap<Key, Value>,
)
    ensures
        #[trigger] spec_btree_map_len(m) == m@.len(),
;

#[verifier::when_used_as_spec(spec_btree_map_len)]
pub assume_specification<Key, Value>[ alloc::collections::BTreeMap::<Key, Value>::len ](
    m: &alloc::collections::BTreeMap<Key, Value>,
) -> (len: usize)
    ensures
        len == spec_btree_map_len(m),
;

// --- is_empty ---
pub assume_specification<Key, Value>[ alloc::collections::BTreeMap::<Key, Value>::is_empty ](
    m: &alloc::collections::BTreeMap<Key, Value>,
) -> (res: bool)
    ensures
        res == m@.is_empty(),
;

// --- insert ---
pub assume_specification<Key: Ord, Value>[ alloc::collections::BTreeMap::<Key, Value>::insert ](
    m: &mut alloc::collections::BTreeMap<Key, Value>,
    k: Key,
    v: Value,
) -> (result: Option<Value>)
    ensures
        m@ == old(m)@.insert(k, v),
        match result {
            Some(v) => old(m)@.contains_key(k) && v == old(m)@[k],
            None => !old(m)@.contains_key(k),
        },
;

// --- contains_key (Q = K) ---
pub assume_specification<Key: Ord, Value>[ alloc::collections::BTreeMap::<Key, Value>::contains_key::<Key> ](
    m: &alloc::collections::BTreeMap<Key, Value>,
    k: &Key,
) -> (result: bool)
    ensures
        result == m@.contains_key(*k),
;

// --- get (Q = K) ---
pub assume_specification<'a, Key: Ord, Value>[ alloc::collections::BTreeMap::<Key, Value>::get::<Key> ](
    m: &'a alloc::collections::BTreeMap<Key, Value>,
    k: &Key,
) -> (result: Option<&'a Value>)
    ensures
        match result {
            Some(v) => m@.contains_key(*k) && *v == m@[*k],
            None => !m@.contains_key(*k),
        },
;

// --- get_mut (Q = K) ---
// NOTE: vstd does NOT provide get_mut specs even for std targets.
// This is a new specification.
pub assume_specification<'a, Key: Ord, Value>[ alloc::collections::BTreeMap::<Key, Value>::get_mut::<Key> ](
    m: &'a mut alloc::collections::BTreeMap<Key, Value>,
    k: &Key,
) -> (result: Option<&'a mut Value>)
    ensures
        // Domain unchanged.
        m@.dom() == old(m)@.dom(),
        match result {
            Some(v) => {
                &&& old(m)@.contains_key(*k)
                &&& *v == old(m)@[*k]
                // All other keys unchanged.
                &&& forall |j: Key| j != *k && old(m)@.contains_key(j) ==> m@[j] == old(m)@[j]
            },
            None => !old(m)@.contains_key(*k) && m@ == old(m)@,
        },
;

// --- remove (Q = K) ---
pub assume_specification<Key: Ord, Value>[ alloc::collections::BTreeMap::<Key, Value>::remove::<Key> ](
    m: &mut alloc::collections::BTreeMap<Key, Value>,
    k: &Key,
) -> (result: Option<Value>)
    ensures
        m@ == old(m)@.remove(*k),
        match result {
            Some(v) => old(m)@.contains_key(*k) && v == old(m)@[*k],
            None => !old(m)@.contains_key(*k),
        },
;

// --- clear ---
pub assume_specification<Key, Value>[ alloc::collections::BTreeMap::<Key, Value>::clear ](
    m: &mut alloc::collections::BTreeMap<Key, Value>,
)
    ensures
        m@ == Map::<Key, Value>::empty(),
;

// --- decreases ---
pub broadcast axiom fn axiom_btree_map_decreases<Key, Value>(
    m: alloc::collections::BTreeMap<Key, Value>,
)
    ensures
        #[trigger] (decreases_to!(m => m@)),
;

//==================================================================================================
// Broadcast group
//==================================================================================================

pub broadcast group group_btree_axioms {
    axiom_btree_map_view_finite_dom,
    axiom_spec_btree_map_len,
    axiom_btree_map_decreases,
}

} // verus!
