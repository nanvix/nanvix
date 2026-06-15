// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

use ::vstd::arithmetic::power2::is_pow2;

// ==================================================================================================
// Kheap — Proofs
//
// View implementation, concrete invariant linking kheap to its abstract view,
// and lemmas used by API proofs.
// ==================================================================================================

verus! {

// --------------------------------------------------------------------------------------------------
// View impl for Kheap
// --------------------------------------------------------------------------------------------------

impl View for Kheap {
    type V = KheapView;

    closed spec fn view(&self) -> KheapView {
        KheapView {
            allocations: self.alloc_map@,
        }
    }
}

/// Maps a SlabSize tier to its block size in bytes.
closed spec fn spec_tier_size(tier: SlabSize) -> usize {
    match tier {
        SlabSize::Slab8 => 8,
        SlabSize::Slab16 => 16,
        SlabSize::Slab32 => 32,
        SlabSize::Slab64 => 64,
        SlabSize::Slab128 => 128,
        SlabSize::Slab256 => 256,
        SlabSize::Slab512 => 512,
    }
}

impl Kheap {
    /// Slab-level invariants kept internal to the module. These bridge the
    /// concrete slab state to the abstract `KheapView`.
    pub closed spec fn internal_inv(&self) -> bool {
        // Each slab tier is well-formed with the expected block size.
        &&& self.slab_8_bytes.inv()
        &&& self.slab_16_bytes.inv()
        &&& self.slab_32_bytes.inv()
        &&& self.slab_64_bytes.inv()
        &&& self.slab_128_bytes.inv()
        &&& self.slab_256_bytes.inv()
        &&& self.slab_512_bytes.inv()

        &&& self.slab_8_bytes@.block_size == 8
        &&& self.slab_16_bytes@.block_size == 16
        &&& self.slab_32_bytes@.block_size == 32
        &&& self.slab_64_bytes@.block_size == 64
        &&& self.slab_128_bytes@.block_size == 128
        &&& self.slab_256_bytes@.block_size == 256
        &&& self.slab_512_bytes@.block_size == 512

        // Slab data regions are laid out in strict address order (and thus
        // disjoint).
        &&& self.slab_8_bytes@.end_addr <= self.slab_16_bytes@.start_addr
        &&& self.slab_16_bytes@.end_addr <= self.slab_32_bytes@.start_addr
        &&& self.slab_32_bytes@.end_addr <= self.slab_64_bytes@.start_addr
        &&& self.slab_64_bytes@.end_addr <= self.slab_128_bytes@.start_addr
        &&& self.slab_128_bytes@.end_addr <= self.slab_256_bytes@.start_addr
        &&& self.slab_256_bytes@.end_addr <= self.slab_512_bytes@.start_addr

        // Every slab starts at a strictly positive address (so returned
        // pointers are guaranteed non-null).
        &&& self.slab_8_bytes@.start_addr > 0

        // Abstract allocation map ⇒ live slab block, routed by size tier.
        &&& forall|addr: int| #[trigger] self.alloc_map@.dom().contains(addr) ==> {
                let size = self.alloc_map@[addr];
                let a = addr as usize;
                &&& 0 < addr <= usize::MAX
                &&& ({
                        ||| (1 <= size <= 8 && self.slab_8_bytes@.allocated_addrs.contains(a))
                        ||| (9 <= size <= 16 && self.slab_16_bytes@.allocated_addrs.contains(a))
                        ||| (17 <= size <= 32 && self.slab_32_bytes@.allocated_addrs.contains(a))
                        ||| (33 <= size <= 64 && self.slab_64_bytes@.allocated_addrs.contains(a))
                        ||| (65 <= size <= 128 && self.slab_128_bytes@.allocated_addrs.contains(a))
                        ||| (129 <= size <= 256 && self.slab_256_bytes@.allocated_addrs.contains(a))
                        ||| (257 <= size <= 512 && self.slab_512_bytes@.allocated_addrs.contains(a))
                    })
            }

        // Live slab block ⇒ abstract allocation map (reverse direction).
        // The stored layout_size must fall in the tier's band.
        &&& forall|a: usize| #[trigger] self.slab_8_bytes@.allocated_addrs.contains(a)
                ==> self.alloc_map@.dom().contains(a as int)
                    && 1 <= self.alloc_map@[a as int] <= 8
        &&& forall|a: usize| #[trigger] self.slab_16_bytes@.allocated_addrs.contains(a)
                ==> self.alloc_map@.dom().contains(a as int)
                    && 9 <= self.alloc_map@[a as int] <= 16
        &&& forall|a: usize| #[trigger] self.slab_32_bytes@.allocated_addrs.contains(a)
                ==> self.alloc_map@.dom().contains(a as int)
                    && 17 <= self.alloc_map@[a as int] <= 32
        &&& forall|a: usize| #[trigger] self.slab_64_bytes@.allocated_addrs.contains(a)
                ==> self.alloc_map@.dom().contains(a as int)
                    && 33 <= self.alloc_map@[a as int] <= 64
        &&& forall|a: usize| #[trigger] self.slab_128_bytes@.allocated_addrs.contains(a)
                ==> self.alloc_map@.dom().contains(a as int)
                    && 65 <= self.alloc_map@[a as int] <= 128
        &&& forall|a: usize| #[trigger] self.slab_256_bytes@.allocated_addrs.contains(a)
                ==> self.alloc_map@.dom().contains(a as int)
                    && 129 <= self.alloc_map@[a as int] <= 256
        &&& forall|a: usize| #[trigger] self.slab_512_bytes@.allocated_addrs.contains(a)
                ==> self.alloc_map@.dom().contains(a as int)
                    && 257 <= self.alloc_map@[a as int] <= 512
    }
}

// --------------------------------------------------------------------------------------------------
// Lemmas (added on demand as API proofs require them).
// --------------------------------------------------------------------------------------------------

