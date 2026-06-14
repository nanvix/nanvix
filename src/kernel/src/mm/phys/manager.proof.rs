// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

// PhysMemoryManager - Proofs
//
// Abstract laws backing the caller-facing guarantees of the `PhysMemoryManager`
// shim contracts in `manager.rs` (stated over `phys_view()` / `FrameAllocView`).
// Bodies are `admit()` in this spec-design phase; the proving phase discharges them.

verus! {

/// The many-frame watermark gate subsumes the single-frame one: if servicing
/// `count >= 1` frames still respects the watermark, then so does servicing one.
///
/// Justifies that `alloc_user_frame` (count = 1) and `alloc_many_user_frames`
/// (count) share the same `check_user_watermark` policy.
pub proof fn lemma_watermark_monotone(v: FrameAllocView, count: int)
    requires
        count >= 1,
        spec_watermark_ok(v, count),
    ensures
        spec_watermark_ok(v, 1),
{
}

/// A contiguous run with a positive page stride has pairwise-distinct addresses:
/// distinct indices map to distinct frame addresses.
///
/// Backs the "no double-allocation" reading of the contiguity guarantee in
/// `alloc_many_kernel_frames`: the `count` returned frames are genuinely distinct
/// physical frames, not aliases.
pub proof fn lemma_contiguous_run_distinct(addrs: Seq<int>, base: int)
    requires
        spec_page_size() > 0,
        is_contiguous_run(addrs, base),
    ensures
        forall|i: int, j: int|
            0 <= i < addrs.len() && 0 <= j < addrs.len() && i != j ==> addrs[i] != addrs[j],
{
    let ps = spec_page_size();
    assert forall|i: int, j: int|
        0 <= i < addrs.len() && 0 <= j < addrs.len() && i != j implies addrs[i]
        != addrs[j] by {
        assert(addrs[i] == base + i * ps);
        assert(addrs[j] == base + j * ps);
        if addrs[i] == addrs[j] {
            // From the equal addresses: i * ps == j * ps.
            vstd::arithmetic::mul::lemma_mul_is_commutative(i, ps);
            vstd::arithmetic::mul::lemma_mul_is_commutative(j, ps);
            // ps * (i - j) == ps * i - ps * j == 0.
            vstd::arithmetic::mul::lemma_mul_is_distributive_sub(ps, i, j);
            assert(ps * (i - j) == 0);
            // A nonzero stride times a nonzero difference cannot be zero.
            vstd::arithmetic::mul::lemma_mul_nonzero(ps, i - j);
            assert(false);
        }
    }
}

} // verus!
