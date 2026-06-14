// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

// FixedSizeBumpAllocator - Proofs
//
// Proof functions for the abstract pool model. Each lemma encodes a caller
// expectation from `verus-ai-logs/nanvix-phys-bump-allocator/caller_analysis.md`.

verus! {

//==================================================================================================
// Arithmetic helper
//==================================================================================================

/// If `base` and `stride` are both multiples of `m > 0`, then so is
/// `base + i * stride` for any `i`. Backs the per-slot alignment guarantee
/// (`slot_addr(i) = base + i * stride`).
proof fn lemma_aligned_sum(base: int, stride: int, i: int, m: int)
    requires
        m > 0,
        base % m == 0,
        stride % m == 0,
    ensures
        (base + i * stride) % m == 0,
{
    vstd::arithmetic::div_mod::lemma_fundamental_div_mod(base, m);
    vstd::arithmetic::div_mod::lemma_fundamental_div_mod(stride, m);
    let b = base / m;
    let s = stride / m;
    assert(base == m * b);
    assert(stride == m * s);
    assert(base + i * stride == (b + i * s) * m) by (nonlinear_arith)
        requires
            base == m * b,
            stride == m * s,
    ;
    vstd::arithmetic::div_mod::lemma_mod_multiples_basic(b + i * s, m);
}

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
    let m = v.unit_align as int;
    let stride = v.stride as int;
    let base = v.base;
    let cap = v.capacity as int;
    let usz = v.unit_size as int;

    assert forall|i: int| 0 <= i < cap implies #[trigger] v.slot_addr(i) % m == 0 by {
        lemma_aligned_sum(base, stride, i, m);
    }

    assert forall|i: int| 0 <= i < cap implies {
        &&& v.base <= #[trigger] v.slot_addr(i)
        &&& v.slot_addr(i) + (v.unit_size as int) <= v.base + (v.storage_size as int)
    } by {
        assert(i * stride >= 0) by (nonlinear_arith)
            requires
                i >= 0,
                stride >= 0,
        ;
        assert(i * stride <= (cap - 1) * stride) by (nonlinear_arith)
            requires
                i <= cap - 1,
                stride >= 0,
        ;
        assert((cap - 1) * stride + stride == cap * stride) by (nonlinear_arith);
    }

    assert forall|i: int, j: int| (0 <= i < cap && 0 <= j < cap && i != j) implies (
    #[trigger] v.slot_addr(i)) != (#[trigger] v.slot_addr(j)) by {
        assert(v.slot_addr(i) - v.slot_addr(j) == (i - j) * stride) by (nonlinear_arith);
        assert((i - j) * stride != 0) by (nonlinear_arith)
            requires
                i != j,
                stride > 0,
        ;
    }
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
    assert(v.spec_alloc() =~= (BumpView { allocated: v.spec_alloc().allocated, ..v }));
}

} // verus!