impl Kheap {
/// Arithmetic helper for `from_raw_parts`: if `size == NUM_OF_SLABS * slab_size`
/// and `addr + size <= usize::MAX`, then for every tier index `k` in
/// `[0, NUM_OF_SLABS]` we have `addr + k * slab_size <= usize::MAX`.
proof fn lemma_tier_offset_bound(addr: usize, size: usize, slab_size: usize, k: usize)
    requires
        size == NUM_OF_SLABS * slab_size,
        addr + size <= usize::MAX,
        k <= NUM_OF_SLABS,
    ensures
        addr + k * slab_size <= usize::MAX,
        k * slab_size <= size,
{
    assert(k * slab_size <= NUM_OF_SLABS * slab_size) by (nonlinear_arith)
        requires k <= NUM_OF_SLABS;
}

/// If `a` and `end` are both multiples of `block > 0` and `a < end`, then
/// `a + block <= end`.
proof fn lemma_block_fits_below_end(a: int, end: int, block: int)
    requires
        block > 0,
        a % block == 0,
        end % block == 0,
        a < end,
    ensures
        a + block <= end,
{
    vstd::arithmetic::div_mod::lemma_sub_mod_noop(end, a, block);
    assert((0 as int) % block == 0);
    assert((end - a) % block == 0);
    assert(end - a >= block) by (nonlinear_arith)
        requires (end - a) % block == 0, end - a > 0, block > 0;
}

/// Two distinct block-aligned addresses in the same slab differ by at
/// least one block.
proof fn lemma_same_slab_no_overlap(a1: int, s1: int, a2: int, s2: int, block: int)
    requires
        block > 0,
        a1 % block == 0,
        a2 % block == 0,
        a1 != a2,
        0 < s1 <= block,
        0 < s2 <= block,
    ensures
        a1 + s1 <= a2 || a2 + s2 <= a1,
{
    if a1 < a2 {
        vstd::arithmetic::div_mod::lemma_sub_mod_noop(a2, a1, block);
        assert((0 as int) % block == 0);
        assert((a2 - a1) % block == 0);
        assert(a2 - a1 >= block) by (nonlinear_arith)
            requires (a2 - a1) % block == 0, a2 - a1 > 0, block > 0;
    } else {
        assert(a2 < a1);
        vstd::arithmetic::div_mod::lemma_sub_mod_noop(a1, a2, block);
        assert((0 as int) % block == 0);
        assert((a1 - a2) % block == 0);
        assert(a1 - a2 >= block) by (nonlinear_arith)
            requires (a1 - a2) % block == 0, a1 - a2 > 0, block > 0;
    }
}

/// Core overlap-freedom lemma: a newly-served allocation (ptr, size) at
/// tier `N` does not overlap with any prior allocation (a2, s2).
///
/// Callers supply the pre-state `Kheap`, the chosen tier, the new `ptr`
/// known to be allocated in the chosen slab after insertion, and any
/// `a2 != ptr` already present in pre. The proof enumerates the a2's
/// tier and compares against the chosen tier.
proof fn lemma_no_overlap_with_new_ptr(
    pre: &Kheap,
    tier: SlabSize,
    ptr: usize,
    size: usize,
    a2: int,
)
    requires
        pre.inv(),
        // Size routes to `tier`.
        (tier == SlabSize::Slab8 && 1 <= size <= 8)
            || (tier == SlabSize::Slab16 && 9 <= size <= 16)
            || (tier == SlabSize::Slab32 && 17 <= size <= 32)
            || (tier == SlabSize::Slab64 && 33 <= size <= 64)
            || (tier == SlabSize::Slab128 && 65 <= size <= 128)
            || (tier == SlabSize::Slab256 && 129 <= size <= 256)
            || (tier == SlabSize::Slab512 && 257 <= size <= 512),
        // ptr is a block in the chosen tier's slab.
        (tier == SlabSize::Slab8 ==> pre.slab_8_bytes@.free_addrs.contains(ptr))
            && (tier == SlabSize::Slab16 ==> pre.slab_16_bytes@.free_addrs.contains(ptr))
            && (tier == SlabSize::Slab32 ==> pre.slab_32_bytes@.free_addrs.contains(ptr))
            && (tier == SlabSize::Slab64 ==> pre.slab_64_bytes@.free_addrs.contains(ptr))
            && (tier == SlabSize::Slab128 ==> pre.slab_128_bytes@.free_addrs.contains(ptr))
            && (tier == SlabSize::Slab256 ==> pre.slab_256_bytes@.free_addrs.contains(ptr))
            && (tier == SlabSize::Slab512 ==> pre.slab_512_bytes@.free_addrs.contains(ptr)),
        pre.alloc_map@.dom().contains(a2),
        !pre.alloc_map@.dom().contains(ptr as int),
    ensures
        ptr as int + size as int <= a2
            || a2 + pre.alloc_map@[a2] as int <= ptr as int,
{
    let s2 = pre.alloc_map@[a2];
    let a2u = a2 as usize;
    // Tier-band and slab membership for a2, from pre.internal_inv forward.
    assert(pre.internal_inv());
    assert(
        (1 <= s2 <= 8 && pre.slab_8_bytes@.allocated_addrs.contains(a2u))
        || (9 <= s2 <= 16 && pre.slab_16_bytes@.allocated_addrs.contains(a2u))
        || (17 <= s2 <= 32 && pre.slab_32_bytes@.allocated_addrs.contains(a2u))
        || (33 <= s2 <= 64 && pre.slab_64_bytes@.allocated_addrs.contains(a2u))
        || (65 <= s2 <= 128 && pre.slab_128_bytes@.allocated_addrs.contains(a2u))
        || (129 <= s2 <= 256 && pre.slab_256_bytes@.allocated_addrs.contains(a2u))
        || (257 <= s2 <= 512 && pre.slab_512_bytes@.allocated_addrs.contains(a2u))
    );
    // a2 != ptr (since ptr wasn't in pre).
    assert(a2 != ptr as int);
    // Dispatch tier × slab_of_a2 by walking the seven a2-slab options and
    // invoking the right geometric helper. Using a macro-like flat chain.
    if 1 <= s2 <= 8 {
        Self::lemma_no_overlap_case(
            pre.slab_8_bytes@.start_addr, pre.slab_8_bytes@.end_addr, 8,
            a2u, s2, pre.slab_8_bytes@.allocated_addrs,
            pre, tier, ptr, size,
        );
    } else if 9 <= s2 <= 16 {
        Self::lemma_no_overlap_case(
            pre.slab_16_bytes@.start_addr, pre.slab_16_bytes@.end_addr, 16,
            a2u, s2, pre.slab_16_bytes@.allocated_addrs,
            pre, tier, ptr, size,
        );
    } else if 17 <= s2 <= 32 {
        Self::lemma_no_overlap_case(
            pre.slab_32_bytes@.start_addr, pre.slab_32_bytes@.end_addr, 32,
            a2u, s2, pre.slab_32_bytes@.allocated_addrs,
            pre, tier, ptr, size,
        );
    } else if 33 <= s2 <= 64 {
        Self::lemma_no_overlap_case(
            pre.slab_64_bytes@.start_addr, pre.slab_64_bytes@.end_addr, 64,
            a2u, s2, pre.slab_64_bytes@.allocated_addrs,
            pre, tier, ptr, size,
        );
    } else if 65 <= s2 <= 128 {
        Self::lemma_no_overlap_case(
            pre.slab_128_bytes@.start_addr, pre.slab_128_bytes@.end_addr, 128,
            a2u, s2, pre.slab_128_bytes@.allocated_addrs,
            pre, tier, ptr, size,
        );
    } else if 129 <= s2 <= 256 {
        Self::lemma_no_overlap_case(
            pre.slab_256_bytes@.start_addr, pre.slab_256_bytes@.end_addr, 256,
            a2u, s2, pre.slab_256_bytes@.allocated_addrs,
            pre, tier, ptr, size,
        );
    } else {
        assert(257 <= s2 <= 512);
        Self::lemma_no_overlap_case(
            pre.slab_512_bytes@.start_addr, pre.slab_512_bytes@.end_addr, 512,
            a2u, s2, pre.slab_512_bytes@.allocated_addrs,
            pre, tier, ptr, size,
        );
    }
}

/// Given that a2 is a block-aligned allocation in the slab with block
/// size `block_m` at `[start_m, end_m)`, and ptr is in the chosen tier's
/// slab (selected by `tier`), show they don't overlap with (ptr, size)
/// and (a2, s2) respectively. `block_m` is known statically per caller.
proof fn lemma_no_overlap_case(
    start_m: usize,
    end_m: usize,
    block_m: usize,
    a2u: usize,
    s2: nat,
    alloc_m: Set<usize>,
    pre: &Kheap,
    tier: SlabSize,
    ptr: usize,
    size: usize,
)
    requires
        pre.inv(),
        block_m == 8 || block_m == 16 || block_m == 32 || block_m == 64
            || block_m == 128 || block_m == 256 || block_m == 512,
        // a2's slab
        (block_m == 8 && start_m == pre.slab_8_bytes@.start_addr
            && end_m == pre.slab_8_bytes@.end_addr
            && alloc_m == pre.slab_8_bytes@.allocated_addrs)
            || (block_m == 16 && start_m == pre.slab_16_bytes@.start_addr
                && end_m == pre.slab_16_bytes@.end_addr
                && alloc_m == pre.slab_16_bytes@.allocated_addrs)
            || (block_m == 32 && start_m == pre.slab_32_bytes@.start_addr
                && end_m == pre.slab_32_bytes@.end_addr
                && alloc_m == pre.slab_32_bytes@.allocated_addrs)
            || (block_m == 64 && start_m == pre.slab_64_bytes@.start_addr
                && end_m == pre.slab_64_bytes@.end_addr
                && alloc_m == pre.slab_64_bytes@.allocated_addrs)
            || (block_m == 128 && start_m == pre.slab_128_bytes@.start_addr
                && end_m == pre.slab_128_bytes@.end_addr
                && alloc_m == pre.slab_128_bytes@.allocated_addrs)
            || (block_m == 256 && start_m == pre.slab_256_bytes@.start_addr
                && end_m == pre.slab_256_bytes@.end_addr
                && alloc_m == pre.slab_256_bytes@.allocated_addrs)
            || (block_m == 512 && start_m == pre.slab_512_bytes@.start_addr
                && end_m == pre.slab_512_bytes@.end_addr
                && alloc_m == pre.slab_512_bytes@.allocated_addrs),
        alloc_m.contains(a2u),
        0 < s2 <= block_m,
        // Size routes to `tier`.
        (tier == SlabSize::Slab8 && 1 <= size <= 8)
            || (tier == SlabSize::Slab16 && 9 <= size <= 16)
            || (tier == SlabSize::Slab32 && 17 <= size <= 32)
            || (tier == SlabSize::Slab64 && 33 <= size <= 64)
            || (tier == SlabSize::Slab128 && 65 <= size <= 128)
            || (tier == SlabSize::Slab256 && 129 <= size <= 256)
            || (tier == SlabSize::Slab512 && 257 <= size <= 512),
        // ptr is a block in the chosen tier's slab.
        (tier == SlabSize::Slab8 ==> pre.slab_8_bytes@.free_addrs.contains(ptr))
            && (tier == SlabSize::Slab16 ==> pre.slab_16_bytes@.free_addrs.contains(ptr))
            && (tier == SlabSize::Slab32 ==> pre.slab_32_bytes@.free_addrs.contains(ptr))
            && (tier == SlabSize::Slab64 ==> pre.slab_64_bytes@.free_addrs.contains(ptr))
            && (tier == SlabSize::Slab128 ==> pre.slab_128_bytes@.free_addrs.contains(ptr))
            && (tier == SlabSize::Slab256 ==> pre.slab_256_bytes@.free_addrs.contains(ptr))
            && (tier == SlabSize::Slab512 ==> pre.slab_512_bytes@.free_addrs.contains(ptr)),
        a2u != ptr,
    ensures
        ptr as int + size as int <= a2u as int
            || a2u as int + s2 as int <= ptr as int,
{
    // Determine ptr's slab (start_n, end_n, block_n) from tier. 7 cases.
    let (start_n, end_n, block_n): (usize, usize, usize) = if tier == SlabSize::Slab8 {
        (pre.slab_8_bytes@.start_addr, pre.slab_8_bytes@.end_addr, 8)
    } else if tier == SlabSize::Slab16 {
        (pre.slab_16_bytes@.start_addr, pre.slab_16_bytes@.end_addr, 16)
    } else if tier == SlabSize::Slab32 {
        (pre.slab_32_bytes@.start_addr, pre.slab_32_bytes@.end_addr, 32)
    } else if tier == SlabSize::Slab64 {
        (pre.slab_64_bytes@.start_addr, pre.slab_64_bytes@.end_addr, 64)
    } else if tier == SlabSize::Slab128 {
        (pre.slab_128_bytes@.start_addr, pre.slab_128_bytes@.end_addr, 128)
    } else if tier == SlabSize::Slab256 {
        (pre.slab_256_bytes@.start_addr, pre.slab_256_bytes@.end_addr, 256)
    } else {
        assert(tier == SlabSize::Slab512);
        (pre.slab_512_bytes@.start_addr, pre.slab_512_bytes@.end_addr, 512)
    };
    assert(size as int <= block_n as int);
    // ptr is block-aligned and in [start_n, end_n).
    assert(start_n <= ptr < end_n);
    assert(ptr as int % block_n as int == 0);
    assert(start_n as int % block_n as int == 0);
    assert(end_n as int % block_n as int == 0);
    // a2 is block-aligned and in [start_m, end_m).
    assert(start_m <= a2u < end_m);
    assert(a2u as int % block_m as int == 0);

    if block_m == block_n && start_m == start_n {
        // Same slab: block-aligned distinct addresses, sizes ≤ block.
        assert(block_m == block_n);
        Self::lemma_same_slab_no_overlap(
            a2u as int, s2 as int, ptr as int, size as int, block_m as int,
        );
    } else {
        // Different slabs: use internal_inv's end/start ordering to get
        // disjoint ranges. Case-split on which tier comes first.
        assert(
            pre.slab_8_bytes@.end_addr <= pre.slab_16_bytes@.start_addr
            && pre.slab_16_bytes@.end_addr <= pre.slab_32_bytes@.start_addr
            && pre.slab_32_bytes@.end_addr <= pre.slab_64_bytes@.start_addr
            && pre.slab_64_bytes@.end_addr <= pre.slab_128_bytes@.start_addr
            && pre.slab_128_bytes@.end_addr <= pre.slab_256_bytes@.start_addr
            && pre.slab_256_bytes@.end_addr <= pre.slab_512_bytes@.start_addr
        );
        // Whichever ends earlier: that allocation's end ≤ the other's start.
        if end_m <= start_n {
            // a2 + s2 ≤ a2 + block_m ≤ end_m ≤ start_n ≤ ptr.
            Self::lemma_block_fits_below_end(a2u as int, end_m as int, block_m as int);
            assert(a2u as int + s2 as int <= ptr as int);
        } else {
            assert(end_n <= start_m);
            Self::lemma_block_fits_below_end(ptr as int, end_n as int, block_n as int);
            assert(ptr as int + size as int <= a2u as int);
        }
    }
}

/// Closes the post-allocate inv-preservation goal: given that `tier` was used
/// to serve a size-`size` request returning `ptr`, and the ghost map was
/// updated accordingly, show `self.inv()`.
///
/// Assumptions (all discharged by slab's spec + the match branch context):
///   * pre.inv() held,
///   * the chosen slab's new state differs from the old by inserting `ptr`
///     into its allocated_addrs,
///   * all OTHER slab tiers are frame-unchanged,
///   * alloc_map' == alloc_map.insert(ptr, size).
proof fn lemma_alloc_preserves_internal_inv(
    pre: &Kheap,
    post: &Kheap,
    tier: SlabSize,
    ptr: usize,
    size: usize,
)
    requires
        pre.inv(),
        // allocate's success contract translated to Kheap fields.
        post.slab_8_bytes.inv(),
        post.slab_16_bytes.inv(),
        post.slab_32_bytes.inv(),
        post.slab_64_bytes.inv(),
        post.slab_128_bytes.inv(),
        post.slab_256_bytes.inv(),
        post.slab_512_bytes.inv(),
        post.slab_8_bytes@.block_size == 8,
        post.slab_16_bytes@.block_size == 16,
        post.slab_32_bytes@.block_size == 32,
        post.slab_64_bytes@.block_size == 64,
        post.slab_128_bytes@.block_size == 128,
        post.slab_256_bytes@.block_size == 256,
        post.slab_512_bytes@.block_size == 512,
        post.slab_8_bytes@.start_addr == pre.slab_8_bytes@.start_addr,
        post.slab_8_bytes@.end_addr == pre.slab_8_bytes@.end_addr,
        post.slab_16_bytes@.start_addr == pre.slab_16_bytes@.start_addr,
        post.slab_16_bytes@.end_addr == pre.slab_16_bytes@.end_addr,
        post.slab_32_bytes@.start_addr == pre.slab_32_bytes@.start_addr,
        post.slab_32_bytes@.end_addr == pre.slab_32_bytes@.end_addr,
        post.slab_64_bytes@.start_addr == pre.slab_64_bytes@.start_addr,
        post.slab_64_bytes@.end_addr == pre.slab_64_bytes@.end_addr,
        post.slab_128_bytes@.start_addr == pre.slab_128_bytes@.start_addr,
        post.slab_128_bytes@.end_addr == pre.slab_128_bytes@.end_addr,
        post.slab_256_bytes@.start_addr == pre.slab_256_bytes@.start_addr,
        post.slab_256_bytes@.end_addr == pre.slab_256_bytes@.end_addr,
        post.slab_512_bytes@.start_addr == pre.slab_512_bytes@.start_addr,
        post.slab_512_bytes@.end_addr == pre.slab_512_bytes@.end_addr,
        // Size routes to `tier`.
        (tier == SlabSize::Slab8 && 1 <= size <= 8)
            || (tier == SlabSize::Slab16 && 9 <= size <= 16)
            || (tier == SlabSize::Slab32 && 17 <= size <= 32)
            || (tier == SlabSize::Slab64 && 33 <= size <= 64)
            || (tier == SlabSize::Slab128 && 65 <= size <= 128)
            || (tier == SlabSize::Slab256 && 129 <= size <= 256)
            || (tier == SlabSize::Slab512 && 257 <= size <= 512),
        // Chosen slab gained `ptr`, others frame.
        post.slab_8_bytes@.allocated_addrs == if tier == SlabSize::Slab8 {
            pre.slab_8_bytes@.allocated_addrs.insert(ptr)
        } else { pre.slab_8_bytes@.allocated_addrs },
        post.slab_16_bytes@.allocated_addrs == if tier == SlabSize::Slab16 {
            pre.slab_16_bytes@.allocated_addrs.insert(ptr)
        } else { pre.slab_16_bytes@.allocated_addrs },
        post.slab_32_bytes@.allocated_addrs == if tier == SlabSize::Slab32 {
            pre.slab_32_bytes@.allocated_addrs.insert(ptr)
        } else { pre.slab_32_bytes@.allocated_addrs },
        post.slab_64_bytes@.allocated_addrs == if tier == SlabSize::Slab64 {
            pre.slab_64_bytes@.allocated_addrs.insert(ptr)
        } else { pre.slab_64_bytes@.allocated_addrs },
        post.slab_128_bytes@.allocated_addrs == if tier == SlabSize::Slab128 {
            pre.slab_128_bytes@.allocated_addrs.insert(ptr)
        } else { pre.slab_128_bytes@.allocated_addrs },
        post.slab_256_bytes@.allocated_addrs == if tier == SlabSize::Slab256 {
            pre.slab_256_bytes@.allocated_addrs.insert(ptr)
        } else { pre.slab_256_bytes@.allocated_addrs },
        post.slab_512_bytes@.allocated_addrs == if tier == SlabSize::Slab512 {
            pre.slab_512_bytes@.allocated_addrs.insert(ptr)
        } else { pre.slab_512_bytes@.allocated_addrs },
        // Ptr came from the chosen slab's free list (from slab.allocate Ok).
        (tier == SlabSize::Slab8 ==> pre.slab_8_bytes@.free_addrs.contains(ptr))
            && (tier == SlabSize::Slab16 ==> pre.slab_16_bytes@.free_addrs.contains(ptr))
            && (tier == SlabSize::Slab32 ==> pre.slab_32_bytes@.free_addrs.contains(ptr))
            && (tier == SlabSize::Slab64 ==> pre.slab_64_bytes@.free_addrs.contains(ptr))
            && (tier == SlabSize::Slab128 ==> pre.slab_128_bytes@.free_addrs.contains(ptr))
            && (tier == SlabSize::Slab256 ==> pre.slab_256_bytes@.free_addrs.contains(ptr))
            && (tier == SlabSize::Slab512 ==> pre.slab_512_bytes@.free_addrs.contains(ptr)),
        // Ghost map: inserted ptr -> size.
        post.alloc_map@ == pre.alloc_map@.insert(ptr as int, size as nat),
        // New allocation wasn't previously live.
        !pre.alloc_map@.dom().contains(ptr as int),
        0 < ptr,
    ensures
        post.internal_inv(),
{
    // Common facts needed by sub-lemmas.
    assert(post.alloc_map@.dom().contains(ptr as int));
    assert(forall|a: int| pre.alloc_map@.dom().contains(a)
        ==> post.alloc_map@.dom().contains(a));

    Self::lemma_alloc_forward(pre, post, tier, ptr, size);
    Self::lemma_alloc_reverse_8(pre, post, tier, ptr, size);
    Self::lemma_alloc_reverse_16(pre, post, tier, ptr, size);
    Self::lemma_alloc_reverse_32(pre, post, tier, ptr, size);
    Self::lemma_alloc_reverse_64(pre, post, tier, ptr, size);
    Self::lemma_alloc_reverse_128(pre, post, tier, ptr, size);
    Self::lemma_alloc_reverse_256(pre, post, tier, ptr, size);
    Self::lemma_alloc_reverse_512(pre, post, tier, ptr, size);
}

/// View-level invariant preservation for `allocate` Ok.
proof fn lemma_alloc_preserves_view_inv(
    pre: &Kheap,
    post: &Kheap,
    ptr: usize,
    size: usize,
)
    requires
        pre@.inv(),
        // Ghost map transition.
        post.alloc_map@ == pre.alloc_map@.insert(ptr as int, size as nat),
        !pre.alloc_map@.dom().contains(ptr as int),
        0 < ptr,
        1 <= size <= MAX_SLAB_SIZE,
        // Overlap-with-new pre-proved by caller.
        forall|a2: int| #[trigger] pre.alloc_map@.dom().contains(a2)
            ==> ptr as int + size as int <= a2
                || a2 + pre.alloc_map@[a2] as int <= ptr as int,
    ensures
        post@.inv(),
        post@ == pre@.spec_allocate(ptr as int, size as nat),
{
    Self::lemma_alloc_positive(pre, post, ptr, size);
    assert(pre@.allocations == pre.alloc_map);
    Self::lemma_alloc_overlap_bundle(
        pre.alloc_map@, post.alloc_map@, ptr as int, size as nat,
    );
    assert(post@ =~= pre@.spec_allocate(ptr as int, size as nat));
}

/// Closes all proof obligations for the successful path of `Kheap::allocate`.
proof fn lemma_allocate_ok(
    pre: &Kheap,
    post: &Kheap,
    tier: SlabSize,
    ptr: usize,
    size: usize,
    align: usize,
)
    requires
        pre.inv(),
        size >= 1,
        is_supported_tier(tier as usize),
        tier as usize >= size,
        forall|s: usize| is_supported_tier(s) && s >= size ==> tier as usize <= s,
        is_pow2(align as int),
        align <= size,
        align <= MAX_SLAB_SIZE,
        post.slab_8_bytes.inv(),
        post.slab_16_bytes.inv(),
        post.slab_32_bytes.inv(),
        post.slab_64_bytes.inv(),
        post.slab_128_bytes.inv(),
        post.slab_256_bytes.inv(),
        post.slab_512_bytes.inv(),
        post.slab_8_bytes@.block_size == 8,
        post.slab_16_bytes@.block_size == 16,
        post.slab_32_bytes@.block_size == 32,
        post.slab_64_bytes@.block_size == 64,
        post.slab_128_bytes@.block_size == 128,
        post.slab_256_bytes@.block_size == 256,
        post.slab_512_bytes@.block_size == 512,
        post.slab_8_bytes@.start_addr == pre.slab_8_bytes@.start_addr,
        post.slab_8_bytes@.end_addr == pre.slab_8_bytes@.end_addr,
        post.slab_16_bytes@.start_addr == pre.slab_16_bytes@.start_addr,
        post.slab_16_bytes@.end_addr == pre.slab_16_bytes@.end_addr,
        post.slab_32_bytes@.start_addr == pre.slab_32_bytes@.start_addr,
        post.slab_32_bytes@.end_addr == pre.slab_32_bytes@.end_addr,
        post.slab_64_bytes@.start_addr == pre.slab_64_bytes@.start_addr,
        post.slab_64_bytes@.end_addr == pre.slab_64_bytes@.end_addr,
        post.slab_128_bytes@.start_addr == pre.slab_128_bytes@.start_addr,
        post.slab_128_bytes@.end_addr == pre.slab_128_bytes@.end_addr,
        post.slab_256_bytes@.start_addr == pre.slab_256_bytes@.start_addr,
        post.slab_256_bytes@.end_addr == pre.slab_256_bytes@.end_addr,
        post.slab_512_bytes@.start_addr == pre.slab_512_bytes@.start_addr,
        post.slab_512_bytes@.end_addr == pre.slab_512_bytes@.end_addr,
        post.slab_8_bytes@.allocated_addrs == if tier == SlabSize::Slab8 {
            pre.slab_8_bytes@.allocated_addrs.insert(ptr)
        } else { pre.slab_8_bytes@.allocated_addrs },
        post.slab_16_bytes@.allocated_addrs == if tier == SlabSize::Slab16 {
            pre.slab_16_bytes@.allocated_addrs.insert(ptr)
        } else { pre.slab_16_bytes@.allocated_addrs },
        post.slab_32_bytes@.allocated_addrs == if tier == SlabSize::Slab32 {
            pre.slab_32_bytes@.allocated_addrs.insert(ptr)
        } else { pre.slab_32_bytes@.allocated_addrs },
        post.slab_64_bytes@.allocated_addrs == if tier == SlabSize::Slab64 {
            pre.slab_64_bytes@.allocated_addrs.insert(ptr)
        } else { pre.slab_64_bytes@.allocated_addrs },
        post.slab_128_bytes@.allocated_addrs == if tier == SlabSize::Slab128 {
            pre.slab_128_bytes@.allocated_addrs.insert(ptr)
        } else { pre.slab_128_bytes@.allocated_addrs },
        post.slab_256_bytes@.allocated_addrs == if tier == SlabSize::Slab256 {
            pre.slab_256_bytes@.allocated_addrs.insert(ptr)
        } else { pre.slab_256_bytes@.allocated_addrs },
        post.slab_512_bytes@.allocated_addrs == if tier == SlabSize::Slab512 {
            pre.slab_512_bytes@.allocated_addrs.insert(ptr)
        } else { pre.slab_512_bytes@.allocated_addrs },
        (tier == SlabSize::Slab8 ==> pre.slab_8_bytes@.free_addrs.contains(ptr))
            && (tier == SlabSize::Slab16 ==> pre.slab_16_bytes@.free_addrs.contains(ptr))
            && (tier == SlabSize::Slab32 ==> pre.slab_32_bytes@.free_addrs.contains(ptr))
            && (tier == SlabSize::Slab64 ==> pre.slab_64_bytes@.free_addrs.contains(ptr))
            && (tier == SlabSize::Slab128 ==> pre.slab_128_bytes@.free_addrs.contains(ptr))
            && (tier == SlabSize::Slab256 ==> pre.slab_256_bytes@.free_addrs.contains(ptr))
            && (tier == SlabSize::Slab512 ==> pre.slab_512_bytes@.free_addrs.contains(ptr)),
        (tier == SlabSize::Slab8 ==> ptr as int % 8 == 0)
            && (tier == SlabSize::Slab16 ==> ptr as int % 16 == 0)
            && (tier == SlabSize::Slab32 ==> ptr as int % 32 == 0)
            && (tier == SlabSize::Slab64 ==> ptr as int % 64 == 0)
            && (tier == SlabSize::Slab128 ==> ptr as int % 128 == 0)
            && (tier == SlabSize::Slab256 ==> ptr as int % 256 == 0)
            && (tier == SlabSize::Slab512 ==> ptr as int % 512 == 0),
        post.alloc_map@ == pre.alloc_map@.insert(ptr as int, size as nat),
        !pre.alloc_map@.dom().contains(ptr as int),
        0 < ptr,
    ensures
        post.inv(),
        post@ == pre@.spec_allocate(ptr as int, size as nat),
        ptr as int % align as int == 0,
{
    lemma_tier_size_bounds(tier, size);
    Kheap::lemma_alloc_preserves_internal_inv(pre, post, tier, ptr, size);
    Kheap::lemma_alloc_overlap_with_new(pre, tier, ptr, size);
    Kheap::lemma_alloc_preserves_view_inv(pre, post, ptr, size);
    Kheap::lemma_pow2_le_512_supported(align);
    Kheap::lemma_tier_align(ptr, size, align, tier);
    assert(post.inv());
}

/// Closes all proof obligations for the failing slab-allocation path of `Kheap::allocate`.
proof fn lemma_allocate_err(pre: &Kheap, post: &Kheap, tier: SlabSize)
    requires
        pre.inv(),
        post.slab_8_bytes.inv(),
        post.slab_16_bytes.inv(),
        post.slab_32_bytes.inv(),
        post.slab_64_bytes.inv(),
        post.slab_128_bytes.inv(),
        post.slab_256_bytes.inv(),
        post.slab_512_bytes.inv(),
        post.slab_8_bytes@ == pre.slab_8_bytes@,
        post.slab_16_bytes@ == pre.slab_16_bytes@,
        post.slab_32_bytes@ == pre.slab_32_bytes@,
        post.slab_64_bytes@ == pre.slab_64_bytes@,
        post.slab_128_bytes@ == pre.slab_128_bytes@,
        post.slab_256_bytes@ == pre.slab_256_bytes@,
        post.slab_512_bytes@ == pre.slab_512_bytes@,
        post.alloc_map@ == pre.alloc_map@,
        (tier == SlabSize::Slab8 ==> pre.slab_8_bytes@.free_addrs =~= Set::<usize>::empty())
            && (tier == SlabSize::Slab16 ==> pre.slab_16_bytes@.free_addrs =~= Set::<usize>::empty())
            && (tier == SlabSize::Slab32 ==> pre.slab_32_bytes@.free_addrs =~= Set::<usize>::empty())
            && (tier == SlabSize::Slab64 ==> pre.slab_64_bytes@.free_addrs =~= Set::<usize>::empty())
            && (tier == SlabSize::Slab128 ==> pre.slab_128_bytes@.free_addrs =~= Set::<usize>::empty())
            && (tier == SlabSize::Slab256 ==> pre.slab_256_bytes@.free_addrs =~= Set::<usize>::empty())
            && (tier == SlabSize::Slab512 ==> pre.slab_512_bytes@.free_addrs =~= Set::<usize>::empty()),
    ensures
        post.inv(),
        post@ == pre@,
        !(pre.alloc_map@ =~= Map::<int, nat>::empty()),
{
    Kheap::lemma_alloc_err_preserves_inv(pre, post, tier);
    Kheap::lemma_alloc_err_implies_nonempty(pre, tier);
}

/// Closes `Kheap::allocate` obligations after the caller updates the ghost allocation map.
proof fn lemma_allocate_result(
    pre: &Kheap,
    post: &Kheap,
    tier: SlabSize,
    result: Result<*mut u8, AllocError>,
    size: usize,
    align: usize,
)
    requires
        pre.inv(),
        size >= 1,
        is_supported_tier(tier as usize),
        tier as usize >= size,
        forall|s: usize| is_supported_tier(s) && s >= size ==> tier as usize <= s,
        is_pow2(align as int),
        align <= size,
        align <= MAX_SLAB_SIZE,
        match result {
            Ok(ptr) => {
                let addr = ptr as usize;
                &&& post.slab_8_bytes.inv()
                &&& post.slab_16_bytes.inv()
                &&& post.slab_32_bytes.inv()
                &&& post.slab_64_bytes.inv()
                &&& post.slab_128_bytes.inv()
                &&& post.slab_256_bytes.inv()
                &&& post.slab_512_bytes.inv()
                &&& post.slab_8_bytes@.block_size == 8
                &&& post.slab_16_bytes@.block_size == 16
                &&& post.slab_32_bytes@.block_size == 32
                &&& post.slab_64_bytes@.block_size == 64
                &&& post.slab_128_bytes@.block_size == 128
                &&& post.slab_256_bytes@.block_size == 256
                &&& post.slab_512_bytes@.block_size == 512
                &&& post.slab_8_bytes@.start_addr == pre.slab_8_bytes@.start_addr
                &&& post.slab_8_bytes@.end_addr == pre.slab_8_bytes@.end_addr
                &&& post.slab_16_bytes@.start_addr == pre.slab_16_bytes@.start_addr
                &&& post.slab_16_bytes@.end_addr == pre.slab_16_bytes@.end_addr
                &&& post.slab_32_bytes@.start_addr == pre.slab_32_bytes@.start_addr
                &&& post.slab_32_bytes@.end_addr == pre.slab_32_bytes@.end_addr
                &&& post.slab_64_bytes@.start_addr == pre.slab_64_bytes@.start_addr
                &&& post.slab_64_bytes@.end_addr == pre.slab_64_bytes@.end_addr
                &&& post.slab_128_bytes@.start_addr == pre.slab_128_bytes@.start_addr
                &&& post.slab_128_bytes@.end_addr == pre.slab_128_bytes@.end_addr
                &&& post.slab_256_bytes@.start_addr == pre.slab_256_bytes@.start_addr
                &&& post.slab_256_bytes@.end_addr == pre.slab_256_bytes@.end_addr
                &&& post.slab_512_bytes@.start_addr == pre.slab_512_bytes@.start_addr
                &&& post.slab_512_bytes@.end_addr == pre.slab_512_bytes@.end_addr
                &&& post.slab_8_bytes@.allocated_addrs == if tier == SlabSize::Slab8 {
                    pre.slab_8_bytes@.allocated_addrs.insert(addr)
                } else { pre.slab_8_bytes@.allocated_addrs }
                &&& post.slab_16_bytes@.allocated_addrs == if tier == SlabSize::Slab16 {
                    pre.slab_16_bytes@.allocated_addrs.insert(addr)
                } else { pre.slab_16_bytes@.allocated_addrs }
                &&& post.slab_32_bytes@.allocated_addrs == if tier == SlabSize::Slab32 {
                    pre.slab_32_bytes@.allocated_addrs.insert(addr)
                } else { pre.slab_32_bytes@.allocated_addrs }
                &&& post.slab_64_bytes@.allocated_addrs == if tier == SlabSize::Slab64 {
                    pre.slab_64_bytes@.allocated_addrs.insert(addr)
                } else { pre.slab_64_bytes@.allocated_addrs }
                &&& post.slab_128_bytes@.allocated_addrs == if tier == SlabSize::Slab128 {
                    pre.slab_128_bytes@.allocated_addrs.insert(addr)
                } else { pre.slab_128_bytes@.allocated_addrs }
                &&& post.slab_256_bytes@.allocated_addrs == if tier == SlabSize::Slab256 {
                    pre.slab_256_bytes@.allocated_addrs.insert(addr)
                } else { pre.slab_256_bytes@.allocated_addrs }
                &&& post.slab_512_bytes@.allocated_addrs == if tier == SlabSize::Slab512 {
                    pre.slab_512_bytes@.allocated_addrs.insert(addr)
                } else { pre.slab_512_bytes@.allocated_addrs }
                &&& (tier == SlabSize::Slab8 ==> pre.slab_8_bytes@.free_addrs.contains(addr))
                &&& (tier == SlabSize::Slab16 ==> pre.slab_16_bytes@.free_addrs.contains(addr))
                &&& (tier == SlabSize::Slab32 ==> pre.slab_32_bytes@.free_addrs.contains(addr))
                &&& (tier == SlabSize::Slab64 ==> pre.slab_64_bytes@.free_addrs.contains(addr))
                &&& (tier == SlabSize::Slab128 ==> pre.slab_128_bytes@.free_addrs.contains(addr))
                &&& (tier == SlabSize::Slab256 ==> pre.slab_256_bytes@.free_addrs.contains(addr))
                &&& (tier == SlabSize::Slab512 ==> pre.slab_512_bytes@.free_addrs.contains(addr))
                &&& (tier == SlabSize::Slab8 ==> addr as int % 8 == 0)
                &&& (tier == SlabSize::Slab16 ==> addr as int % 16 == 0)
                &&& (tier == SlabSize::Slab32 ==> addr as int % 32 == 0)
                &&& (tier == SlabSize::Slab64 ==> addr as int % 64 == 0)
                &&& (tier == SlabSize::Slab128 ==> addr as int % 128 == 0)
                &&& (tier == SlabSize::Slab256 ==> addr as int % 256 == 0)
                &&& (tier == SlabSize::Slab512 ==> addr as int % 512 == 0)
                &&& post.alloc_map@ == pre.alloc_map@.insert(addr as int, size as nat)
                &&& !pre.alloc_map@.dom().contains(addr as int)
                &&& 0 < addr
            },
            Err(_) => {
                &&& post.slab_8_bytes.inv()
                &&& post.slab_16_bytes.inv()
                &&& post.slab_32_bytes.inv()
                &&& post.slab_64_bytes.inv()
                &&& post.slab_128_bytes.inv()
                &&& post.slab_256_bytes.inv()
                &&& post.slab_512_bytes.inv()
                &&& post.slab_8_bytes@ == pre.slab_8_bytes@
                &&& post.slab_16_bytes@ == pre.slab_16_bytes@
                &&& post.slab_32_bytes@ == pre.slab_32_bytes@
                &&& post.slab_64_bytes@ == pre.slab_64_bytes@
                &&& post.slab_128_bytes@ == pre.slab_128_bytes@
                &&& post.slab_256_bytes@ == pre.slab_256_bytes@
                &&& post.slab_512_bytes@ == pre.slab_512_bytes@
                &&& post.alloc_map@ == pre.alloc_map@
                &&& (tier == SlabSize::Slab8 ==> pre.slab_8_bytes@.free_addrs =~= Set::<usize>::empty())
                &&& (tier == SlabSize::Slab16 ==> pre.slab_16_bytes@.free_addrs =~= Set::<usize>::empty())
                &&& (tier == SlabSize::Slab32 ==> pre.slab_32_bytes@.free_addrs =~= Set::<usize>::empty())
                &&& (tier == SlabSize::Slab64 ==> pre.slab_64_bytes@.free_addrs =~= Set::<usize>::empty())
                &&& (tier == SlabSize::Slab128 ==> pre.slab_128_bytes@.free_addrs =~= Set::<usize>::empty())
                &&& (tier == SlabSize::Slab256 ==> pre.slab_256_bytes@.free_addrs =~= Set::<usize>::empty())
                &&& (tier == SlabSize::Slab512 ==> pre.slab_512_bytes@.free_addrs =~= Set::<usize>::empty())
            },
        },
    ensures
        post.inv(),
        match result {
            Ok(ptr) => {
                let addr = ptr as usize;
                &&& post@ == pre@.spec_allocate(addr as int, size as nat)
                &&& addr as int % align as int == 0
            },
            Err(_) => {
                &&& post@ == pre@
                &&& !(pre.alloc_map@ =~= Map::<int, nat>::empty())
            },
        },
{
    match result {
        Ok(ptr) => {
            let addr = ptr as usize;
            Kheap::lemma_allocate_ok(pre, post, tier, addr, size, align);
        },
        Err(_) => {
            Kheap::lemma_allocate_err(pre, post, tier);
        },
    }
}

proof fn lemma_alloc_forward(pre: &Kheap, post: &Kheap, tier: SlabSize, ptr: usize, size: usize)
    requires
        pre.inv(),
        (tier == SlabSize::Slab8 && 1 <= size <= 8)
            || (tier == SlabSize::Slab16 && 9 <= size <= 16)
            || (tier == SlabSize::Slab32 && 17 <= size <= 32)
            || (tier == SlabSize::Slab64 && 33 <= size <= 64)
            || (tier == SlabSize::Slab128 && 65 <= size <= 128)
            || (tier == SlabSize::Slab256 && 129 <= size <= 256)
            || (tier == SlabSize::Slab512 && 257 <= size <= 512),
        post.slab_8_bytes@.allocated_addrs == if tier == SlabSize::Slab8 {
            pre.slab_8_bytes@.allocated_addrs.insert(ptr)
        } else { pre.slab_8_bytes@.allocated_addrs },
        post.slab_16_bytes@.allocated_addrs == if tier == SlabSize::Slab16 {
            pre.slab_16_bytes@.allocated_addrs.insert(ptr)
        } else { pre.slab_16_bytes@.allocated_addrs },
        post.slab_32_bytes@.allocated_addrs == if tier == SlabSize::Slab32 {
            pre.slab_32_bytes@.allocated_addrs.insert(ptr)
        } else { pre.slab_32_bytes@.allocated_addrs },
        post.slab_64_bytes@.allocated_addrs == if tier == SlabSize::Slab64 {
            pre.slab_64_bytes@.allocated_addrs.insert(ptr)
        } else { pre.slab_64_bytes@.allocated_addrs },
        post.slab_128_bytes@.allocated_addrs == if tier == SlabSize::Slab128 {
            pre.slab_128_bytes@.allocated_addrs.insert(ptr)
        } else { pre.slab_128_bytes@.allocated_addrs },
        post.slab_256_bytes@.allocated_addrs == if tier == SlabSize::Slab256 {
            pre.slab_256_bytes@.allocated_addrs.insert(ptr)
        } else { pre.slab_256_bytes@.allocated_addrs },
        post.slab_512_bytes@.allocated_addrs == if tier == SlabSize::Slab512 {
            pre.slab_512_bytes@.allocated_addrs.insert(ptr)
        } else { pre.slab_512_bytes@.allocated_addrs },
        post.alloc_map@ == pre.alloc_map@.insert(ptr as int, size as nat),
        !pre.alloc_map@.dom().contains(ptr as int),
        0 < ptr,
    ensures
        forall|addr: int| #[trigger] post.alloc_map@.dom().contains(addr) ==> ({
            let sz = post.alloc_map@[addr];
            let a = addr as usize;
            &&& 0 < addr <= usize::MAX
            &&& ({
                    ||| (1 <= sz <= 8 && post.slab_8_bytes@.allocated_addrs.contains(a))
                    ||| (9 <= sz <= 16 && post.slab_16_bytes@.allocated_addrs.contains(a))
                    ||| (17 <= sz <= 32 && post.slab_32_bytes@.allocated_addrs.contains(a))
                    ||| (33 <= sz <= 64 && post.slab_64_bytes@.allocated_addrs.contains(a))
                    ||| (65 <= sz <= 128 && post.slab_128_bytes@.allocated_addrs.contains(a))
                    ||| (129 <= sz <= 256 && post.slab_256_bytes@.allocated_addrs.contains(a))
                    ||| (257 <= sz <= 512 && post.slab_512_bytes@.allocated_addrs.contains(a))
                })
        }),
{
    assert forall|addr: int| #[trigger] post.alloc_map@.dom().contains(addr)
        implies ({
            let sz = post.alloc_map@[addr];
            let a = addr as usize;
            &&& 0 < addr <= usize::MAX
            &&& ({
                    ||| (1 <= sz <= 8 && post.slab_8_bytes@.allocated_addrs.contains(a))
                    ||| (9 <= sz <= 16 && post.slab_16_bytes@.allocated_addrs.contains(a))
                    ||| (17 <= sz <= 32 && post.slab_32_bytes@.allocated_addrs.contains(a))
                    ||| (33 <= sz <= 64 && post.slab_64_bytes@.allocated_addrs.contains(a))
                    ||| (65 <= sz <= 128 && post.slab_128_bytes@.allocated_addrs.contains(a))
                    ||| (129 <= sz <= 256 && post.slab_256_bytes@.allocated_addrs.contains(a))
                    ||| (257 <= sz <= 512 && post.slab_512_bytes@.allocated_addrs.contains(a))
                })
        })
    by {
        if addr == ptr as int {
            assert(post.alloc_map@[addr] == size as nat);
        } else {
            assert(pre.alloc_map@.dom().contains(addr));
            assert(pre.alloc_map@[addr] == post.alloc_map@[addr]);
        }
    };
}

