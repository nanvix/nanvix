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
use ::sys::error::{
    Error,
    ErrorCode,
};

//==================================================================================================
// Constants
//==================================================================================================

/// Number of alloc/free rounds. Each round performs `ALLOCS_PER_ROUND` allocations of varying
/// sizes, keeps some alive as anchors, and frees the rest. This exercises the OOM handler under
/// realistic fragmentation: a mix of large stack-sized blocks and medium/small blocks that
/// prevent coalescing.
const ROUNDS: usize = 32;

/// Number of allocations per round.
const ALLOCS_PER_ROUND: usize = 8;

/// Block sizes to cycle through. The mix of large (512 KB, 256 KB) and medium (128 KB, 64 KB)
/// sizes creates fragmentation that Talc cannot always resolve by reusing a single freed block.
const BLOCK_SIZES: [usize; 4] = [
    512 * KILOBYTE,
    256 * KILOBYTE,
    128 * KILOBYTE,
    64 * KILOBYTE,
];

/// Size of small anchor allocations placed between freed blocks.
const ANCHOR_SIZE: usize = 64;

/// Simple deterministic hasher seeded per iteration to vary allocation sizes without requiring
/// a random number generator. Uses a 32-bit xorshift.
const XORSHIFT_SEED: u32 = 0xDEAD_BEEF;

//==================================================================================================
// Public Functions
//==================================================================================================

///
/// # Description
///
/// Stresses the heap allocator with many alloc/free cycles of varying sizes under fragmentation
/// pressure.
///
/// Each round allocates 8 blocks of pseudo-random sizes drawn from `BLOCK_SIZES`, interleaves
/// them with small persistent anchors, then frees the large blocks while keeping the anchors.
/// The varying sizes prevent the allocator from trivially reusing a same-sized freed block and
/// force the OOM handler to run. Accumulated anchors create address-space fragmentation that
/// stresses heap reclamation under realistic workloads.
///
/// # Returns
///
/// `Ok(())` on success or an error if any allocation fails.
///
pub fn run() -> Result<(), StressError> {
    let mut anchors: Vec<Box<[u8; ANCHOR_SIZE]>> = Vec::with_capacity(ROUNDS * ALLOCS_PER_ROUND);
    let mut rng: u32 = XORSHIFT_SEED;

    for round in 0..ROUNDS {
        let mut blocks: Vec<Box<[u8]>> = Vec::with_capacity(ALLOCS_PER_ROUND);

        for slot in 0..ALLOCS_PER_ROUND {
            // Pick a block size pseudo-randomly.
            rng = super::common::xorshift32(rng);
            let size_index: usize = usize::try_from(rng).unwrap_or(0) % BLOCK_SIZES.len();
            let size: usize = BLOCK_SIZES[size_index];

            let block: Box<[u8]> = alloc_block(size, round * ALLOCS_PER_ROUND + slot)?;
            blocks.push(block);

            // Place an anchor after every second block to fragment the heap without
            // anchoring every single allocation (which would use too much metadata space).
            if slot % 2 == 1 {
                let mut anchor: Box<[u8; ANCHOR_SIZE]> = Box::new([0u8; ANCHOR_SIZE]);
                anchor[0] = u8::try_from((round + slot) & 0xFF).unwrap_or(0);
                anchors.push(anchor);
            }
        }

        // Free all large blocks from this round. Anchors persist, splitting the free list.
        drop(blocks);
    }

    // Clean up all anchors.
    drop(anchors);

    Ok(())
}

//==================================================================================================
// Private Functions
//==================================================================================================

///
/// # Description
///
/// Allocates a zeroed block of `size` bytes and writes a tag into the first byte for
/// verification. Note that allocation failure will abort the process (Rust's default global
/// allocator behavior); only the tag verification can return an error.
///
/// # Parameters
///
/// - `size`: Number of bytes to allocate.
/// - `index`: Block index (used for the tag byte).
///
/// # Returns
///
/// A boxed byte slice on success, or an error if the tag verification fails.
///
fn alloc_block(size: usize, index: usize) -> Result<Box<[u8]>, Error> {
    let mut block: Box<[u8]> = ::alloc::vec![0u8; size].into_boxed_slice();
    // Write a tag so the allocation is not optimized away.
    let tag: u8 = u8::try_from(index & 0xFF).unwrap_or(0);
    block[0] = tag;
    if block[0] != tag {
        return Err(Error::new(ErrorCode::InvalidArgument, "block tag mismatch"));
    }
    Ok(block)
}
