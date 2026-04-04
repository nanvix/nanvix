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

} // verus!