proof fn lemma_alloc_reverse_8(pre: &Kheap, post: &Kheap, tier: SlabSize, ptr: usize, size: usize)
    requires
        pre.inv(),
        post.slab_8_bytes@.allocated_addrs == if tier == SlabSize::Slab8 {
            pre.slab_8_bytes@.allocated_addrs.insert(ptr)
        } else { pre.slab_8_bytes@.allocated_addrs },
        forall|a: int| pre.alloc_map@.dom().contains(a) ==> post.alloc_map@.dom().contains(a),
        post.alloc_map@.dom().contains(ptr as int),
        post.alloc_map@ == pre.alloc_map@.insert(ptr as int, size as nat),
        !pre.alloc_map@.dom().contains(ptr as int),
        tier == SlabSize::Slab8 ==> 1 <= size <= 8,
    ensures
        forall|a: usize| #[trigger] post.slab_8_bytes@.allocated_addrs.contains(a)
            ==> post.alloc_map@.dom().contains(a as int)
                && 1 <= post.alloc_map@[a as int] <= 8,
{
    assert forall|a: usize| #[trigger] post.slab_8_bytes@.allocated_addrs.contains(a)
        implies post.alloc_map@.dom().contains(a as int)
            && 1 <= post.alloc_map@[a as int] <= 8
    by {
        if tier == SlabSize::Slab8 && a == ptr { }
        else { assert(pre.slab_8_bytes@.allocated_addrs.contains(a)); }
    };
}

