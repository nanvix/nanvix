verus! {

//==================================================================================================
// Helper Lemmas
//==================================================================================================

/// Helper: regions are ordered across non-consecutive slabs (by transitivity).
proof fn lemma_regions_ordered(kv: &KheapView, i: int, j: int)
    requires
        kv.inv(),
        0 <= i < j < kv.slabs.len(),
    ensures
        kv.slabs[i].end_addr <= kv.slabs[j].start_addr,
    decreases j - i,
{
    if j == i + 1 {
        // Direct from kv.inv(): consecutive pair
    } else {
        lemma_regions_ordered(kv, i, j - 1);
        // IH: kv.slabs[i].end_addr <= kv.slabs[j-1].start_addr
        // kv.slabs[j-1].inv(): start_addr < end_addr
        // kv.inv() consecutive: kv.slabs[j-1].end_addr <= kv.slabs[j].start_addr
        // Chain: slabs[i].end_addr <= slabs[j-1].start_addr < slabs[j-1].end_addr <= slabs[j].start_addr
    }
}

//==================================================================================================
// Proof Bodies
//==================================================================================================

/// MOD-3: Cross-slab disjointness follows from TYPE-2 (region disjointness)
/// and SlabView::inv() (addresses lie within [start_addr, end_addr)).
proof fn lemma_kheap_inv_implies_cross_slab_disjointness(kv: &KheapView)
    requires kv.inv(),
    ensures
        // MOD-1: allocated sets disjoint across slabs
        forall|i: int, j: int| 0 <= i < j < kv.slabs.len() ==>
            kv.slabs[i].allocated_addrs.disjoint(kv.slabs[j].allocated_addrs),
        // MOD-2: free sets disjoint across slabs
        forall|i: int, j: int| 0 <= i < j < kv.slabs.len() ==>
            kv.slabs[i].free_addrs.disjoint(kv.slabs[j].free_addrs),
        // MOD-3 (full): allocated/free cross-disjoint
        forall|i: int, j: int| 0 <= i < j < kv.slabs.len() ==>
            kv.slabs[i].allocated_addrs.disjoint(kv.slabs[j].free_addrs),
{
    assert forall|i: int, j: int| 0 <= i < j < kv.slabs.len() implies
        kv.slabs[i].allocated_addrs.disjoint(kv.slabs[j].allocated_addrs)
    by {
        lemma_regions_ordered(kv, i, j);
        // slabs[i].end_addr <= slabs[j].start_addr
        // Any addr in slab i: addr < slabs[i].end_addr
        // Any addr in slab j: addr >= slabs[j].start_addr
        // So no overlap
    };
    assert forall|i: int, j: int| 0 <= i < j < kv.slabs.len() implies
        kv.slabs[i].free_addrs.disjoint(kv.slabs[j].free_addrs)
    by {
        lemma_regions_ordered(kv, i, j);
    };
    assert forall|i: int, j: int| 0 <= i < j < kv.slabs.len() implies
        kv.slabs[i].allocated_addrs.disjoint(kv.slabs[j].free_addrs)
    by {
        lemma_regions_ordered(kv, i, j);
    };
}

/// spec_slab_for_size maps to a valid index with correct block size bound.
proof fn lemma_slab_for_size_valid(size: int)
    requires spec_slab_for_size(size).is_some(),
    ensures
        0 <= spec_slab_for_size(size).unwrap() < NUM_OF_SLABS as int,
        block_sizes()[spec_slab_for_size(size).unwrap()] >= size,
{
    // spec_slab_for_size and block_sizes are open specs — automatic
}

/// LIVE-5: Allocate-then-deallocate round-trip restores abstract state.
proof fn lemma_alloc_dealloc_round_trip(kv: KheapView, idx: int, addr: usize)
    requires
        kv.inv(),
        0 <= idx < kv.slabs.len(),
        kv.slabs[idx].free_addrs.contains(addr),
    ensures
        kv.spec_allocate(idx, addr).spec_deallocate(idx, addr) == kv,
{
    let slab = kv.slabs[idx];
    // addr is in free, so not in allocated (disjoint from SlabView::inv)
    assert(!slab.allocated_addrs.contains(addr));

    let after_alloc = kv.spec_allocate(idx, addr);
    let after_dealloc = after_alloc.spec_deallocate(idx, addr);

    // Show the slab at idx is restored
    assert(slab.allocated_addrs.insert(addr).remove(addr) =~= slab.allocated_addrs);
    assert(slab.free_addrs.remove(addr).insert(addr) =~= slab.free_addrs);

    // Show slabs sequence is restored
    assert(after_dealloc.slabs =~= kv.slabs);
    assert(after_dealloc =~= kv);
}

/// MOD-5: Allocation conservation — union of allocated+free is preserved
/// across spec_allocate.
proof fn lemma_allocate_conserves(kv: KheapView, idx: int, addr: usize)
    requires
        kv.inv(),
        0 <= idx < kv.slabs.len(),
        kv.slabs[idx].free_addrs.contains(addr),
    ensures
        forall|j: int| 0 <= j < kv.slabs.len() ==>
            (#[trigger] kv.slabs[j]).allocated_addrs.union(kv.slabs[j].free_addrs)
                == kv.spec_allocate(idx, addr).slabs[j].allocated_addrs.union(
                       kv.spec_allocate(idx, addr).slabs[j].free_addrs),
{
    let new_kv = kv.spec_allocate(idx, addr);
    assert forall|j: int| 0 <= j < kv.slabs.len() implies
        (#[trigger] kv.slabs[j]).allocated_addrs.union(kv.slabs[j].free_addrs)
            == new_kv.slabs[j].allocated_addrs.union(new_kv.slabs[j].free_addrs)
    by {
        if j == idx {
            let old_slab = kv.slabs[idx];
            assert(!old_slab.allocated_addrs.contains(addr));
            // allocated.insert(addr) ∪ free.remove(addr) =~= allocated ∪ free
            assert(old_slab.allocated_addrs.insert(addr).union(old_slab.free_addrs.remove(addr))
                =~= old_slab.allocated_addrs.union(old_slab.free_addrs));
        } else {
            // Unchanged
        }
    };
}

/// MOD-5: Deallocation conservation.
proof fn lemma_deallocate_conserves(kv: KheapView, idx: int, addr: usize)
    requires
        kv.inv(),
        0 <= idx < kv.slabs.len(),
        kv.slabs[idx].allocated_addrs.contains(addr),
    ensures
        forall|j: int| 0 <= j < kv.slabs.len() ==>
            (#[trigger] kv.slabs[j]).allocated_addrs.union(kv.slabs[j].free_addrs)
                == kv.spec_deallocate(idx, addr).slabs[j].allocated_addrs.union(
                       kv.spec_deallocate(idx, addr).slabs[j].free_addrs),
{
    let new_kv = kv.spec_deallocate(idx, addr);
    assert forall|j: int| 0 <= j < kv.slabs.len() implies
        (#[trigger] kv.slabs[j]).allocated_addrs.union(kv.slabs[j].free_addrs)
            == new_kv.slabs[j].allocated_addrs.union(new_kv.slabs[j].free_addrs)
    by {
        if j == idx {
            let old_slab = kv.slabs[idx];
            assert(!old_slab.free_addrs.contains(addr));
            assert(old_slab.allocated_addrs.remove(addr).union(old_slab.free_addrs.insert(addr))
                =~= old_slab.allocated_addrs.union(old_slab.free_addrs));
        } else {
            // Unchanged
        }
    };
}

//==================================================================================================
// Strengthening Lemmas
//==================================================================================================

/// FN-1c strengthened: spec_slab_for_size selects the tightest-fitting slab.
/// All smaller slab tiers have block sizes strictly less than the requested size.
proof fn lemma_slab_for_size_tightest_fit(size: int)
    requires spec_slab_for_size(size).is_some(),
    ensures ({
        let idx = spec_slab_for_size(size).unwrap();
        &&& (idx > 0 ==> block_sizes()[idx - 1] < size)
        &&& (idx > 1 ==> block_sizes()[idx - 2] < size)
        &&& block_sizes()[idx] >= size
    }),
{
}

/// TYPE-3 strengthened: block_sizes() is strictly monotonically increasing.
proof fn lemma_block_sizes_strictly_increasing()
    ensures
        forall|i: int| #![trigger block_sizes()[i]]
            0 <= i < (block_sizes().len() - 1) ==>
            block_sizes()[i] < block_sizes()[i + 1],
{
}

/// spec_slab_for_size is total over the supported range [1, max_slab_size()].
proof fn lemma_slab_for_size_total(size: int)
    requires 1 <= size <= max_slab_size(),
    ensures spec_slab_for_size(size).is_some(),
{
}

/// MOD-4: No allocation at address zero (conditional on base address).
/// If the heap was constructed from a non-zero base address, no slab
/// contains address 0 in either its allocated or free sets.
/// The base address is non-zero at runtime because HEAP_STORAGE is a
/// static with linker-assigned address > 0, but this is a runtime fact
/// that cannot be expressed as a Verus axiom.
proof fn lemma_no_null_address(kv: &KheapView, base_addr: int, slab_size: int)
    requires
        kv.inv(),
        base_addr > 0,
        slab_size > 0,
        forall|i: int| 0 <= i < kv.slabs.len() ==>
            (#[trigger] kv.slabs[i]).start_addr >= base_addr + i * slab_size,
    ensures
        forall|i: int| 0 <= i < kv.slabs.len() ==> {
            &&& !(#[trigger] kv.slabs[i]).allocated_addrs.contains(0usize)
            &&& !kv.slabs[i].free_addrs.contains(0usize)
        },
{
    assert forall|i: int| 0 <= i < kv.slabs.len() implies {
        &&& !(#[trigger] kv.slabs[i]).allocated_addrs.contains(0usize)
        &&& !kv.slabs[i].free_addrs.contains(0usize)
    } by {
        // start_addr >= base_addr + i * slab_size >= base_addr > 0
        // SlabView::inv: all addresses in [start_addr, end_addr), so >= start_addr > 0
        // Therefore 0 is not in any slab's address sets
        let slab = kv.slabs[i];
        assert(slab.start_addr >= base_addr + i * slab_size);
        assert(slab.start_addr > 0);
    };
}

/// LIVE-1 (conditional): For init()-standard parameters (size = MIN_HEAP_SIZE,
/// non-zero base), none of the Slab::from_raw_parts error conditions hold
/// for any slab index.
///
/// The Slab spec provides a bidirectional error clause:
///   Err(e) ==> (addr==0 || len==0 || len>=i32::MAX || len>isize::MAX
///               || addr+len>usize::MAX || block_size==0 || block_size>=i32::MAX
///               || block_size>(usize::MAX-1)/8 || len<block_size*2
///               || addr%block_size!=0)
/// By contrapositive: ¬(any error condition) ==> Ok.
///
/// Remaining architecture assumptions (requires parameters):
/// - base_addr > 0: HEAP_STORAGE is a static; linker guarantees non-zero address.
/// - usize::MAX >= 8 * max_slab_size() + 1: true on all ≥16-bit platforms.
/// - MIN_HEAP_SIZE <= isize::MAX: true on all ≥32-bit platforms.
proof fn lemma_slab_construction_feasible(
    base_addr: int,
    slab_idx: int,
)
    requires
        base_addr > 0,
        base_addr % PAGE_SIZE as int == 0,
        base_addr + MIN_HEAP_SIZE as int <= usize::MAX as int,
        MIN_HEAP_SIZE as int <= isize::MAX as int,
        0 <= slab_idx < NUM_OF_SLABS as int,
        usize::MAX as int >= 8 * max_slab_size() + 1,
    ensures ({
        let slab_size = MIN_SLAB_SIZE as int;
        let slab_addr = base_addr + slab_idx * slab_size;
        let block_size = block_sizes()[slab_idx];
        // Negation of ALL Slab::from_raw_parts error conditions:
        &&& slab_addr != 0
        &&& slab_size > 0
        &&& slab_size < i32::MAX as int
        &&& slab_size <= isize::MAX as int
        &&& slab_addr + slab_size <= usize::MAX as int
        &&& block_size > 0
        &&& block_size < i32::MAX as int
        &&& block_size <= (usize::MAX as int - 1) / 8
        &&& slab_size >= block_size * 2
        &&& slab_addr % block_size == 0
    }),
{
    let slab_size: int = MIN_SLAB_SIZE as int;
    let slab_addr: int = base_addr + slab_idx * slab_size;
    let block_size: int = block_sizes()[slab_idx];

    // slab_addr > 0: base_addr > 0, slab_idx >= 0, slab_size > 0
    assert(slab_addr > 0);

    // MIN_SLAB_SIZE = SLAB_COUNT * PAGE_SIZE = 32 * 4096 = 131072
    assert(slab_size > 0);
    assert(slab_size < i32::MAX as int);

    // slab_size <= isize::MAX: MIN_SLAB_SIZE <= MIN_HEAP_SIZE <= isize::MAX
    assert(MIN_SLAB_SIZE as int <= MIN_HEAP_SIZE as int);

    // slab_addr + slab_size <= usize::MAX:
    //   = base_addr + (slab_idx + 1) * slab_size
    //   <= base_addr + NUM_OF_SLABS * slab_size = base_addr + MIN_HEAP_SIZE
    //   <= usize::MAX
    assert(NUM_OF_SLABS as int * MIN_SLAB_SIZE as int == MIN_HEAP_SIZE as int);

    // block_size > 0 and < i32::MAX (max is max_slab_size() <= 4096)
    assert(block_size > 0);
    assert(max_slab_size() < i32::MAX as int);
    assert(block_size <= max_slab_size());

    // block_size <= (usize::MAX - 1) / 8:
    //   usize::MAX - 1 >= 8 * max_slab_size() >= 8 * block_size
    //   By div monotonicity: (usize::MAX-1)/8 >= (8*block_size)/8 = block_size
    assert(usize::MAX as int - 1 >= 8 * block_size);
    vstd::arithmetic::div_mod::lemma_div_is_ordered(
        8 * block_size,
        usize::MAX as int - 1,
        8,
    );
    vstd::arithmetic::div_mod::lemma_div_multiples_vanish(block_size, 8);

    // slab_size >= block_size * 2: 131072 >= 2 * max_slab_size()
    assert(slab_size >= max_slab_size() * 2);

    // slab_addr % block_size == 0 via modular transitivity:
    // (a) PAGE_SIZE % block_size == 0 — case-split on slab_idx for concrete block sizes
    #[cfg(not(feature = "hyperlight"))]
    {
        assert(PAGE_SIZE as int % block_size == 0) by {
            if slab_idx == 0 { }      // 4096 % 8 = 0
            else if slab_idx == 1 { }  // 4096 % 16 = 0
            else if slab_idx == 2 { }  // 4096 % 32 = 0
            else if slab_idx == 3 { }  // 4096 % 64 = 0
            else if slab_idx == 4 { }  // 4096 % 128 = 0
            else if slab_idx == 5 { }  // 4096 % 256 = 0
            else { }                   // 4096 % 512 = 0
        };
    }
    #[cfg(feature = "hyperlight")]
    {
        assert(PAGE_SIZE as int % block_size == 0) by {
            if slab_idx == 0 { }
            else if slab_idx == 1 { }
            else if slab_idx == 2 { }
            else if slab_idx == 3 { }
            else if slab_idx == 4 { }
            else if slab_idx == 5 { }
            else if slab_idx == 6 { }
            else if slab_idx == 7 { }
            else if slab_idx == 8 { }
            else { }
        };
    }

    // (b) base_addr % block_size == 0 (from base_addr % PAGE_SIZE == 0)
    //     PAGE_SIZE = block_size * (PAGE_SIZE / block_size)
    //     lemma_mod_mod: (base_addr % (block_size * b)) % block_size == base_addr % block_size
    let b = PAGE_SIZE as int / block_size;
    vstd::arithmetic::div_mod::lemma_fundamental_div_mod(PAGE_SIZE as int, block_size);
    assert(PAGE_SIZE as int == block_size * b);
    vstd::arithmetic::div_mod::lemma_mod_mod(base_addr, block_size, b);
    assert(base_addr % block_size == 0);

    // (c) (slab_idx * slab_size) % block_size == 0
    //     MIN_SLAB_SIZE = SLAB_COUNT * PAGE_SIZE, and we showed PAGE_SIZE % block_size == 0
    vstd::arithmetic::div_mod::lemma_mod_multiples_basic(SLAB_COUNT as int, PAGE_SIZE as int);
    assert((SLAB_COUNT as int * PAGE_SIZE as int) % PAGE_SIZE as int == 0);
    let b2 = PAGE_SIZE as int / block_size;
    vstd::arithmetic::div_mod::lemma_mod_mod(MIN_SLAB_SIZE as int, block_size, b2);
    assert(MIN_SLAB_SIZE as int % block_size == 0);
    vstd::arithmetic::div_mod::lemma_mul_mod_noop_right(slab_idx, slab_size, block_size);
    assert((slab_idx * slab_size) % block_size == 0);

    // (d) slab_addr = base_addr + slab_idx * slab_size, both 0 mod block_size
    vstd::arithmetic::div_mod::lemma_add_mod_noop(
        base_addr,
        slab_idx * slab_size,
        block_size,
    );
    assert(slab_addr % block_size == 0);
}

} // verus!
