// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use super::common::StressError;
use ::alloc::{
    boxed::Box,
    vec::Vec,
};
use ::config::constants::{
    KILOBYTE,
    MEGABYTE,
};
use ::core::convert::TryFrom;
use ::sys::error::{
    Error,
    ErrorCode,
};

//==================================================================================================
// Constants
//==================================================================================================

/// Maximum heap capacity as configured for the system.
const HEAP_CAPACITY: usize = ::config::memory_layout::USER_HEAP_CAPACITY;

/// Size of each large block used to fill the heap. Using 1 MB blocks balances between
/// exercising the OOM handler often enough and keeping the number of allocations manageable.
const FILL_BLOCK_SIZE: usize = MEGABYTE;

/// Maximum number of large blocks we attempt to allocate. This is derived from the heap
/// capacity so the pre-allocated `Vec` never needs to grow during the fill loop.
const MAX_FILL_BLOCKS: usize = HEAP_CAPACITY / FILL_BLOCK_SIZE;

/// Size of a medium block used in the fragmentation phase.
const MEDIUM_BLOCK_SIZE: usize = 256 * KILOBYTE;

/// Maximum number of medium blocks we attempt to allocate.
const MAX_MEDIUM_BLOCKS: usize = HEAP_CAPACITY / MEDIUM_BLOCK_SIZE;

/// Number of allocations in the reuse-after-drain validation.
const REUSE_ROUNDS: usize = 8;

/// Size of blocks allocated in the reuse-after-drain phase.
const REUSE_BLOCK_SIZE: usize = 512 * KILOBYTE;

//==================================================================================================
// Public Functions
//==================================================================================================