proof fn lemma_alloc_reverse_16(pre: &Kheap, post: &Kheap, tier: SlabSize, ptr: usize, size: usize)
    requires
        pre.inv(),
        post.slab_16_bytes@.allocated_addrs == if tier == SlabSize::Slab16 {
            pre.slab_16_bytes@.allocated_addrs.insert(ptr)
        } else { pre.slab_16_bytes@.allocated_addrs },
        forall|a: int| pre.alloc_map@.dom().contains(a) ==> post.alloc_map@.dom().contains(a),
        post.alloc_map@.dom().contains(ptr as int),
        post.alloc_map@ == pre.alloc_map@.insert(ptr as int, size as nat),
        !pre.alloc_map@.dom().contains(ptr as int),
        tier == SlabSize::Slab16 ==> 9 <= size <= 16,
    ensures
        forall|a: usize| #[trigger] post.slab_16_bytes@.allocated_addrs.contains(a)
            ==> post.alloc_map@.dom().contains(a as int)
                && 9 <= post.alloc_map@[a as int] <= 16,
{
    assert forall|a: usize| #[trigger] post.slab_16_bytes@.allocated_addrs.contains(a)
        implies post.alloc_map@.dom().contains(a as int)
            && 9 <= post.alloc_map@[a as int] <= 16
    by {
        if tier == SlabSize::Slab16 && a == ptr { }
        else { assert(pre.slab_16_bytes@.allocated_addrs.contains(a)); }
    };
}

