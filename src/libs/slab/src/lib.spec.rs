// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

// Specifications.

verus! {

/// A view of the Slab as an abstract specification.
#[verifier::ext_equal]
pub struct SlabView {
    /// Set of allocated block indices (relative to data blocks).
    pub allocated_blocks: Set<int>,
    /// Total number of data blocks.
    pub num_data_blocks: int,
    /// Block size in bytes.
    pub block_size: int,
    /// Base address of data region.
    pub data_addr: int,
}

impl SlabView {
    /// Returns the number of allocated blocks.
    pub open spec fn used(&self) -> int {
        self.allocated_blocks.len() as int
    }

    /// Returns the capacity (total number of data blocks).
    pub open spec fn capacity(&self) -> int {
        self.num_data_blocks
    }

    /// Returns the number of free blocks.
    pub open spec fn free(&self) -> int {
        self.capacity() - self.used()
    }

    /// Returns true if block at given index is allocated.
    pub open spec fn is_allocated(&self, block_idx: int) -> bool {
        self.allocated_blocks.contains(block_idx)
    }

    /// Returns true if the slab is full.
    pub open spec fn is_full(&self) -> bool {
        self.used() == self.num_data_blocks
    }

    /// Returns true if the slab is empty.
    pub open spec fn is_empty(&self) -> bool {
        self.allocated_blocks.len() == 0
    }

    /// Returns the address of a block given its index.
    pub open spec fn block_addr(&self, block_idx: int) -> int {
        self.data_addr + block_idx * self.block_size
    }

    /// Returns the block index for a given address.
    pub open spec fn addr_to_block_idx(&self, addr: int) -> int {
        (addr - self.data_addr) / self.block_size
    }

    /// Returns true if the data address is aligned to block size.
    pub open spec fn is_aligned(&self) -> bool {
        self.data_addr % self.block_size == 0
    }

    /// Returns true if the address is valid for this slab.
    pub open spec fn is_valid_addr(&self, addr: int) -> bool {
        &&& addr >= self.data_addr
        &&& addr < self.data_addr + self.num_data_blocks * self.block_size
        &&& (addr - self.data_addr) % self.block_size == 0
    }

    //==============================================================================================
    // High-Level Memory Management Properties
    //==============================================================================================
    /// Property: All allocated block indices are within valid range.
    pub open spec fn allocated_blocks_in_range(&self) -> bool {
        forall|i: int|
            #![trigger self.is_allocated(i)]
            self.is_allocated(i) ==> (0 <= i < self.num_data_blocks)
    }

    /// Property: Memory regions of different blocks are disjoint.
    /// Two blocks with different indices have non-overlapping memory regions.
    pub open spec fn blocks_are_disjoint(&self, i: int, j: int) -> bool
        recommends
            0 <= i < self.num_data_blocks,
            0 <= j < self.num_data_blocks,
            i != j,
    {
        let addr_i = self.block_addr(i);
        let addr_j = self.block_addr(j);
        // Block i's region [addr_i, addr_i + block_size) does not overlap with block j's region.
        addr_i + self.block_size <= addr_j || addr_j + self.block_size <= addr_i
    }

    /// Property: All allocated blocks have disjoint memory regions (no aliasing).
    pub open spec fn no_memory_aliasing(&self) -> bool {
        forall|i: int, j: int|
            #![trigger self.is_allocated(i), self.is_allocated(j)]
            (self.is_allocated(i) && self.is_allocated(j) && i != j) ==> self.blocks_are_disjoint(
                i,
                j,
            )
    }

    /// Property: Inverse relationship - addr_to_block_idx(block_addr(i)) == i for valid i.
    pub open spec fn addr_block_idx_inverse(&self, i: int) -> bool
        recommends
            0 <= i < self.num_data_blocks,
            self.block_size > 0,
    {
        self.addr_to_block_idx(self.block_addr(i)) == i
    }

    /// Property: Inverse relationship - block_addr(addr_to_block_idx(a)) == a for valid addresses.
    pub open spec fn block_addr_inverse(&self, addr: int) -> bool
        recommends
            self.is_valid_addr(addr),
            self.block_size > 0,
    {
        self.block_addr(self.addr_to_block_idx(addr)) == addr
    }

    //==============================================================================================
    // Liveness Properties
    //==============================================================================================
    /// Property (Liveness): If there's free capacity, allocation can succeed.
    pub open spec fn can_allocate(&self) -> bool {
        self.free() > 0
    }

    /// Property (Liveness): If a block is allocated, it can be deallocated.
    pub open spec fn can_deallocate(&self, block_idx: int) -> bool {
        self.is_allocated(block_idx) && 0 <= block_idx < self.num_data_blocks
    }

    /// Property (Liveness): After deallocation, allocation becomes possible.
    pub open spec fn dealloc_enables_alloc(&self, freed_view: &SlabView) -> bool
        recommends
            self.used() == self.capacity(),
    {
        freed_view.used() < freed_view.capacity()
    }

    //==============================================================================================
    // Memory Initialization Properties
    //==============================================================================================
    /// Property: A freshly initialized slab has no allocated data blocks.
    pub open spec fn is_freshly_initialized(&self) -> bool {
        self.allocated_blocks =~= Set::<int>::empty()
    }

    //==============================================================================================
    // PointsToRaw Memory Permission Properties
    //==============================================================================================
    /// Returns the memory domain (set of addresses) for a given block index.
    pub open spec fn block_dom(&self, block_idx: int) -> Set<int> {
        set_int_range(self.block_addr(block_idx), self.block_addr(block_idx) + self.block_size)
    }

    /// Returns the memory domain for the entire data region.
    pub open spec fn data_region_dom(&self) -> Set<int> {
        set_int_range(self.data_addr, self.data_addr + self.num_data_blocks * self.block_size)
    }

    /// Returns the union of all free block domains.
    pub open spec fn free_region_dom(&self) -> Set<int> {
        Set::new(
            |addr: int|
                exists|i: int|
                    #![trigger self.block_dom(i)]
                    0 <= i < self.num_data_blocks && !self.is_allocated(i) && self.block_dom(
                        i,
                    ).contains(addr),
        )
    }

    /// Property: A permission map is well-formed for this slab view.
    /// Every free block i has a permission in the map covering [block_addr(i), block_addr(i) + block_size).
    /// Every allocated block has no permission in the map.
    /// The map domain is exactly the set of free block indices.
    pub open spec fn perms_wf(&self, perms: Map<int, PointsToRaw>, prov: Provenance) -> bool {
        &&& forall|i: int|
            #![trigger perms.dom().contains(i)]
            (0 <= i < self.num_data_blocks && !self.is_allocated(i)) ==> perms.dom().contains(i)
                && (#[trigger] perms[i]).is_range(self.block_addr(i), self.block_size)
                && perms[i].provenance() == prov
        &&& forall|i: int|
            #![trigger perms.dom().contains(i)]
            (0 <= i < self.num_data_blocks && self.is_allocated(i)) ==> !perms.dom().contains(
                i,
            )
            // M1: domain is exactly the free block set (no extra entries).
        &&& forall|i: int|
            #![trigger perms.dom().contains(i)]
            perms.dom().contains(i) ==> (0 <= i < self.num_data_blocks && !self.is_allocated(i))
    }
    //==============================================================================================

}

//==================================================================================================
// Tracked Memory Permissions
//==================================================================================================
/// Tracked proof state for slab memory ownership.
/// This struct is erased at compile time — it exists only for verification.
/// It tracks per-block PointsToRaw permissions for the free data blocks
/// and the index region permission (trust boundary with RawArray).
pub tracked struct SlabPerms {
    /// Per-block permissions for free data blocks.
    pub free_perms: Map<int, PointsToRaw>,
    /// Permission for the index region.
    pub index_perm: PointsToRaw,
}

impl SlabPerms {
    /// Well-formedness predicate linking permissions to a slab view.
    /// The `index_perm` covers the index region (its range is established by `from_raw_parts`
    /// and preserved by `take_block_perm`/`put_block_perm` which guarantee `index_perm` invariance).
    pub open spec fn wf(&self, view: SlabView, prov: Provenance) -> bool {
        &&& view.perms_wf(self.free_perms, prov)
        &&& self.index_perm.provenance() == prov
    }

    /// Removes a block's permission from free_perms and returns it.
    pub proof fn take_block_perm(tracked &mut self, block_idx: int) -> (tracked perm: PointsToRaw)
        requires
            old(self).free_perms.dom().contains(block_idx),
        ensures
            perm == old(self).free_perms[block_idx],
            self.free_perms == old(self).free_perms.remove(block_idx),
            self.index_perm == old(self).index_perm,
    {
        self.free_perms.tracked_remove(block_idx)
    }

    /// Inserts a block's permission back into free_perms.
    pub proof fn put_block_perm(tracked &mut self, block_idx: int, tracked perm: PointsToRaw)
        ensures
            self.free_perms == old(self).free_perms.insert(block_idx, perm),
            self.index_perm == old(self).index_perm,
    {
        self.free_perms.tracked_insert(block_idx, perm);
    }
}

} // verus!
