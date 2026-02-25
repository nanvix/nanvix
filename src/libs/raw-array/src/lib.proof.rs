// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

// RawArray - Proofs
//
// This file contains lemmas and proof functions for RawArray.

verus! {

//==================================================================================================
// Lemmas about RawArrayView
//==================================================================================================

/// Lemma: Updating an array preserves its length.
pub proof fn lemma_update_preserves_len<T>(view: RawArrayView<T>, i: int, value: T)
    requires
        0 <= i < view.len() as int,
    ensures
        view.update(i, value).len() == view.len(),
{
}

/// Lemma: Updating index i only changes index i.
pub proof fn lemma_update_only_changes_index<T>(view: RawArrayView<T>, i: int, value: T, j: int)
    requires
        0 <= i < view.len() as int,
        0 <= j < view.len() as int,
        i != j,
    ensures
        view.update(i, value).index(j) == view.index(j),
{
}

/// Lemma: Updating index i sets index i to the new value.
pub proof fn lemma_update_sets_index<T>(view: RawArrayView<T>, i: int, value: T)
    requires
        0 <= i < view.len() as int,
    ensures
        view.update(i, value).index(i) == value,
{
}

//==================================================================================================
// Lemmas about View Equality
//==================================================================================================

/// Lemma: View equality is reflexive.
pub proof fn lemma_view_eq_reflexive<T>(view: &RawArrayView<T>)
    ensures
        view.spec_eq(view),
{
}

/// Lemma: View equality is symmetric.
pub proof fn lemma_view_eq_symmetric<T>(v1: &RawArrayView<T>, v2: &RawArrayView<T>)
    requires
        v1.spec_eq(v2),
    ensures
        v2.spec_eq(v1),
{
}

/// Lemma: View equality is transitive.
pub proof fn lemma_view_eq_transitive<T>(
    v1: &RawArrayView<T>,
    v2: &RawArrayView<T>,
    v3: &RawArrayView<T>,
)
    requires
        v1.spec_eq(v2),
        v2.spec_eq(v3),
    ensures
        v1.spec_eq(v3),
{
}

/// Lemma: Equal views have equal elements.
pub proof fn lemma_equal_views_equal_elements<T>(v1: &RawArrayView<T>, v2: &RawArrayView<T>, i: int)
    requires
        v1.spec_eq(v2),
        0 <= i < v1.len() as int,
    ensures
        v1.index(i) == v2.index(i),
{
}

/// Lemma: Equal views have equal lengths.
pub proof fn lemma_equal_views_equal_len<T>(v1: &RawArrayView<T>, v2: &RawArrayView<T>)
    requires
        v1.spec_eq(v2),
    ensures
        v1.len() == v2.len(),
{
}

//==================================================================================================
// Lemmas for Seq Operations
//==================================================================================================

/// Lemma: Consecutive sets to different indices commute.
pub proof fn lemma_set_commutes<T>(view: Seq<T>, i: int, vi: T, j: int, vj: T)
    requires
        0 <= i < view.len(),
        0 <= j < view.len(),
        i != j,
    ensures
        view.update(i, vi).update(j, vj) =~= view.update(j, vj).update(i, vi),
{
}

/// Lemma: Setting the same index twice results in the last value.
pub proof fn lemma_set_overwrite<T>(view: Seq<T>, i: int, v1: T, v2: T)
    requires
        0 <= i < view.len(),
    ensures
        view.update(i, v1).update(i, v2) =~= view.update(i, v2),
{
}

/// Lemma: Setting an element preserves all other elements (frame condition).
pub proof fn lemma_set_frame<T>(view: Seq<T>, i: int, value: T)
    requires
        0 <= i < view.len(),
    ensures
        view.update(i, value).len() == view.len(),
        forall|j: int| 0 <= j < view.len() && j != i
            ==> #[trigger] view.update(i, value)[j] == view[j],
{
}

} // verus!