proof fn lemma_alloc_reverse_32(pre: &Kheap, post: &Kheap, tier: SlabSize, ptr: usize, size: usize)
    requires
        pre.inv(),
        post.slab_32_bytes@.allocated_addrs == if tier == SlabSize::Slab32 {
            pre.slab_32_bytes@.allocated_addrs.insert(ptr)
        } else { pre.slab_32_bytes@.allocated_addrs },
        forall|a: int| pre.alloc_map@.dom().contains(a) ==> post.alloc_map@.dom().contains(a),
        post.alloc_map@.dom().contains(ptr as int),
        post.alloc_map@ == pre.alloc_map@.insert(ptr as int, size as nat),
        !pre.alloc_map@.dom().contains(ptr as int),
        tier == SlabSize::Slab32 ==> 17 <= size <= 32,
    ensures
        forall|a: usize| #[trigger] post.slab_32_bytes@.allocated_addrs.contains(a)
            ==> post.alloc_map@.dom().contains(a as int)
                && 17 <= post.alloc_map@[a as int] <= 32,
{
    assert forall|a: usize| #[trigger] post.slab_32_bytes@.allocated_addrs.contains(a)
        implies post.alloc_map@.dom().contains(a as int)
            && 17 <= post.alloc_map@[a as int] <= 32
    by {
        if tier == SlabSize::Slab32 && a == ptr { }
        else { assert(pre.slab_32_bytes@.allocated_addrs.contains(a)); }
    };
}

proof fn lemma_alloc_reverse_64(pre: &Kheap, post: &Kheap, tier: SlabSize, ptr: usize, size: usize)
    requires
        pre.inv(),
        post.slab_64_bytes@.allocated_addrs == if tier == SlabSize::Slab64 {
            pre.slab_64_bytes@.allocated_addrs.insert(ptr)
        } else { pre.slab_64_bytes@.allocated_addrs },
        forall|a: int| pre.alloc_map@.dom().contains(a) ==> post.alloc_map@.dom().contains(a),
        post.alloc_map@.dom().contains(ptr as int),
        post.alloc_map@ == pre.alloc_map@.insert(ptr as int, size as nat),
        !pre.alloc_map@.dom().contains(ptr as int),
        tier == SlabSize::Slab64 ==> 33 <= size <= 64,
    ensures
        forall|a: usize| #[trigger] post.slab_64_bytes@.allocated_addrs.contains(a)
            ==> post.alloc_map@.dom().contains(a as int)
                && 33 <= post.alloc_map@[a as int] <= 64,
{
    assert forall|a: usize| #[trigger] post.slab_64_bytes@.allocated_addrs.contains(a)
        implies post.alloc_map@.dom().contains(a as int)
            && 33 <= post.alloc_map@[a as int] <= 64
    by {
        if tier == SlabSize::Slab64 && a == ptr { }
        else { assert(pre.slab_64_bytes@.allocated_addrs.contains(a)); }
    };
}

proof fn lemma_alloc_reverse_128(pre: &Kheap, post: &Kheap, tier: SlabSize, ptr: usize, size: usize)
    requires
        pre.inv(),
        post.slab_128_bytes@.allocated_addrs == if tier == SlabSize::Slab128 {
            pre.slab_128_bytes@.allocated_addrs.insert(ptr)
        } else { pre.slab_128_bytes@.allocated_addrs },
        forall|a: int| pre.alloc_map@.dom().contains(a) ==> post.alloc_map@.dom().contains(a),
        post.alloc_map@.dom().contains(ptr as int),
        post.alloc_map@ == pre.alloc_map@.insert(ptr as int, size as nat),
        !pre.alloc_map@.dom().contains(ptr as int),
        tier == SlabSize::Slab128 ==> 65 <= size <= 128,
    ensures
        forall|a: usize| #[trigger] post.slab_128_bytes@.allocated_addrs.contains(a)
            ==> post.alloc_map@.dom().contains(a as int)
                && 65 <= post.alloc_map@[a as int] <= 128,
{
    assert forall|a: usize| #[trigger] post.slab_128_bytes@.allocated_addrs.contains(a)
        implies post.alloc_map@.dom().contains(a as int)
            && 65 <= post.alloc_map@[a as int] <= 128
    by {
        if tier == SlabSize::Slab128 && a == ptr { }
        else { assert(pre.slab_128_bytes@.allocated_addrs.contains(a)); }
    };
}

proof fn lemma_alloc_reverse_256(pre: &Kheap, post: &Kheap, tier: SlabSize, ptr: usize, size: usize)
    requires
        pre.inv(),
        post.slab_256_bytes@.allocated_addrs == if tier == SlabSize::Slab256 {
            pre.slab_256_bytes@.allocated_addrs.insert(ptr)
        } else { pre.slab_256_bytes@.allocated_addrs },
        forall|a: int| pre.alloc_map@.dom().contains(a) ==> post.alloc_map@.dom().contains(a),
        post.alloc_map@.dom().contains(ptr as int),
        post.alloc_map@ == pre.alloc_map@.insert(ptr as int, size as nat),
        !pre.alloc_map@.dom().contains(ptr as int),
        tier == SlabSize::Slab256 ==> 129 <= size <= 256,
    ensures
        forall|a: usize| #[trigger] post.slab_256_bytes@.allocated_addrs.contains(a)
            ==> post.alloc_map@.dom().contains(a as int)
                && 129 <= post.alloc_map@[a as int] <= 256,
{
    assert forall|a: usize| #[trigger] post.slab_256_bytes@.allocated_addrs.contains(a)
        implies post.alloc_map@.dom().contains(a as int)
            && 129 <= post.alloc_map@[a as int] <= 256
    by {
        if tier == SlabSize::Slab256 && a == ptr { }
        else { assert(pre.slab_256_bytes@.allocated_addrs.contains(a)); }
    };
}

proof fn lemma_alloc_reverse_512(pre: &Kheap, post: &Kheap, tier: SlabSize, ptr: usize, size: usize)
    requires
        pre.inv(),
        post.slab_512_bytes@.allocated_addrs == if tier == SlabSize::Slab512 {
            pre.slab_512_bytes@.allocated_addrs.insert(ptr)
        } else { pre.slab_512_bytes@.allocated_addrs },
        forall|a: int| pre.alloc_map@.dom().contains(a) ==> post.alloc_map@.dom().contains(a),
        post.alloc_map@.dom().contains(ptr as int),
        post.alloc_map@ == pre.alloc_map@.insert(ptr as int, size as nat),
        !pre.alloc_map@.dom().contains(ptr as int),
        tier == SlabSize::Slab512 ==> 257 <= size <= 512,
    ensures
        forall|a: usize| #[trigger] post.slab_512_bytes@.allocated_addrs.contains(a)
            ==> post.alloc_map@.dom().contains(a as int)
                && 257 <= post.alloc_map@[a as int] <= 512,
{
    assert forall|a: usize| #[trigger] post.slab_512_bytes@.allocated_addrs.contains(a)
        implies post.alloc_map@.dom().contains(a as int)
            && 257 <= post.alloc_map@[a as int] <= 512
    by {
        if tier == SlabSize::Slab512 && a == ptr { }
        else { assert(pre.slab_512_bytes@.allocated_addrs.contains(a)); }
    };
}

proof fn lemma_alloc_positive(pre: &Kheap, post: &Kheap, ptr: usize, size: usize)
    requires
        pre@.inv(),
        post.alloc_map@ == pre.alloc_map@.insert(ptr as int, size as nat),
        0 < ptr,
        0 < size,
    ensures
        forall|a: int| #[trigger] post.alloc_map@.dom().contains(a)
            ==> post.alloc_map@[a] > 0 && a > 0,
{
    assert forall|a: int| #[trigger] post.alloc_map@.dom().contains(a)
        implies post.alloc_map@[a] > 0 && a > 0
    by {
        if a == ptr as int {
            assert(post.alloc_map@[a] == size as nat);
        } else {
            assert(pre.alloc_map@.dom().contains(a));
            assert(pre@.allocations.dom().contains(a));
        }
    };
}

proof fn lemma_alloc_overlap_with_new(pre: &Kheap, tier: SlabSize, ptr: usize, size: usize)
    requires
        pre.inv(),
        (tier == SlabSize::Slab8 && 1 <= size <= 8)
            || (tier == SlabSize::Slab16 && 9 <= size <= 16)
            || (tier == SlabSize::Slab32 && 17 <= size <= 32)
            || (tier == SlabSize::Slab64 && 33 <= size <= 64)
            || (tier == SlabSize::Slab128 && 65 <= size <= 128)
            || (tier == SlabSize::Slab256 && 129 <= size <= 256)
            || (tier == SlabSize::Slab512 && 257 <= size <= 512),
        (tier == SlabSize::Slab8 ==> pre.slab_8_bytes@.free_addrs.contains(ptr))
            && (tier == SlabSize::Slab16 ==> pre.slab_16_bytes@.free_addrs.contains(ptr))
            && (tier == SlabSize::Slab32 ==> pre.slab_32_bytes@.free_addrs.contains(ptr))
            && (tier == SlabSize::Slab64 ==> pre.slab_64_bytes@.free_addrs.contains(ptr))
            && (tier == SlabSize::Slab128 ==> pre.slab_128_bytes@.free_addrs.contains(ptr))
            && (tier == SlabSize::Slab256 ==> pre.slab_256_bytes@.free_addrs.contains(ptr))
            && (tier == SlabSize::Slab512 ==> pre.slab_512_bytes@.free_addrs.contains(ptr)),
        !pre.alloc_map@.dom().contains(ptr as int),
        0 < ptr,
    ensures
        forall|a2: int| #[trigger] pre.alloc_map@.dom().contains(a2)
            ==> ptr as int + size as int <= a2
                || a2 + pre.alloc_map@[a2] as int <= ptr as int,
{
    assert forall|a2: int| #[trigger] pre.alloc_map@.dom().contains(a2)
        implies ptr as int + size as int <= a2
            || a2 + pre.alloc_map@[a2] as int <= ptr as int
    by {
        Self::lemma_no_overlap_with_new_ptr(pre, tier, ptr, size, a2);
    };
}

proof fn lemma_alloc_overlap_bundle(
    pre_map: Map<int, nat>,
    post_map: Map<int, nat>,
    ptr: int,
    size: nat,
)
    requires
        post_map == pre_map.insert(ptr, size),
        !pre_map.dom().contains(ptr),
        forall|a2: int| #[trigger] pre_map.dom().contains(a2)
            ==> ptr + size as int <= a2 || a2 + pre_map[a2] as int <= ptr,
        forall|a1: int, a2: int| #![auto]
            pre_map.dom().contains(a1)
            && pre_map.dom().contains(a2)
            && a1 != a2
            ==> a1 + pre_map[a1] as int <= a2
                || a2 + pre_map[a2] as int <= a1,
    ensures
        forall|a1: int, a2: int| #![auto]
            post_map.dom().contains(a1)
            && post_map.dom().contains(a2)
            && a1 != a2
            ==> a1 + post_map[a1] as int <= a2
                || a2 + post_map[a2] as int <= a1,
{
    assert forall|a1: int, a2: int| #![auto]
        post_map.dom().contains(a1)
        && post_map.dom().contains(a2)
        && a1 != a2
        implies a1 + post_map[a1] as int <= a2
            || a2 + post_map[a2] as int <= a1
    by {
        if a1 == ptr {
            assert(pre_map.dom().contains(a2));
        } else if a2 == ptr {
            assert(pre_map.dom().contains(a1));
        } else {
            assert(pre_map.dom().contains(a1));
            assert(pre_map.dom().contains(a2));
        }
    };
}

