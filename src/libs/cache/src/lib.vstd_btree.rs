// BTreeMap specifications for no_std targets.
//
// Copied from vstd v0.0.0-2026-04-05 (std_specs/btree.rs) and adapted to use
// `alloc::collections::BTreeMap` instead of `std::collections::BTreeMap`.
//
// The upstream specs are gated behind `cfg(all(feature = "alloc", feature = "std"))`,
// making them unavailable on no_std kernel targets. The specifications here are
// semantically identical — only the import path differs.
//
// Unlike upstream vstd, we use an uninterpreted `btreemap_view_spec` function
// instead of `impl View for BTreeMap`, because the orphan rule prevents
// implementing the `View` trait (defined in vstd) for `BTreeMap` (defined in alloc)
// from this crate.
//
// Unlike upstream vstd which uses `std::collections::BTreeMap<K, V>` (hiding the
// allocator), `alloc::collections::BTreeMap` exposes the `A: Allocator + Clone`
// parameter, so all assume_specifications must include it.
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
// BTreeMap abstract view (uninterpreted spec function)
//==================================================================================================

// Uninterpreted spec function mirroring vstd's View::view for BTreeMap.
// The View trait impl is in vstd::std_specs::btree, gated behind cfg(std) which is
// unavailable on this no_std kernel target. We use a standalone spec function instead
// because the orphan rule prevents implementing View for BTreeMap from this crate.
pub uninterp spec fn btreemap_view_spec<K, V, A: core::alloc::Allocator + Clone>(
    m: alloc::collections::BTreeMap<K, V, A>,
) -> Map<K, V>;

/// A BTreeMap always has a finite domain.
pub broadcast axiom fn axiom_btree_map_view_finite_dom<K, V, A: core::alloc::Allocator + Clone>(
    m: alloc::collections::BTreeMap<K, V, A>,
)
    ensures
        #[trigger] btreemap_view_spec(m).dom().finite(),
;

//==================================================================================================
// BTreeMap method specifications
//==================================================================================================

// --- new ---
// BTreeMap::new is in `impl<K, V>` (no A parameter).
pub assume_specification<Key, Value>[ alloc::collections::BTreeMap::<Key, Value>::new ]()
    -> (m: alloc::collections::BTreeMap<Key, Value>)
    ensures
        btreemap_view_spec(m) == Map::<Key, Value>::empty(),
;

// --- len ---
pub uninterp spec fn spec_btree_map_len<Key, Value, A: core::alloc::Allocator + Clone>(
    m: &alloc::collections::BTreeMap<Key, Value, A>,
) -> usize;

pub broadcast axiom fn axiom_spec_btree_map_len<Key, Value, A: core::alloc::Allocator + Clone>(
    m: &alloc::collections::BTreeMap<Key, Value, A>,
)
    ensures
        #[trigger] spec_btree_map_len(m) == btreemap_view_spec(*m).len(),
;

#[verifier::when_used_as_spec(spec_btree_map_len)]
pub assume_specification<Key, Value, A: core::alloc::Allocator + Clone>[
    alloc::collections::BTreeMap::<Key, Value, A>::len
](
    m: &alloc::collections::BTreeMap<Key, Value, A>,
) -> (len: usize)
    ensures
        len == spec_btree_map_len(m),
;

// --- is_empty ---
pub assume_specification<Key, Value, A: core::alloc::Allocator + Clone>[
    alloc::collections::BTreeMap::<Key, Value, A>::is_empty
](
    m: &alloc::collections::BTreeMap<Key, Value, A>,
) -> (res: bool)
    ensures
        res == btreemap_view_spec(*m).is_empty(),
;

