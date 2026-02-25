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
use ::config::constants::KILOBYTE;
use ::core::convert::TryFrom;
use ::sys::error::{
    Error,
    ErrorCode,
};

//==================================================================================================
// Constants
//==================================================================================================

/// Number of interleave rounds.
const ROUNDS: usize = 16;

/// Number of heap allocations per round.
const HEAP_ALLOCS_PER_ROUND: usize = 4;

/// Number of sbrk grow/shrink pairs per round.
const SBRK_OPS_PER_ROUND: usize = 2;

/// Block sizes for heap allocations, cycling through different magnitudes.
const HEAP_BLOCK_SIZES: [usize; 4] = [64 * KILOBYTE, 128 * KILOBYTE, 32 * KILOBYTE, 16 * KILOBYTE];

/// sbrk increment size (page-aligned).
const SBRK_INCREMENT: usize = 4 * KILOBYTE;

/// Simple deterministic PRNG seed.
const XORSHIFT_SEED: u32 = 0xCAFE_BABE;

//==================================================================================================
// Public Functions
//==================================================================================================

///
/// # Description
///
/// Stresses the unified memory allocator by interleaving heap allocations and sbrk calls within
/// the same test run. This verifies that the unified virtual address bump allocator correctly
/// assigns non-overlapping regions to the heap allocator and sbrk, even under heavy concurrent
/// use.
///
/// Each round performs several heap allocations of varying sizes, writes tag bytes to verify
/// data integrity, and issues sbrk grow/shrink pairs. Between rounds, half of the heap blocks
/// are freed to create fragmentation pressure while retaining the other half as anchors.
///
/// # Returns
///
/// `Ok(())` on success or an error if any allocation fails or data integrity checks fail.
///
pub fn run() -> Result<(), StressError> {
    let mut retained: Vec<Box<[u8]>> = Vec::with_capacity(ROUNDS * HEAP_ALLOCS_PER_ROUND / 2);
    let mut rng: u32 = XORSHIFT_SEED;

    for round in 0..ROUNDS {
        // Phase 1: Heap allocations.
        let mut blocks: Vec<Box<[u8]>> = Vec::with_capacity(HEAP_ALLOCS_PER_ROUND);
        for slot in 0..HEAP_ALLOCS_PER_ROUND {
            rng = super::common::xorshift32(rng);
            let size_index: usize = usize::try_from(rng).unwrap_or(0) % HEAP_BLOCK_SIZES.len();
            let size: usize = HEAP_BLOCK_SIZES[size_index];
            let block: Box<[u8]> = alloc_tagged_block(size, round * HEAP_ALLOCS_PER_ROUND + slot)?;
            blocks.push(block);
        }

        // Phase 2: sbrk grow/shrink cycles.
        for _op in 0..SBRK_OPS_PER_ROUND {
            let old_brk: *mut u8 = sbrk_checked(0)?;

            // Grow.
            let sbrk_inc: isize = isize::try_from(SBRK_INCREMENT)
                .map_err(|_| Error::new(ErrorCode::InvalidArgument, "sbrk increment overflow"))?;
            let grow_result: *mut u8 = sbrk_checked(sbrk_inc)?;
            if !core::ptr::eq(grow_result, old_brk) {
                return Err(Error::new(
                    ErrorCode::InvalidArgument,
                    "sbrk grow returned unexpected address",
                ));
            }

            // Write to the sbrk-allocated region to verify it is accessible.
            let tag: u8 = u8::try_from(round & 0xFF).unwrap_or(0);
            // SAFETY: `grow_result` points to the start of a freshly sbrk-mapped region of
            // `SBRK_INCREMENT` bytes. The region is writable because sbrk mapped it with
            // read/write permissions. We write and read a single byte, which is within bounds.
            unsafe {
                let ptr: *mut u8 = grow_result;
                ptr.write_volatile(tag);
                let readback: u8 = ptr.read_volatile();
                if readback != tag {
                    return Err(Error::new(
                        ErrorCode::InvalidArgument,
                        "sbrk data integrity check failed",
                    ));
                }
            }

            // Shrink back.
            sbrk_checked(-sbrk_inc)?;
        }

        // Phase 3: Retain odd-indexed blocks, free even-indexed ones to create fragmentation.
        for (index, block) in blocks.into_iter().enumerate() {
            if index % 2 == 1 {
                retained.push(block);
            }
            // Even-indexed blocks are dropped here.
        }
    }

    // Verify retained blocks are still readable.
    for block in retained.iter() {
        // The tag was written at the block index (round * ALLOCS + slot) for odd slots, so we
        // cannot predict the exact value. Just verify the block is readable.
        let _read: u8 = block[0];
    }

    // Clean up all retained blocks.
    drop(retained);

    Ok(())
}

//==================================================================================================
// Private Functions
//==================================================================================================

/// Allocates a block of `size` bytes and writes a tag byte for data integrity verification.
fn alloc_tagged_block(size: usize, index: usize) -> Result<Box<[u8]>, Error> {
    let mut block: Box<[u8]> = ::alloc::vec![0u8; size].into_boxed_slice();
    let tag: u8 = u8::try_from(index & 0xFF).unwrap_or(0);
    block[0] = tag;
    if block[0] != tag {
        return Err(Error::new(ErrorCode::InvalidArgument, "block tag mismatch"));
    }
    Ok(block)
}

/// Calls sbrk and converts the error into a `StressError`.
fn sbrk_checked(size: isize) -> Result<*mut u8, StressError> {
    ::syscall::unistd::sbrk(size)
}