proof fn lemma_pow2_le_512_supported(align: usize)
    requires
        is_pow2(align as int),
        align <= MAX_SLAB_SIZE,
    ensures
        align == 1 || align == 2 || align == 4 || align == 8 || align == 16
            || align == 32 || align == 64 || align == 128 || align == 256 || align == 512,
    decreases align,
{
    reveal(is_pow2);
    if align <= 1 {
        assert(align == 1);
    } else {
        assert(align as int % 2 == 0);
        assert(is_pow2(align as int / 2));
        assert(align / 2 < align) by (nonlinear_arith) requires align > 1;
        assert(align / 2 <= MAX_SLAB_SIZE);
        assert(align as int / 2 == (align / 2) as int) by (nonlinear_arith)
            requires align as int % 2 == 0;
        Self::lemma_pow2_le_512_supported(align / 2);
        assert(align == 2 * (align / 2)) by (nonlinear_arith)
            requires align as int % 2 == 0;
    }
}

/// Alignment of the slab-returned pointer: since `align <= size <= block_size`
/// and both are powers of two, `block_size % align == 0`; combined with
/// slab's `ptr % block_size == 0`, we get `ptr % align == 0`.
proof fn lemma_tier_align(ptr: usize, size: usize, align: usize, tier: SlabSize)
    requires
        0 < align,
        align <= size,
        size <= MAX_SLAB_SIZE,
        align == 1 || align == 2 || align == 4 || align == 8 || align == 16
            || align == 32 || align == 64 || align == 128 || align == 256 || align == 512,
        (tier == SlabSize::Slab8 && 1 <= size <= 8 && ptr as int % 8 == 0)
            || (tier == SlabSize::Slab16 && 9 <= size <= 16 && ptr as int % 16 == 0)
            || (tier == SlabSize::Slab32 && 17 <= size <= 32 && ptr as int % 32 == 0)
            || (tier == SlabSize::Slab64 && 33 <= size <= 64 && ptr as int % 64 == 0)
            || (tier == SlabSize::Slab128 && 65 <= size <= 128 && ptr as int % 128 == 0)
            || (tier == SlabSize::Slab256 && 129 <= size <= 256 && ptr as int % 256 == 0)
            || (tier == SlabSize::Slab512 && 257 <= size <= 512 && ptr as int % 512 == 0),
    ensures
        ptr as int % align as int == 0,
{
    // `align <= size` and size falls into exactly one tier band. For that
    // tier, the block size `B` is the tier's value. Because align is a
    // power of two with align <= B (which is also a power of two), align
    // divides B. Slab guarantees `ptr % B == 0`, so `ptr % align == 0`.
    //
    // We expand this by the ten possible values of align, then let Z3 close
    // each goal with linear reasoning only.
    assert(ptr as int % align as int == 0) by (nonlinear_arith)
        requires
            0 < align,
            align <= size,
            size <= MAX_SLAB_SIZE,
            align == 1 || align == 2 || align == 4 || align == 8 || align == 16
                || align == 32 || align == 64 || align == 128 || align == 256 || align == 512,
            (tier == SlabSize::Slab8 && 1 <= size <= 8 && ptr as int % 8 == 0)
                || (tier == SlabSize::Slab16 && 9 <= size <= 16 && ptr as int % 16 == 0)
                || (tier == SlabSize::Slab32 && 17 <= size <= 32 && ptr as int % 32 == 0)
                || (tier == SlabSize::Slab64 && 33 <= size <= 64 && ptr as int % 64 == 0)
                || (tier == SlabSize::Slab128 && 65 <= size <= 128 && ptr as int % 128 == 0)
                || (tier == SlabSize::Slab256 && 129 <= size <= 256 && ptr as int % 256 == 0)
                || (tier == SlabSize::Slab512 && 257 <= size <= 512 && ptr as int % 512 == 0);
}

/// Closes the inv-preservation goal in the error path of allocate:
/// state is unchanged except possibly a transient slab internal, and
/// ghost map is unchanged.
proof fn lemma_alloc_err_preserves_inv(pre: &Kheap, post: &Kheap, tier: SlabSize)
    requires
        pre.inv(),
        post.slab_8_bytes.inv(),
        post.slab_16_bytes.inv(),
        post.slab_32_bytes.inv(),
        post.slab_64_bytes.inv(),
        post.slab_128_bytes.inv(),
        post.slab_256_bytes.inv(),
        post.slab_512_bytes.inv(),
        post.slab_8_bytes@ == pre.slab_8_bytes@,
        post.slab_16_bytes@ == pre.slab_16_bytes@,
        post.slab_32_bytes@ == pre.slab_32_bytes@,
        post.slab_64_bytes@ == pre.slab_64_bytes@,
        post.slab_128_bytes@ == pre.slab_128_bytes@,
        post.slab_256_bytes@ == pre.slab_256_bytes@,
        post.slab_512_bytes@ == pre.slab_512_bytes@,
        post.alloc_map@ == pre.alloc_map@,
    ensures
        post.inv(),
        post@ == pre@,
{
    // All slab views and alloc_map are identical to pre — inv holds by
    // substitutivity in internal_inv.
    assert(post.internal_inv()) by {
        assert(pre.internal_inv());
    }
}

/// When a slab tier's allocation fails (free_addrs empty), the abstract
/// alloc_map must contain at least one entry.
proof fn lemma_alloc_err_implies_nonempty(pre: &Kheap, tier: SlabSize)
    requires
        pre.inv(),
        (tier == SlabSize::Slab8 ==> pre.slab_8_bytes@.free_addrs =~= Set::<usize>::empty())
            && (tier == SlabSize::Slab16 ==> pre.slab_16_bytes@.free_addrs =~= Set::<usize>::empty())
            && (tier == SlabSize::Slab32 ==> pre.slab_32_bytes@.free_addrs =~= Set::<usize>::empty())
            && (tier == SlabSize::Slab64 ==> pre.slab_64_bytes@.free_addrs =~= Set::<usize>::empty())
            && (tier == SlabSize::Slab128 ==> pre.slab_128_bytes@.free_addrs =~= Set::<usize>::empty())
            && (tier == SlabSize::Slab256 ==> pre.slab_256_bytes@.free_addrs =~= Set::<usize>::empty())
            && (tier == SlabSize::Slab512 ==> pre.slab_512_bytes@.free_addrs =~= Set::<usize>::empty()),
    ensures
        !(pre.alloc_map@ =~= Map::<int, nat>::empty()),
{
    // For the failing tier, start_addr is block-aligned and in [start, end).
    // Completeness + free_addrs empty → start_addr ∈ allocated_addrs.
    // Reverse internal_inv → start_addr ∈ alloc_map.
    match tier {
        SlabSize::Slab8 => {
            assert(pre.slab_8_bytes@.inv());
            let a = pre.slab_8_bytes@.start_addr;
            assert(pre.slab_8_bytes@.free_addrs =~= Set::<usize>::empty());
            assert(!pre.slab_8_bytes@.free_addrs.contains(a));
            assert(pre.slab_8_bytes@.allocated_addrs.contains(a)
                || pre.slab_8_bytes@.free_addrs.contains(a));
            assert(pre.slab_8_bytes@.allocated_addrs.contains(a));
            assert(pre.internal_inv());
            assert(pre.alloc_map@.dom().contains(a as int));
        },
        SlabSize::Slab16 => {
            let a = pre.slab_16_bytes@.start_addr;
            assert(pre.slab_16_bytes@.free_addrs =~= Set::<usize>::empty());
            assert(!pre.slab_16_bytes@.free_addrs.contains(a));
            assert(pre.slab_16_bytes@.allocated_addrs.contains(a)
                || pre.slab_16_bytes@.free_addrs.contains(a));
            assert(pre.slab_16_bytes@.allocated_addrs.contains(a));
            assert(pre.internal_inv());
            assert(pre.alloc_map@.dom().contains(a as int));
        },
        SlabSize::Slab32 => {
            let a = pre.slab_32_bytes@.start_addr;
            assert(pre.slab_32_bytes@.free_addrs =~= Set::<usize>::empty());
            assert(!pre.slab_32_bytes@.free_addrs.contains(a));
            assert(pre.slab_32_bytes@.allocated_addrs.contains(a)
                || pre.slab_32_bytes@.free_addrs.contains(a));
            assert(pre.slab_32_bytes@.allocated_addrs.contains(a));
            assert(pre.internal_inv());
            assert(pre.alloc_map@.dom().contains(a as int));
        },
        SlabSize::Slab64 => {
            let a = pre.slab_64_bytes@.start_addr;
            assert(pre.slab_64_bytes@.free_addrs =~= Set::<usize>::empty());
            assert(!pre.slab_64_bytes@.free_addrs.contains(a));
            assert(pre.slab_64_bytes@.allocated_addrs.contains(a)
                || pre.slab_64_bytes@.free_addrs.contains(a));
            assert(pre.slab_64_bytes@.allocated_addrs.contains(a));
            assert(pre.internal_inv());
            assert(pre.alloc_map@.dom().contains(a as int));
        },
        SlabSize::Slab128 => {
            let a = pre.slab_128_bytes@.start_addr;
            assert(pre.slab_128_bytes@.free_addrs =~= Set::<usize>::empty());
            assert(!pre.slab_128_bytes@.free_addrs.contains(a));
            assert(pre.slab_128_bytes@.allocated_addrs.contains(a)
                || pre.slab_128_bytes@.free_addrs.contains(a));
            assert(pre.slab_128_bytes@.allocated_addrs.contains(a));
            assert(pre.internal_inv());
            assert(pre.alloc_map@.dom().contains(a as int));
        },
        SlabSize::Slab256 => {
            let a = pre.slab_256_bytes@.start_addr;
            assert(pre.slab_256_bytes@.free_addrs =~= Set::<usize>::empty());
            assert(!pre.slab_256_bytes@.free_addrs.contains(a));
            assert(pre.slab_256_bytes@.allocated_addrs.contains(a)
                || pre.slab_256_bytes@.free_addrs.contains(a));
            assert(pre.slab_256_bytes@.allocated_addrs.contains(a));
            assert(pre.internal_inv());
            assert(pre.alloc_map@.dom().contains(a as int));
        },
        SlabSize::Slab512 => {
            let a = pre.slab_512_bytes@.start_addr;
            assert(pre.slab_512_bytes@.free_addrs =~= Set::<usize>::empty());
            assert(!pre.slab_512_bytes@.free_addrs.contains(a));
            assert(pre.slab_512_bytes@.allocated_addrs.contains(a)
                || pre.slab_512_bytes@.free_addrs.contains(a));
            assert(pre.slab_512_bytes@.allocated_addrs.contains(a));
            assert(pre.internal_inv());
            assert(pre.alloc_map@.dom().contains(a as int));
        },
    }
}