// --- insert ---
pub assume_specification<Key: Ord, Value, A: core::alloc::Allocator + Clone>[
    alloc::collections::BTreeMap::<Key, Value, A>::insert
](
    m: &mut alloc::collections::BTreeMap<Key, Value, A>,
    k: Key,
    v: Value,
) -> (result: Option<Value>)
    ensures
        btreemap_view_spec(*m) == btreemap_view_spec(*old(m)).insert(k, v),
        match result {
            Some(v) => btreemap_view_spec(*old(m)).contains_key(k)
                && v == btreemap_view_spec(*old(m))[k],
            None => !btreemap_view_spec(*old(m)).contains_key(k),
        },
;

// --- contains_key (Q = K) ---
pub assume_specification<Key: Ord, Value, A: core::alloc::Allocator + Clone>[
    alloc::collections::BTreeMap::<Key, Value, A>::contains_key::<Key>
](
    m: &alloc::collections::BTreeMap<Key, Value, A>,
    k: &Key,
) -> (result: bool)
    ensures
        result == btreemap_view_spec(*m).contains_key(*k),
;

// --- get (Q = K) ---
pub assume_specification<'a, Key: Ord, Value, A: core::alloc::Allocator + Clone>[
    alloc::collections::BTreeMap::<Key, Value, A>::get::<Key>
](
    m: &'a alloc::collections::BTreeMap<Key, Value, A>,
    k: &Key,
) -> (result: Option<&'a Value>)
    ensures
        match result {
            Some(v) => btreemap_view_spec(*m).contains_key(*k)
                && *v == btreemap_view_spec(*m)[*k],
            None => !btreemap_view_spec(*m).contains_key(*k),
        },
;

// --- get_mut (Q = K) ---
// NOTE: vstd does NOT provide get_mut specs even for std targets.
// This is a new specification.
pub assume_specification<'a, Key: Ord, Value, A: core::alloc::Allocator + Clone>[
    alloc::collections::BTreeMap::<Key, Value, A>::get_mut::<Key>
](
    m: &'a mut alloc::collections::BTreeMap<Key, Value, A>,
    k: &Key,
) -> (result: Option<&'a mut Value>)
    ensures
        // Domain unchanged.
        btreemap_view_spec(*m).dom() == btreemap_view_spec(*old(m)).dom(),
        match result {
            Some(v) => {
                &&& btreemap_view_spec(*old(m)).contains_key(*k)
                &&& *v == btreemap_view_spec(*old(m))[*k]
                // All other keys unchanged.
                &&& forall |j: Key| j != *k && btreemap_view_spec(*old(m)).contains_key(j)
                        ==> btreemap_view_spec(*m)[j] == btreemap_view_spec(*old(m))[j]
            },
            None => !btreemap_view_spec(*old(m)).contains_key(*k)
                && btreemap_view_spec(*m) == btreemap_view_spec(*old(m)),
        },
;

// --- remove (Q = K) ---
pub assume_specification<Key: Ord, Value, A: core::alloc::Allocator + Clone>[
    alloc::collections::BTreeMap::<Key, Value, A>::remove::<Key>
](
    m: &mut alloc::collections::BTreeMap<Key, Value, A>,
    k: &Key,
) -> (result: Option<Value>)
    ensures
        btreemap_view_spec(*m) == btreemap_view_spec(*old(m)).remove(*k),
        match result {
            Some(v) => btreemap_view_spec(*old(m)).contains_key(*k)
                && v == btreemap_view_spec(*old(m))[*k],
            None => !btreemap_view_spec(*old(m)).contains_key(*k),
        },
;

// --- clear ---
pub assume_specification<Key, Value, A: core::alloc::Allocator + Clone>[
    alloc::collections::BTreeMap::<Key, Value, A>::clear
](
    m: &mut alloc::collections::BTreeMap<Key, Value, A>,
)
    ensures
        btreemap_view_spec(*m) == Map::<Key, Value>::empty(),
;

//==================================================================================================
// Broadcast group
//==================================================================================================

pub broadcast group group_btree_axioms {
    axiom_btree_map_view_finite_dom,
    axiom_spec_btree_map_len,
}

} // verus!
