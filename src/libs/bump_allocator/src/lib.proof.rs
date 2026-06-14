// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

// FixedSizeBumpAllocator - Proofs
//
// Proof functions for the abstract pool model. Each lemma is fully discharged
// (no `admit()`/`assume()`) and encodes a caller expectation from
// `verus-ai-logs/nanvix-phys-bump-allocator/caller_analysis.md`.

verus! {

//==================================================================================================
// align_up - ceiling-division lemma
//==================================================================================================

/// Open-coded ceiling division matches the spec-level `(v + d - 1) / d`.
///
/// Backs the body of `align_up`, which computes `value.div_ceil(alignment)` as
/// `value / alignment` adjusted by one when the remainder is non-zero. Verus has
/// no specification for the `usize::div_ceil` intrinsic, so the equivalence is
/// proven here explicitly.
pub proof fn lemma_ceil_div(v: int, d: int, qd: int, r: int)
    requires
        d > 0,
        v >= 0,
        qd == v / d,
        r == v % d,
    ensures
        (if r == 0 {
            qd
        } else {
            qd + 1
        }) == (v + d - 1) / d,
{
    vstd::arithmetic::div_mod::lemma_fundamental_div_mod(v, d);
    assert(0 <= r < d);
    assert(v == d * qd + r);
    if r == 0 {
        assert((v + d - 1) / d == qd) by (nonlinear_arith)
            requires
                d > 0,
                v == d * qd,
                0 <= 0 < d,
        ;
    } else {
        assert((v + d - 1) / d == qd + 1) by (nonlinear_arith)
            requires
                d > 0,
                v == d * qd + r,
                0 < r < d,
        ;
    }
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
    let a: int = v.unit_align as int;
    let s: int = v.stride as int;
    let b: int = v.base as int;
    let us: int = v.unit_size as int;
    let cap: int = v.capacity as int;
    let ss: int = v.storage_size as int;

    // `stride` is an exact multiple of `unit_align`: s == a * (s / a).
    let q: int = s / a;
    vstd::arithmetic::div_mod::lemma_fundamental_div_mod(s, a);
    assert(s == a * q);

    // (a) Every slot start is `unit_align`-aligned.
    assert forall|i: int| 0 <= i < cap implies #[trigger] v.slot_addr(i) % a == 0 by {
        // i * s == a * (i * q), so slot_addr(i) == a * (i * q) + b.
        assert(i * s == a * (i * q)) by (nonlinear_arith)
            requires
                s == a * q,
        ;
        vstd::arithmetic::div_mod::lemma_mod_multiples_vanish(i * q, b, a);
    }

    // (b) Every slot lies fully inside the backing region.
    assert forall|i: int| 0 <= i < cap implies {
        &&& v.base <= #[trigger] v.slot_addr(i)
        &&& v.slot_addr(i) + us <= v.base + ss
    } by {
        // Lower bound: i * s >= 0.
        vstd::arithmetic::mul::lemma_mul_nonnegative(i, s);
        // Upper bound: i * s + us <= (i + 1) * s <= cap * s <= ss.
        assert((i + 1) * s == i * s + s) by {
            vstd::arithmetic::mul::lemma_mul_is_distributive_add_other_way(s, i, 1);
        }
        vstd::arithmetic::mul::lemma_mul_inequality(i + 1, cap, s);
    }

    // (c) Distinct indices map to distinct addresses.
    assert forall|i: int, j: int|
        (0 <= i < cap && 0 <= j < cap && i != j) implies (#[trigger] v.slot_addr(i)) != (
    #[trigger] v.slot_addr(j)) by {
        if i < j {
            vstd::arithmetic::mul::lemma_mul_strict_inequality(i, j, s);
        } else {
            vstd::arithmetic::mul::lemma_mul_strict_inequality(j, i, s);
        }
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