/// Closes all proof obligations for `Kheap::deallocate`, keeping the
/// implementation body free of low-level proof plumbing.
proof fn lemma_deallocate_result(
    pre: &Kheap,
    post: &Kheap,
    tier: SlabSize,
    ptr: usize,
    result: Result<(), AllocError>,
    size: usize,
)
    requires
        pre.inv(),
        size >= 1,
        is_supported_tier(tier as usize),
        tier as usize >= size,
        forall|s: usize| is_supported_tier(s) && s >= size ==> tier as usize <= s,
        match result {
            Ok(()) => {
                &&& post.slab_8_bytes.inv()
                &&& post.slab_16_bytes.inv()
                &&& post.slab_32_bytes.inv()
                &&& post.slab_64_bytes.inv()
                &&& post.slab_128_bytes.inv()
                &&& post.slab_256_bytes.inv()
                &&& post.slab_512_bytes.inv()
                &&& post.slab_8_bytes@.block_size == 8
                &&& post.slab_16_bytes@.block_size == 16
                &&& post.slab_32_bytes@.block_size == 32
                &&& post.slab_64_bytes@.block_size == 64
                &&& post.slab_128_bytes@.block_size == 128
                &&& post.slab_256_bytes@.block_size == 256
                &&& post.slab_512_bytes@.block_size == 512
                &&& post.slab_8_bytes@.start_addr == pre.slab_8_bytes@.start_addr
                &&& post.slab_8_bytes@.end_addr == pre.slab_8_bytes@.end_addr
                &&& post.slab_16_bytes@.start_addr == pre.slab_16_bytes@.start_addr
                &&& post.slab_16_bytes@.end_addr == pre.slab_16_bytes@.end_addr
                &&& post.slab_32_bytes@.start_addr == pre.slab_32_bytes@.start_addr
                &&& post.slab_32_bytes@.end_addr == pre.slab_32_bytes@.end_addr
                &&& post.slab_64_bytes@.start_addr == pre.slab_64_bytes@.start_addr
                &&& post.slab_64_bytes@.end_addr == pre.slab_64_bytes@.end_addr
                &&& post.slab_128_bytes@.start_addr == pre.slab_128_bytes@.start_addr
                &&& post.slab_128_bytes@.end_addr == pre.slab_128_bytes@.end_addr
                &&& post.slab_256_bytes@.start_addr == pre.slab_256_bytes@.start_addr
                &&& post.slab_256_bytes@.end_addr == pre.slab_256_bytes@.end_addr
                &&& post.slab_512_bytes@.start_addr == pre.slab_512_bytes@.start_addr
                &&& post.slab_512_bytes@.end_addr == pre.slab_512_bytes@.end_addr
                &&& post.slab_8_bytes@.allocated_addrs == if tier == SlabSize::Slab8 {
                    pre.slab_8_bytes@.allocated_addrs.remove(ptr)
                } else { pre.slab_8_bytes@.allocated_addrs }
                &&& post.slab_16_bytes@.allocated_addrs == if tier == SlabSize::Slab16 {
                    pre.slab_16_bytes@.allocated_addrs.remove(ptr)
                } else { pre.slab_16_bytes@.allocated_addrs }
                &&& post.slab_32_bytes@.allocated_addrs == if tier == SlabSize::Slab32 {
                    pre.slab_32_bytes@.allocated_addrs.remove(ptr)
                } else { pre.slab_32_bytes@.allocated_addrs }
                &&& post.slab_64_bytes@.allocated_addrs == if tier == SlabSize::Slab64 {
                    pre.slab_64_bytes@.allocated_addrs.remove(ptr)
                } else { pre.slab_64_bytes@.allocated_addrs }
                &&& post.slab_128_bytes@.allocated_addrs == if tier == SlabSize::Slab128 {
                    pre.slab_128_bytes@.allocated_addrs.remove(ptr)
                } else { pre.slab_128_bytes@.allocated_addrs }
                &&& post.slab_256_bytes@.allocated_addrs == if tier == SlabSize::Slab256 {
                    pre.slab_256_bytes@.allocated_addrs.remove(ptr)
                } else { pre.slab_256_bytes@.allocated_addrs }
                &&& post.slab_512_bytes@.allocated_addrs == if tier == SlabSize::Slab512 {
                    pre.slab_512_bytes@.allocated_addrs.remove(ptr)
                } else { pre.slab_512_bytes@.allocated_addrs }
                &&& post.alloc_map@ == pre.alloc_map@.remove(ptr as int)
                &&& pre.alloc_map@.dom().contains(ptr as int)
                &&& (tier == SlabSize::Slab8 ==> pre.slab_8_bytes@.allocated_addrs.contains(ptr))
                &&& (tier == SlabSize::Slab16 ==> pre.slab_16_bytes@.allocated_addrs.contains(ptr))
                &&& (tier == SlabSize::Slab32 ==> pre.slab_32_bytes@.allocated_addrs.contains(ptr))
                &&& (tier == SlabSize::Slab64 ==> pre.slab_64_bytes@.allocated_addrs.contains(ptr))
                &&& (tier == SlabSize::Slab128 ==> pre.slab_128_bytes@.allocated_addrs.contains(ptr))
                &&& (tier == SlabSize::Slab256 ==> pre.slab_256_bytes@.allocated_addrs.contains(ptr))
                &&& (tier == SlabSize::Slab512 ==> pre.slab_512_bytes@.allocated_addrs.contains(ptr))
            },
            Err(_) => {
                &&& post.slab_8_bytes@ == pre.slab_8_bytes@
                &&& post.slab_16_bytes@ == pre.slab_16_bytes@
                &&& post.slab_32_bytes@ == pre.slab_32_bytes@
                &&& post.slab_64_bytes@ == pre.slab_64_bytes@
                &&& post.slab_128_bytes@ == pre.slab_128_bytes@
                &&& post.slab_256_bytes@ == pre.slab_256_bytes@
                &&& post.slab_512_bytes@ == pre.slab_512_bytes@
                &&& post.slab_8_bytes.inv()
                &&& post.slab_16_bytes.inv()
                &&& post.slab_32_bytes.inv()
                &&& post.slab_64_bytes.inv()
                &&& post.slab_128_bytes.inv()
                &&& post.slab_256_bytes.inv()
                &&& post.slab_512_bytes.inv()
                &&& post.alloc_map@ == pre.alloc_map@
                &&& (tier == SlabSize::Slab8 ==> !pre.slab_8_bytes@.allocated_addrs.contains(ptr))
                &&& (tier == SlabSize::Slab16 ==> !pre.slab_16_bytes@.allocated_addrs.contains(ptr))
                &&& (tier == SlabSize::Slab32 ==> !pre.slab_32_bytes@.allocated_addrs.contains(ptr))
                &&& (tier == SlabSize::Slab64 ==> !pre.slab_64_bytes@.allocated_addrs.contains(ptr))
                &&& (tier == SlabSize::Slab128 ==> !pre.slab_128_bytes@.allocated_addrs.contains(ptr))
                &&& (tier == SlabSize::Slab256 ==> !pre.slab_256_bytes@.allocated_addrs.contains(ptr))
                &&& (tier == SlabSize::Slab512 ==> !pre.slab_512_bytes@.allocated_addrs.contains(ptr))
            },
        },
    ensures
        post.inv(),
        match result {
            Ok(()) => {
                &&& pre.alloc_map@.dom().contains(ptr as int)
                &&& post@ == pre@.spec_deallocate(ptr as int)
            },
            Err(_) => {
                &&& post@ == pre@
                &&& (!pre.alloc_map@.dom().contains(ptr as int)
                    || pre.alloc_map@[ptr as int] != size as nat)
            },
        },
{
    lemma_tier_size_bounds(tier, size);
    match result {
        Ok(()) => {
            Kheap::lemma_dealloc_preserves_internal_inv(pre, post, tier, ptr);
            Kheap::lemma_dealloc_preserves_view_inv(pre, post, ptr);
        },
        Err(_) => {
            Kheap::lemma_dealloc_err_preserves_inv(pre, post);
            Kheap::lemma_dealloc_err_reason(pre, tier, ptr, size);
        },
    }
}

proof fn lemma_dealloc_preserves_internal_inv(
    pre: &Kheap,
    post: &Kheap,
    tier: SlabSize,
    ptr: usize,
)
    requires
        pre.inv(),
        post.slab_8_bytes.inv(),
        post.slab_16_bytes.inv(),
        post.slab_32_bytes.inv(),
        post.slab_64_bytes.inv(),
        post.slab_128_bytes.inv(),
        post.slab_256_bytes.inv(),
        post.slab_512_bytes.inv(),
        post.slab_8_bytes@.block_size == 8,
        post.slab_16_bytes@.block_size == 16,
        post.slab_32_bytes@.block_size == 32,
        post.slab_64_bytes@.block_size == 64,
        post.slab_128_bytes@.block_size == 128,
        post.slab_256_bytes@.block_size == 256,
        post.slab_512_bytes@.block_size == 512,
        post.slab_8_bytes@.start_addr == pre.slab_8_bytes@.start_addr,
        post.slab_8_bytes@.end_addr == pre.slab_8_bytes@.end_addr,
        post.slab_16_bytes@.start_addr == pre.slab_16_bytes@.start_addr,
        post.slab_16_bytes@.end_addr == pre.slab_16_bytes@.end_addr,
        post.slab_32_bytes@.start_addr == pre.slab_32_bytes@.start_addr,
        post.slab_32_bytes@.end_addr == pre.slab_32_bytes@.end_addr,
        post.slab_64_bytes@.start_addr == pre.slab_64_bytes@.start_addr,
        post.slab_64_bytes@.end_addr == pre.slab_64_bytes@.end_addr,
        post.slab_128_bytes@.start_addr == pre.slab_128_bytes@.start_addr,
        post.slab_128_bytes@.end_addr == pre.slab_128_bytes@.end_addr,
        post.slab_256_bytes@.start_addr == pre.slab_256_bytes@.start_addr,
        post.slab_256_bytes@.end_addr == pre.slab_256_bytes@.end_addr,
        post.slab_512_bytes@.start_addr == pre.slab_512_bytes@.start_addr,
        post.slab_512_bytes@.end_addr == pre.slab_512_bytes@.end_addr,
        post.slab_8_bytes@.allocated_addrs == if tier == SlabSize::Slab8 {
            pre.slab_8_bytes@.allocated_addrs.remove(ptr)
        } else { pre.slab_8_bytes@.allocated_addrs },
        post.slab_16_bytes@.allocated_addrs == if tier == SlabSize::Slab16 {
            pre.slab_16_bytes@.allocated_addrs.remove(ptr)
        } else { pre.slab_16_bytes@.allocated_addrs },
        post.slab_32_bytes@.allocated_addrs == if tier == SlabSize::Slab32 {
            pre.slab_32_bytes@.allocated_addrs.remove(ptr)
        } else { pre.slab_32_bytes@.allocated_addrs },
        post.slab_64_bytes@.allocated_addrs == if tier == SlabSize::Slab64 {
            pre.slab_64_bytes@.allocated_addrs.remove(ptr)
        } else { pre.slab_64_bytes@.allocated_addrs },
        post.slab_128_bytes@.allocated_addrs == if tier == SlabSize::Slab128 {
            pre.slab_128_bytes@.allocated_addrs.remove(ptr)
        } else { pre.slab_128_bytes@.allocated_addrs },
        post.slab_256_bytes@.allocated_addrs == if tier == SlabSize::Slab256 {
            pre.slab_256_bytes@.allocated_addrs.remove(ptr)
        } else { pre.slab_256_bytes@.allocated_addrs },
        post.slab_512_bytes@.allocated_addrs == if tier == SlabSize::Slab512 {
            pre.slab_512_bytes@.allocated_addrs.remove(ptr)
        } else { pre.slab_512_bytes@.allocated_addrs },
        post.alloc_map@ == pre.alloc_map@.remove(ptr as int),
        pre.alloc_map@.dom().contains(ptr as int),
        // ptr was in the chosen tier's slab before deallocation.
        (tier == SlabSize::Slab8 ==> pre.slab_8_bytes@.allocated_addrs.contains(ptr))
            && (tier == SlabSize::Slab16 ==> pre.slab_16_bytes@.allocated_addrs.contains(ptr))
            && (tier == SlabSize::Slab32 ==> pre.slab_32_bytes@.allocated_addrs.contains(ptr))
            && (tier == SlabSize::Slab64 ==> pre.slab_64_bytes@.allocated_addrs.contains(ptr))
            && (tier == SlabSize::Slab128 ==> pre.slab_128_bytes@.allocated_addrs.contains(ptr))
            && (tier == SlabSize::Slab256 ==> pre.slab_256_bytes@.allocated_addrs.contains(ptr))
            && (tier == SlabSize::Slab512 ==> pre.slab_512_bytes@.allocated_addrs.contains(ptr)),
    ensures
        post.internal_inv(),
{
    assert(pre.internal_inv());
    // Forward direction: every addr in post.alloc_map is in some slab.
    assert forall|addr: int| #[trigger] post.alloc_map@.dom().contains(addr)
        implies ({
            let sz = post.alloc_map@[addr];
            let a = addr as usize;
            &&& 0 < addr <= usize::MAX
            &&& ({
                    ||| (1 <= sz <= 8 && post.slab_8_bytes@.allocated_addrs.contains(a))
                    ||| (9 <= sz <= 16 && post.slab_16_bytes@.allocated_addrs.contains(a))
                    ||| (17 <= sz <= 32 && post.slab_32_bytes@.allocated_addrs.contains(a))
                    ||| (33 <= sz <= 64 && post.slab_64_bytes@.allocated_addrs.contains(a))
                    ||| (65 <= sz <= 128 && post.slab_128_bytes@.allocated_addrs.contains(a))
                    ||| (129 <= sz <= 256 && post.slab_256_bytes@.allocated_addrs.contains(a))
                    ||| (257 <= sz <= 512 && post.slab_512_bytes@.allocated_addrs.contains(a))
                })
        })
    by {
        assert(addr != ptr as int);
        assert(pre.alloc_map@.dom().contains(addr));
        assert(pre.alloc_map@[addr] == post.alloc_map@[addr]);
    };
    // Reverse direction for each tier.
    assert forall|a: usize| #[trigger] post.slab_8_bytes@.allocated_addrs.contains(a)
        implies post.alloc_map@.dom().contains(a as int)
            && 1 <= post.alloc_map@[a as int] <= 8
    by {
        assert(pre.slab_8_bytes@.allocated_addrs.contains(a));
        assert(a as int != ptr as int);
    };
    assert forall|a: usize| #[trigger] post.slab_16_bytes@.allocated_addrs.contains(a)
        implies post.alloc_map@.dom().contains(a as int)
            && 9 <= post.alloc_map@[a as int] <= 16
    by {
        assert(pre.slab_16_bytes@.allocated_addrs.contains(a));
        assert(a as int != ptr as int);
    };
    assert forall|a: usize| #[trigger] post.slab_32_bytes@.allocated_addrs.contains(a)
        implies post.alloc_map@.dom().contains(a as int)
            && 17 <= post.alloc_map@[a as int] <= 32
    by {
        assert(pre.slab_32_bytes@.allocated_addrs.contains(a));
        assert(a as int != ptr as int);
    };
    assert forall|a: usize| #[trigger] post.slab_64_bytes@.allocated_addrs.contains(a)
        implies post.alloc_map@.dom().contains(a as int)
            && 33 <= post.alloc_map@[a as int] <= 64
    by {
        assert(pre.slab_64_bytes@.allocated_addrs.contains(a));
        assert(a as int != ptr as int);
    };
    assert forall|a: usize| #[trigger] post.slab_128_bytes@.allocated_addrs.contains(a)
        implies post.alloc_map@.dom().contains(a as int)
            && 65 <= post.alloc_map@[a as int] <= 128
    by {
        assert(pre.slab_128_bytes@.allocated_addrs.contains(a));
        assert(a as int != ptr as int);
    };
    assert forall|a: usize| #[trigger] post.slab_256_bytes@.allocated_addrs.contains(a)
        implies post.alloc_map@.dom().contains(a as int)
            && 129 <= post.alloc_map@[a as int] <= 256
    by {
        assert(pre.slab_256_bytes@.allocated_addrs.contains(a));
        assert(a as int != ptr as int);
    };
    assert forall|a: usize| #[trigger] post.slab_512_bytes@.allocated_addrs.contains(a)
        implies post.alloc_map@.dom().contains(a as int)
            && 257 <= post.alloc_map@[a as int] <= 512
    by {
        assert(pre.slab_512_bytes@.allocated_addrs.contains(a));
        assert(a as int != ptr as int);
    };
}

