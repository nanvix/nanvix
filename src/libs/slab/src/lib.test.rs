// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

// Slab Allocator - Verified Tests.
// Verified test functions that prove key slab allocator properties.

verus! {

//==================================================================================================
// Verified Test Functions
//==================================================================================================

/// Common preconditions macro-like spec for slab test parameters.
/// All test functions share these requirements on addr, len, block_size, mem.

/// Verifiable test: from_raw_parts creates a valid slab with expected properties.
fn test_slab_from_raw_parts_verified(
    addr: *mut u8,
    len: usize,
    block_size: usize,
    Tracked(mem): Tracked<PointsToRaw>,
)
    requires
        len > 0,
        len < i32::MAX as usize,
        block_size > 0,
        block_size < i32::MAX as usize,
        block_size <= len,
        is_pow2(block_size as int),
        (addr as usize) % block_size == 0,
        addr as int > 0,
        (addr as int) + (len as int) <= (usize::MAX as int),
        (len / block_size) % (u8::BITS as usize) == 0,
        len / block_size >= 8,
        mem.is_range(addr as int, len as int),
{
    match unsafe { Slab::from_raw_parts(addr, len, block_size, Tracked(mem)) } {
        Ok(pair) => {
            let (slab, Tracked(slab_perms)) = pair;
            assert(slab.inv());
            assert(forall|i: int| 0 <= i < slab@.num_data_blocks ==> !slab@.is_allocated(i));
            assert(slab@.block_size == block_size as int);
            assert(slab@.data_addr % (block_size as int) == 0);
            assert(slab@.num_data_blocks > 0);
        },
        Err(_) => {},
    }
}

/// Verifiable test: from_raw_parts followed by allocate/deallocate works correctly.
fn test_slab_from_raw_parts_allocate_verified(
    addr: *mut u8,
    len: usize,
    block_size: usize,
    Tracked(mem): Tracked<PointsToRaw>,
)
    requires
        len > 0,
        len < i32::MAX as usize,
        block_size > 0,
        block_size < i32::MAX as usize,
        block_size <= len,
        is_pow2(block_size as int),
        (addr as usize) % block_size == 0,
        addr as int > 0,
        (addr as int) + (len as int) <= (usize::MAX as int),
        (len / block_size) % (u8::BITS as usize) == 0,
        len / block_size >= 8,
        mem.is_range(addr as int, len as int),
{
    match unsafe { Slab::from_raw_parts(addr, len, block_size, Tracked(mem)) } {
        Ok(pair) => {
            let (mut slab, Tracked(mut slab_perms)) = pair;
            assert(forall|i: int| 0 <= i < slab@.num_data_blocks ==> !slab@.is_allocated(i));

            match slab.allocate(Tracked(&mut slab_perms)) {
                Ok(alloc_pair) => {
                    let (alloc_addr, Tracked(block_perm)) = alloc_pair;
                    proof {
                        assert(slab@.is_valid_addr(alloc_addr as int));
                        let block_idx = slab@.addr_to_block_idx(alloc_addr as int);
                        assert(slab@.is_allocated(block_idx));
                        assert(block_perm.is_range(alloc_addr as int, slab@.block_size));
                    }
                    match unsafe {
                        slab.deallocate(alloc_addr, Tracked(block_perm), Tracked(&mut slab_perms))
                    } {
                        Ok(()) => {
                            proof {
                                let block_idx = slab@.addr_to_block_idx(alloc_addr as int);
                                assert(!slab@.is_allocated(block_idx));
                            }
                        },
                        Err(_) => {},
                    }
                },
                Err(_) => {},
            }
        },
        Err(_) => {},
    }
}

//==================================================================================================

/// Verifiable test: allocating a block and then deallocating it.
fn test_allocate_deallocate_verified(
    addr: *mut u8,
    len: usize,
    block_size: usize,
    Tracked(mem): Tracked<PointsToRaw>,
)
    requires
        len > 0,
        len < i32::MAX as usize,
        block_size > 0,
        block_size < i32::MAX as usize,
        block_size <= len,
        is_pow2(block_size as int),
        (addr as usize) % block_size == 0,
        addr as int > 0,
        (addr as int) + (len as int) <= (usize::MAX as int),
        (len / block_size) % (u8::BITS as usize) == 0,
        len / block_size >= 8,
        mem.is_range(addr as int, len as int),
{
    match unsafe { Slab::from_raw_parts(addr, len, block_size, Tracked(mem)) } {
        Ok(pair) => {
            let (mut slab, Tracked(mut slab_perms)) = pair;
            match slab.allocate(Tracked(&mut slab_perms)) {
                Ok(alloc_pair) => {
                    let (block_addr, Tracked(block_perm)) = alloc_pair;
                    proof {
                        let block_idx = slab@.addr_to_block_idx(block_addr as int);
                        assert(slab@.is_allocated(block_idx));
                    }
                    match unsafe {
                        slab.deallocate(block_addr, Tracked(block_perm), Tracked(&mut slab_perms))
                    } {
                        Ok(()) => {
                            proof {
                                let block_idx = slab@.addr_to_block_idx(block_addr as int);
                                assert(!slab@.is_allocated(block_idx));
                            }
                        },
                        Err(_) => {},
                    }
                },
                Err(_) => {},
            }
        },
        Err(_) => {},
    }
}

/// Verifiable test: double deallocation is prevented by linear permissions.
/// After deallocating, the caller no longer holds the PointsToRaw permission,
/// so a second deallocate call is impossible at the type level.
fn test_double_deallocate_verified(
    addr: *mut u8,
    len: usize,
    block_size: usize,
    Tracked(mem): Tracked<PointsToRaw>,
)
    requires
        len > 0,
        len < i32::MAX as usize,
        block_size > 0,
        block_size < i32::MAX as usize,
        block_size <= len,
        is_pow2(block_size as int),
        (addr as usize) % block_size == 0,
        addr as int > 0,
        (addr as int) + (len as int) <= (usize::MAX as int),
        (len / block_size) % (u8::BITS as usize) == 0,
        len / block_size >= 8,
        mem.is_range(addr as int, len as int),
{
    match unsafe { Slab::from_raw_parts(addr, len, block_size, Tracked(mem)) } {
        Ok(pair) => {
            let (mut slab, Tracked(mut slab_perms)) = pair;
            match slab.allocate(Tracked(&mut slab_perms)) {
                Ok(alloc_pair) => {
                    let (block_addr, Tracked(block_perm)) = alloc_pair;
                    // First deallocation consumes block_perm.
                    match unsafe {
                        slab.deallocate(block_addr, Tracked(block_perm), Tracked(&mut slab_perms))
                    } {
                        Ok(()) => {
                            proof {
                                let block_idx = slab@.addr_to_block_idx(block_addr as int);
                                assert(!slab@.is_allocated(block_idx));
                                // block_perm was consumed — a second deallocate is
                                // impossible because the caller has no permission to pass.
                            }
                        },
                        Err(_) => {},
                    }
                },
                Err(_) => {},
            }
        },
        Err(_) => {},
    }
}

/// Verifiable test: deallocating an out-of-bounds address would violate preconditions.
fn test_allocate_out_of_bounds_verified(
    addr: *mut u8,
    len: usize,
    block_size: usize,
    Tracked(mem): Tracked<PointsToRaw>,
)
    requires
        len > 0,
        len < i32::MAX as usize,
        block_size > 0,
        block_size < i32::MAX as usize,
        block_size <= len,
        is_pow2(block_size as int),
        (addr as usize) % block_size == 0,
        addr as int > 0,
        (addr as int) + (len as int) <= (usize::MAX as int),
        (len / block_size) % (u8::BITS as usize) == 0,
        len / block_size >= 8,
        mem.is_range(addr as int, len as int),
{
    match unsafe { Slab::from_raw_parts(addr, len, block_size, Tracked(mem)) } {
        Ok(pair) => {
            let (slab, Tracked(_perms)) = pair;
            proof {
                let invalid_addr = slab@.data_addr + slab@.num_data_blocks * slab@.block_size;
                assert(!slab@.is_valid_addr(invalid_addr));
            }
        },
        Err(_) => {},
    }
}

/// Verifiable test: multiple allocations return different addresses.
fn test_multiple_allocations_verified(
    addr: *mut u8,
    len: usize,
    block_size: usize,
    Tracked(mem): Tracked<PointsToRaw>,
)
    requires
        len > 0,
        len < i32::MAX as usize,
        block_size > 0,
        block_size < i32::MAX as usize,
        block_size <= len,
        is_pow2(block_size as int),
        (addr as usize) % block_size == 0,
        addr as int > 0,
        (addr as int) + (len as int) <= (usize::MAX as int),
        (len / block_size) % (u8::BITS as usize) == 0,
        len / block_size >= 16,
        mem.is_range(addr as int, len as int),
{
    match unsafe { Slab::from_raw_parts(addr, len, block_size, Tracked(mem)) } {
        Ok(pair) => {
            let (mut slab, Tracked(mut slab_perms)) = pair;
            match slab.allocate(Tracked(&mut slab_perms)) {
                Ok(pair1) => {
                    let (addr1, Tracked(_perm1)) = pair1;
                    match slab.allocate(Tracked(&mut slab_perms)) {
                        Ok(pair2) => {
                            let (addr2, Tracked(_perm2)) = pair2;
                            assert(addr1 != addr2);
                            proof {
                                let idx1 = slab@.addr_to_block_idx(addr1 as int);
                                let idx2 = slab@.addr_to_block_idx(addr2 as int);
                                assert(slab@.is_allocated(idx1));
                                assert(slab@.is_allocated(idx2));
                                assert(idx1 != idx2);
                            }
                        },
                        Err(_) => {},
                    }
                },
                Err(_) => {},
            }
        },
        Err(_) => {},
    }
}

/// Verifiable test: address computation properties.
fn test_address_computation_verified(
    addr: *mut u8,
    len: usize,
    block_size: usize,
    Tracked(mem): Tracked<PointsToRaw>,
)
    requires
        len > 0,
        len < i32::MAX as usize,
        block_size > 0,
        block_size < i32::MAX as usize,
        block_size <= len,
        is_pow2(block_size as int),
        (addr as usize) % block_size == 0,
        addr as int > 0,
        (addr as int) + (len as int) <= (usize::MAX as int),
        (len / block_size) % (u8::BITS as usize) == 0,
        len / block_size >= 8,
        mem.is_range(addr as int, len as int),
{
    match unsafe { Slab::from_raw_parts(addr, len, block_size, Tracked(mem)) } {
        Ok(pair) => {
            let (mut slab, Tracked(mut slab_perms)) = pair;
            match slab.allocate(Tracked(&mut slab_perms)) {
                Ok(alloc_pair) => {
                    let (alloc_addr, Tracked(_perm)) = alloc_pair;
                    proof {
                        assert(slab@.is_valid_addr(alloc_addr as int));
                        let block_idx = slab@.addr_to_block_idx(alloc_addr as int);
                        assert(0 <= block_idx < slab@.num_data_blocks);
                        assert(slab@.is_allocated(block_idx));
                    }
                },
                Err(_) => {},
            }
        },
        Err(_) => {},
    }
}

//==================================================================================================

/// Verifiable test: after deallocation, the same block can be reallocated.
fn test_allocation_reuse_verified(
    addr: *mut u8,
    len: usize,
    block_size: usize,
    Tracked(mem): Tracked<PointsToRaw>,
)
    requires
        len > 0,
        len < i32::MAX as usize,
        block_size > 0,
        block_size < i32::MAX as usize,
        block_size <= len,
        is_pow2(block_size as int),
        (addr as usize) % block_size == 0,
        addr as int > 0,
        (addr as int) + (len as int) <= (usize::MAX as int),
        (len / block_size) % (u8::BITS as usize) == 0,
        len / block_size >= 8,
        mem.is_range(addr as int, len as int),
{
    match unsafe { Slab::from_raw_parts(addr, len, block_size, Tracked(mem)) } {
        Ok(pair) => {
            let (mut slab, Tracked(mut slab_perms)) = pair;
            match slab.allocate(Tracked(&mut slab_perms)) {
                Ok(alloc_pair) => {
                    let (addr1, Tracked(perm1)) = alloc_pair;
                    match unsafe { slab.deallocate(addr1, Tracked(perm1), Tracked(&mut slab_perms))
                    } {
                        Ok(()) => {
                            match slab.allocate(Tracked(&mut slab_perms)) {
                                Ok(alloc_pair2) => {
                                    let (addr2, Tracked(_perm2)) = alloc_pair2;
                                    proof {
                                        assert(slab@.is_valid_addr(addr2 as int));
                                        assert(slab@.is_allocated(
                                            slab@.addr_to_block_idx(addr2 as int),
                                        ));
                                    }
                                },
                                Err(_) => {},
                            }
                        },
                        Err(_) => {},
                    }
                },
                Err(_) => {},
            }
        },
        Err(_) => {},
    }
}

/// Verifiable test: all allocated addresses are aligned to block_size.
fn test_memory_block_alignment_verified(
    addr: *mut u8,
    len: usize,
    block_size: usize,
    Tracked(mem): Tracked<PointsToRaw>,
)
    requires
        len > 0,
        len < i32::MAX as usize,
        block_size > 0,
        block_size < i32::MAX as usize,
        block_size <= len,
        is_pow2(block_size as int),
        (addr as usize) % block_size == 0,
        addr as int > 0,
        (addr as int) + (len as int) <= (usize::MAX as int),
        (len / block_size) % (u8::BITS as usize) == 0,
        len / block_size >= 16,
        mem.is_range(addr as int, len as int),
{
    match unsafe { Slab::from_raw_parts(addr, len, block_size, Tracked(mem)) } {
        Ok(pair) => {
            let (mut slab, Tracked(mut slab_perms)) = pair;
            match slab.allocate(Tracked(&mut slab_perms)) {
                Ok(pair1) => {
                    let (addr1, Tracked(_perm1)) = pair1;
                    match slab.allocate(Tracked(&mut slab_perms)) {
                        Ok(pair2) => {
                            let (addr2, Tracked(_perm2)) = pair2;
                            proof {
                                assert(slab@.is_valid_addr(addr1 as int));
                                assert(slab@.is_valid_addr(addr2 as int));
                                assert(addr1 as int >= slab@.data_addr);
                                assert(addr2 as int >= slab@.data_addr);
                            }
                        },
                        Err(_) => {},
                    }
                },
                Err(_) => {},
            }
        },
        Err(_) => {},
    }
}

/// Verifiable test: deallocating one block doesn't affect other allocated blocks.
fn test_no_data_corruption_verified(
    addr: *mut u8,
    len: usize,
    block_size: usize,
    Tracked(mem): Tracked<PointsToRaw>,
)
    requires
        len > 0,
        len < i32::MAX as usize,
        block_size > 0,
        block_size < i32::MAX as usize,
        block_size <= len,
        is_pow2(block_size as int),
        (addr as usize) % block_size == 0,
        addr as int > 0,
        (addr as int) + (len as int) <= (usize::MAX as int),
        (len / block_size) % (u8::BITS as usize) == 0,
        len / block_size >= 16,
        mem.is_range(addr as int, len as int),
{
    match unsafe { Slab::from_raw_parts(addr, len, block_size, Tracked(mem)) } {
        Ok(pair) => {
            let (mut slab, Tracked(mut slab_perms)) = pair;
            match slab.allocate(Tracked(&mut slab_perms)) {
                Ok(pair1) => {
                    let (addr1, Tracked(perm1)) = pair1;
                    match slab.allocate(Tracked(&mut slab_perms)) {
                        Ok(pair2) => {
                            let (addr2, Tracked(_perm2)) = pair2;
                            proof {
                                let idx1 = slab@.addr_to_block_idx(addr1 as int);
                                let idx2 = slab@.addr_to_block_idx(addr2 as int);
                                assert(slab@.is_allocated(idx1));
                                assert(slab@.is_allocated(idx2));
                            }
                            // Deallocate block 1.
                            match unsafe {
                                slab.deallocate(addr1, Tracked(perm1), Tracked(&mut slab_perms))
                            } {
                                Ok(()) => {
                                    proof {
                                        let idx1 = slab@.addr_to_block_idx(addr1 as int);
                                        let idx2 = slab@.addr_to_block_idx(addr2 as int);
                                        assert(!slab@.is_allocated(idx1));
                                        assert(slab@.is_allocated(idx2));
                                    }
                                },
                                Err(_) => {},
                            }
                        },
                        Err(_) => {},
                    }
                },
                Err(_) => {},
            }
        },
        Err(_) => {},
    }
}

/// Verifiable test: fresh slab has all data blocks free.
fn test_fresh_slab_all_free_verified(
    addr: *mut u8,
    len: usize,
    block_size: usize,
    Tracked(mem): Tracked<PointsToRaw>,
)
    requires
        len > 0,
        len < i32::MAX as usize,
        block_size > 0,
        block_size < i32::MAX as usize,
        block_size <= len,
        is_pow2(block_size as int),
        (addr as usize) % block_size == 0,
        addr as int > 0,
        (addr as int) + (len as int) <= (usize::MAX as int),
        (len / block_size) % (u8::BITS as usize) == 0,
        len / block_size >= 8,
        mem.is_range(addr as int, len as int),
{
    match unsafe { Slab::from_raw_parts(addr, len, block_size, Tracked(mem)) } {
        Ok(pair) => {
            let (slab, Tracked(_perms)) = pair;
            proof {
                assert(forall|i: int| 0 <= i < slab@.num_data_blocks ==> !slab@.is_allocated(i));
            }
        },
        Err(_) => {},
    }
}

/// Verifiable test: index blocks are always marked as used.
fn test_index_blocks_always_used_verified(
    addr: *mut u8,
    len: usize,
    block_size: usize,
    Tracked(mem): Tracked<PointsToRaw>,
)
    requires
        len > 0,
        len < i32::MAX as usize,
        block_size > 0,
        block_size < i32::MAX as usize,
        block_size <= len,
        is_pow2(block_size as int),
        (addr as usize) % block_size == 0,
        addr as int > 0,
        (addr as int) + (len as int) <= (usize::MAX as int),
        (len / block_size) % (u8::BITS as usize) == 0,
        len / block_size >= 8,
        mem.is_range(addr as int, len as int),
{
    match unsafe { Slab::from_raw_parts(addr, len, block_size, Tracked(mem)) } {
        Ok(pair) => {
            let (mut slab, Tracked(mut slab_perms)) = pair;
            // Invariant holds after construction.
            assert(slab.inv());
            match slab.allocate(Tracked(&mut slab_perms)) {
                Ok(alloc_pair) => {
                    let (_addr, Tracked(_perm)) = alloc_pair;
                    // Invariant still holds after allocation.
                    assert(slab.inv());
                },
                Err(_) => {},
            }
        },
        Err(_) => {},
    }
}

} // verus!
