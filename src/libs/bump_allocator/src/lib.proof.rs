// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

// FixedSizeBumpAllocator - Proofs
//
// Proof-function signatures for the abstract pool model. Bodies are `admit()`
// placeholders during the specification phase; the proving phase discharges them.
// Each lemma encodes a caller expectation from
// `verus-ai-logs/nanvix-phys-bump-allocator/caller_analysis.md`.

verus! {

//==================================================================================================
// Geometry lemmas (uniqueness / in-bounds / alignment)
//==================================================================================================

/// A well-formed pool satisfies all three geometric guarantees: every slot is
/// `unit_align`-aligned, lies fully inside the backing region, and is distinct
/// from every other slot.
///
/// Backs caller invariants: *Alignment*, *In-bounds*, *Uniqueness / non-aliasing*.
pub proof fn lemma_geometry(v: BumpView)
    requires
        v.inv(),
    ensures
        v.geometry_ok(),
{
    admit();
}

//==================================================================================================
// Capacity / transition lemmas (monotone capacity, no spurious consumption)
//==================================================================================================

/// The exhaustion boundary is exactly `allocated == capacity`: the pool has a free
/// slot iff fewer than `capacity` slots have been handed out.
///
/// Backs caller invariant: *Monotone capacity* (the `Exhausted` boundary).
pub proof fn lemma_exhausted_boundary(v: BumpView)
    requires
        v.inv(),
    ensures
        !v.has_capacity() <==> v.allocated == v.capacity,
{
    admit();
}

/// A successful allocation advances the cursor by exactly one slot and preserves
/// the invariant. The freshly handed-out index `v.allocated` was previously free
/// and becomes consumed.
///
/// Backs caller invariants: *Monotone capacity* (advance by one), *Uniqueness*
/// (each index handed out at most once).
pub proof fn lemma_alloc_transition(v: BumpView)
    requires
        v.inv(),
        v.has_capacity(),
    ensures
        v.spec_alloc().inv(),
        v.spec_alloc().allocated == v.allocated + 1,
        !v.is_consumed(v.allocated as int),
        v.spec_alloc().is_consumed(v.allocated as int),
        // Configuration (everything except `allocated`) is unchanged.
        v.spec_alloc() == (BumpView { allocated: v.spec_alloc().allocated, ..v }),
{
    admit();
}

} // verus!