proof fn lemma_dealloc_preserves_view_inv(pre: &Kheap, post: &Kheap, ptr: usize)
    requires
        pre@.inv(),
        post.alloc_map@ == pre.alloc_map@.remove(ptr as int),
    ensures
        post@.inv(),
        post@ == pre@.spec_deallocate(ptr as int),
{
    assert(post@ =~= pre@.spec_deallocate(ptr as int));
}

/// Error-path for deallocate.
proof fn lemma_dealloc_err_preserves_inv(pre: &Kheap, post: &Kheap)
    requires
        pre.inv(),
        post.slab_8_bytes@ == pre.slab_8_bytes@,
        post.slab_16_bytes@ == pre.slab_16_bytes@,
        post.slab_32_bytes@ == pre.slab_32_bytes@,
        post.slab_64_bytes@ == pre.slab_64_bytes@,
        post.slab_128_bytes@ == pre.slab_128_bytes@,
        post.slab_256_bytes@ == pre.slab_256_bytes@,
        post.slab_512_bytes@ == pre.slab_512_bytes@,
        post.slab_8_bytes.inv(),
        post.slab_16_bytes.inv(),
        post.slab_32_bytes.inv(),
        post.slab_64_bytes.inv(),
        post.slab_128_bytes.inv(),
        post.slab_256_bytes.inv(),
        post.slab_512_bytes.inv(),
        post.alloc_map@ == pre.alloc_map@,
    ensures
        post.inv(),
        post@ == pre@,
{
    assert(post.internal_inv()) by {
        assert(pre.internal_inv());
    }
}

/// When slab.deallocate fails for a given tier, either ptr is not in
/// alloc_map, or the stored layout_size differs from the current one.
///
/// Proof sketch: if ptr IS in alloc_map with stored size == current size,
/// then both map to the same tier (layout_to_allocator is deterministic
/// on size). The forward internal_inv places ptr in that tier's
/// allocated_addrs. But slab.deallocate only fails when ptr is NOT in
/// allocated_addrs — contradiction.
proof fn lemma_dealloc_err_reason(pre: &Kheap, tier: SlabSize, ptr: usize, size: usize)
    requires
        pre.inv(),
        // size routes to tier
        (tier == SlabSize::Slab8 && 1 <= size <= 8)
            || (tier == SlabSize::Slab16 && 9 <= size <= 16)
            || (tier == SlabSize::Slab32 && 17 <= size <= 32)
            || (tier == SlabSize::Slab64 && 33 <= size <= 64)
            || (tier == SlabSize::Slab128 && 65 <= size <= 128)
            || (tier == SlabSize::Slab256 && 129 <= size <= 256)
            || (tier == SlabSize::Slab512 && 257 <= size <= 512),
        // The chosen tier's slab rejected ptr (ptr not in allocated_addrs,
        // or ptr out of bounds — in either case, ptr is not currently
        // allocated in this slab).
        (tier == SlabSize::Slab8 ==> !pre.slab_8_bytes@.allocated_addrs.contains(ptr))
            && (tier == SlabSize::Slab16 ==> !pre.slab_16_bytes@.allocated_addrs.contains(ptr))
            && (tier == SlabSize::Slab32 ==> !pre.slab_32_bytes@.allocated_addrs.contains(ptr))
            && (tier == SlabSize::Slab64 ==> !pre.slab_64_bytes@.allocated_addrs.contains(ptr))
            && (tier == SlabSize::Slab128 ==> !pre.slab_128_bytes@.allocated_addrs.contains(ptr))
            && (tier == SlabSize::Slab256 ==> !pre.slab_256_bytes@.allocated_addrs.contains(ptr))
            && (tier == SlabSize::Slab512 ==> !pre.slab_512_bytes@.allocated_addrs.contains(ptr)),
    ensures
        !pre.alloc_map@.dom().contains(ptr as int)
            || pre.alloc_map@[ptr as int] != size as nat,
{
    // By contradiction: assume ptr is in alloc_map with stored value == size.
    // Then internal_inv forward says ptr is in the slab for size's tier band.
    // But size maps to `tier`, and we know ptr is NOT in tier's allocated_addrs.
    // So the stored value must differ from size.
    if pre.alloc_map@.dom().contains(ptr as int) && pre.alloc_map@[ptr as int] == size as nat {
        assert(pre.internal_inv());
        let stored = pre.alloc_map@[ptr as int];
        // stored == size, so stored falls in the same tier band as size.
        // internal_inv forward: ptr is in that tier's allocated_addrs.
        // But our precondition says it's NOT. Contradiction.
        assert(false);
    }
}

} // impl Kheap

/// When layout_to_allocator returns Err (size == 0 or size > MAX_SLAB_SIZE),
/// either ptr is not in alloc_map, or the stored value differs from size.
proof fn lemma_dealloc_layout_to_allocator_err(kheap: &Kheap, ptr: usize, size: usize)
    requires
        kheap.inv(),
        size == 0 || size > MAX_SLAB_SIZE,
    ensures
        !kheap.alloc_map@.dom().contains(ptr as int)
            || kheap.alloc_map@[ptr as int] != size as nat,
{
    if kheap.alloc_map@.dom().contains(ptr as int) {
        assert(kheap.internal_inv());
        let stored = kheap.alloc_map@[ptr as int];
        assert(1 <= stored <= MAX_SLAB_SIZE);
        assert(stored != size as nat);
    }
}

/// Proves all preconditions for pointer .add() calls in from_raw_parts.
proof fn lemma_from_raw_parts_ptr_preconditions(addr: usize, size: usize, slab_size: usize, heap_start_addr: *mut u8)
    requires
        addr + size <= usize::MAX,
        size <= isize::MAX as usize,
        addr > 0,
        size >= MIN_HEAP_SIZE,
        size as int % MIN_HEAP_SIZE as int == 0,
        slab_size == size / NUM_OF_SLABS,
        heap_start_addr as usize == addr,
    ensures
        slab_size * NUM_OF_SLABS == size,
        ::core::mem::size_of::<u8>() == 1usize,
        slab_size <= isize::MAX as usize,
        6 * slab_size <= isize::MAX as usize,
        heap_start_addr as usize + slab_size <= usize::MAX,
        heap_start_addr as usize + 2 * slab_size <= usize::MAX,
        heap_start_addr as usize + 3 * slab_size <= usize::MAX,
        heap_start_addr as usize + 4 * slab_size <= usize::MAX,
        heap_start_addr as usize + 5 * slab_size <= usize::MAX,
        heap_start_addr as usize + 6 * slab_size <= usize::MAX,
        heap_start_addr as usize + 7 * slab_size <= usize::MAX,
{
    assert(MIN_HEAP_SIZE == NUM_OF_SLABS * MIN_SLAB_SIZE);
    assert(size as int % MIN_HEAP_SIZE as int == 0);
    assert(size as int % NUM_OF_SLABS as int == 0) by (nonlinear_arith)
        requires
            MIN_HEAP_SIZE == NUM_OF_SLABS * MIN_SLAB_SIZE,
            size as int % MIN_HEAP_SIZE as int == 0,
            NUM_OF_SLABS > 0,
            MIN_SLAB_SIZE > 0;
    assert(slab_size * NUM_OF_SLABS == size) by (nonlinear_arith)
        requires
            slab_size == size / NUM_OF_SLABS,
            size as int % NUM_OF_SLABS as int == 0,
            NUM_OF_SLABS > 0;
    Kheap::lemma_tier_offset_bound(addr, size, slab_size, 1);
    Kheap::lemma_tier_offset_bound(addr, size, slab_size, 2);
    Kheap::lemma_tier_offset_bound(addr, size, slab_size, 3);
    Kheap::lemma_tier_offset_bound(addr, size, slab_size, 4);
    Kheap::lemma_tier_offset_bound(addr, size, slab_size, 5);
    Kheap::lemma_tier_offset_bound(addr, size, slab_size, 6);
    Kheap::lemma_tier_offset_bound(addr, size, slab_size, 7);
    assert(slab_size <= isize::MAX as usize) by (nonlinear_arith)
        requires
            slab_size == size / NUM_OF_SLABS,
            size <= isize::MAX as usize,
            NUM_OF_SLABS > 0;
    assert(6 * slab_size <= isize::MAX as usize) by (nonlinear_arith)
        requires
            slab_size == size / NUM_OF_SLABS,
            size <= isize::MAX as usize,
            NUM_OF_SLABS == 7,
            size as int % NUM_OF_SLABS as int == 0;
}

/// Whether `t` is one of the supported slab tier sizes.
spec fn is_supported_tier(t: usize) -> bool {
    t == 8 || t == 16 || t == 32 || t == 64 || t == 128 || t == 256 || t == 512
}

/// Derives per-tier size bounds from the declarative spec of layout_to_allocator.
proof fn lemma_tier_size_bounds(tier: SlabSize, size: usize)
    requires
        size >= 1,
        is_supported_tier(tier as usize),
        tier as usize >= size,
        forall|s: usize| is_supported_tier(s) && s >= size ==> tier as usize <= s,
    ensures
        (tier == SlabSize::Slab8 ==> 1 <= size <= 8),
        (tier == SlabSize::Slab16 ==> size > 8 && size <= 16),
        (tier == SlabSize::Slab32 ==> size > 16 && size <= 32),
        (tier == SlabSize::Slab64 ==> size > 32 && size <= 64),
        (tier == SlabSize::Slab128 ==> size > 64 && size <= 128),
        (tier == SlabSize::Slab256 ==> size > 128 && size <= 256),
        (tier == SlabSize::Slab512 ==> size > 256 && size <= 512),
{
    assert(is_supported_tier(8usize));
    assert(is_supported_tier(16usize));
    assert(is_supported_tier(32usize));
    assert(is_supported_tier(64usize));
    assert(is_supported_tier(128usize));
    assert(is_supported_tier(256usize));
}

/// Proves the minimality clause for layout_to_allocator's postcondition.
proof fn lemma_layout_to_allocator_minimality(tier: SlabSize, layout_size: usize)
    requires
        is_supported_tier(tier as usize),
        tier as usize >= layout_size,
        (tier == SlabSize::Slab8 ==> 1 <= layout_size <= 8),
        (tier == SlabSize::Slab16 ==> layout_size > 8 && layout_size <= 16),
        (tier == SlabSize::Slab32 ==> layout_size > 16 && layout_size <= 32),
        (tier == SlabSize::Slab64 ==> layout_size > 32 && layout_size <= 64),
        (tier == SlabSize::Slab128 ==> layout_size > 64 && layout_size <= 128),
        (tier == SlabSize::Slab256 ==> layout_size > 128 && layout_size <= 256),
        (tier == SlabSize::Slab512 ==> layout_size > 256 && layout_size <= 512),
    ensures
        forall|s: usize| is_supported_tier(s) && s >= layout_size ==> tier as usize <= s,
{
    assert forall|s: usize| is_supported_tier(s) && s >= layout_size implies tier as usize <= s by {}
}

} // verus!