///
/// # Description
///
/// Tests the allocator by attempting to consume the maximum allowed heap capacity.
///
/// The test proceeds in four phases:
///
/// 1. **Fill phase** — Allocates 1 MB blocks until the heap is nearly full, verifying data
///    integrity on each block. This pushes the OOM handler to grow the backing heap repeatedly
///    until the configured `USER_HEAP_CAPACITY` is exhausted.
///
/// 2. **Drain phase** — Frees all large blocks at once and verifies the allocator can reclaim
///    the space by performing a large allocation that spans multiple previously-freed blocks.
///
/// 3. **Fragmentation phase** — Refills the heap with medium-sized blocks (256 KB), retains
///    every other block as an anchor, and frees the rest. Then attempts to allocate into the
///    freed gaps, which exercises the allocator's ability to reuse non-contiguous free space
///    near capacity.
///
/// 4. **Reuse-after-drain phase** — Frees all remaining allocations and performs several more
///    allocation/free cycles to confirm the heap is fully reusable after being drained from
///    maximum capacity.
///
/// # Returns
///
/// `Ok(())` on success or an error if any allocation or integrity check fails.
///
pub fn run() -> Result<(), StressError> {
    //==============================================================================================
    // Phase 1: Fill the heap to near-capacity with large blocks.
    //==============================================================================================

    let mut large_blocks: Vec<Box<[u8]>> = Vec::with_capacity(MAX_FILL_BLOCKS);
    let mut total_allocated: usize = 0;

    // Keep allocating until the allocator cannot satisfy another request.
    // We use try_alloc because the allocator will abort the process on a plain alloc failure.
    // The Vec is pre-allocated so push() never triggers an infallible reallocation.
    let mut block_index: usize = 0;
    while block_index < MAX_FILL_BLOCKS {
        match try_alloc_tagged_block(FILL_BLOCK_SIZE, block_index) {
            Ok(block) => {
                total_allocated += FILL_BLOCK_SIZE;
                large_blocks.push(block);
                block_index += 1;
            },
            Err(_) => break, // OOM — heap is full.
        }
    }

    // Verify we allocated a substantial amount. Even after prior tests we should be able to claim
    // a meaningful fraction of USER_HEAP_CAPACITY.
    if total_allocated < FILL_BLOCK_SIZE {
        return Err(Error::new(
            ErrorCode::InvalidArgument,
            "fill phase could not allocate even a single block",
        ));
    }

    // Verify all blocks are still intact.
    for (index, block) in large_blocks.iter().enumerate() {
        let expected_tag: u8 = u8::try_from(index & 0xFF).unwrap_or(0);
        if block[0] != expected_tag {
            return Err(Error::new(
                ErrorCode::InvalidArgument,
                "fill phase data integrity check failed",
            ));
        }
    }

    //==============================================================================================
    // Phase 2: Drain and verify reclaimability.
    //==============================================================================================

    let num_large: usize = large_blocks.len();
    drop(large_blocks);

    // The heap should now have all that space free. Verify by allocating a block that spans
    // multiple previously-freed regions.
    let reclaim_size: usize = if num_large >= 2 {
        2 * FILL_BLOCK_SIZE
    } else {
        FILL_BLOCK_SIZE
    };
    let reclaim_block: Box<[u8]> = try_alloc_tagged_block(reclaim_size, 0xAA)?;
    drop(reclaim_block);

    //==============================================================================================
    // Phase 3: Fragmentation near capacity.
    //==============================================================================================

    let mut medium_blocks: Vec<Box<[u8]>> = Vec::with_capacity(MAX_MEDIUM_BLOCKS);
    block_index = 0;

    // Fill with medium blocks until OOM. The Vec is pre-allocated so push() is safe.
    while block_index < MAX_MEDIUM_BLOCKS {
        match try_alloc_tagged_block(MEDIUM_BLOCK_SIZE, block_index) {
            Ok(block) => {
                medium_blocks.push(block);
                block_index += 1;
            },
            Err(_) => break, // OOM — heap is full.
        }
    }

    // Retain odd-indexed blocks as anchors, free even-indexed ones.
    let anchor_count: usize = medium_blocks.len() / 2;
    let mut anchors: Vec<Box<[u8]>> = Vec::with_capacity(anchor_count);
    for (index, block) in medium_blocks.into_iter().enumerate() {
        if index % 2 == 1 {
            anchors.push(block);
        }
        // Even blocks are dropped here, creating gaps.
    }

    // Attempt to allocate into the freed gaps.
    let mut gap_blocks: Vec<Box<[u8]>> = Vec::with_capacity(anchor_count);
    for index in 0..anchors.len() {
        match try_alloc_tagged_block(MEDIUM_BLOCK_SIZE, index) {
            Ok(block) => gap_blocks.push(block),
            Err(_) => break, // No more room — expected near capacity.
        }
    }

    // We should have been able to fit at least some blocks into the gaps.
    if gap_blocks.is_empty() && !anchors.is_empty() {
        return Err(Error::new(
            ErrorCode::InvalidArgument,
            "fragmentation phase could not allocate into any freed gap",
        ));
    }

    // Verify gap block integrity.
    for (index, block) in gap_blocks.iter().enumerate() {
        let expected_tag: u8 = u8::try_from(index & 0xFF).unwrap_or(0);
        if block[0] != expected_tag {
            return Err(Error::new(
                ErrorCode::InvalidArgument,
                "fragmentation gap data integrity check failed",
            ));
        }
    }

    drop(gap_blocks);
    drop(anchors);

    //==============================================================================================
    // Phase 4: Reuse after full drain.
    //==============================================================================================

    for round in 0..REUSE_ROUNDS {
        let block: Box<[u8]> = try_alloc_tagged_block(REUSE_BLOCK_SIZE, round)?;
        let expected_tag: u8 = u8::try_from(round & 0xFF).unwrap_or(0);
        if block[0] != expected_tag {
            return Err(Error::new(
                ErrorCode::InvalidArgument,
                "reuse-after-drain data integrity check failed",
            ));
        }
        drop(block);
    }

    Ok(())
}

//==================================================================================================
// Private Functions
//==================================================================================================

/// Attempts to allocate a tagged block, returning `Err` on allocation failure instead of
/// panicking. This is used in phases where running out of memory is an expected outcome.
fn try_alloc_tagged_block(size: usize, index: usize) -> Result<Box<[u8]>, Error> {
    // Use try_reserve to check capacity, then fill via write_bytes + set_len to avoid
    // the infallible reallocation path in Vec::resize.
    let mut v: Vec<u8> = Vec::new();
    v.try_reserve(size)
        .map_err(|_| Error::new(ErrorCode::OutOfMemory, "allocation failed"))?;

    // SAFETY: try_reserve succeeded so the buffer has capacity for `size` bytes.
    // We zero the memory before exposing it.
    unsafe {
        ::core::ptr::write_bytes(v.as_mut_ptr(), 0u8, size);
        v.set_len(size);
    }

    let mut block: Box<[u8]> = v.into_boxed_slice();
    let tag: u8 = u8::try_from(index & 0xFF).unwrap_or(0);
    block[0] = tag;
    Ok(block)
}
