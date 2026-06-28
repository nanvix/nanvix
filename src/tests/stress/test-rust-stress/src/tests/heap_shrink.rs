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

/// Block size used in bulk-free, anchor, and cycle scenarios. Large enough to push the heap
/// to capacity, where the allocator triggers `try_reclaim()` on both dealloc and alloc paths.
const LARGE_BLOCK_SIZE: usize = MEGABYTE;

/// Number of large blocks allocated in the bulk-free and anchor scenarios.
const NUM_LARGE_BLOCKS: usize = 4;

/// Size of small anchor allocations that pin the heap and prevent full tail reclamation.
const ANCHOR_SIZE: usize = 64;

/// Number of shrink-grow iterations in the repeated-cycle scenario.
const SHRINK_GROW_CYCLES: usize = 16;

/// Number of small allocations in the small-heap scenario.
const NUM_SMALL_ALLOCS: usize = 8;

/// Size of each small allocation. Must be well below a single page (4 KB).
const SMALL_ALLOC_SIZE: usize = 128;

/// Number of small blocks allocated after shrink-to-minimum to verify regrowth.
const REGROWTH_BLOCKS: usize = 4;

/// Size of each regrowth block (64 KB each; total 256 KB forces multiple pages).
const REGROWTH_BLOCK_SIZE: usize = 64 * KILOBYTE;

//==================================================================================================
// Public Functions
//==================================================================================================

///
/// # Description
///
/// Exercises the heap shrink path introduced in `Heap::shrink()` and the `try_reclaim()` logic
/// that triggers it during `dealloc()`. Five scenarios cover all major code paths:
///
/// 1. Bulk-free triggers shrink — frees at capacity trigger `try_reclaim()`.
/// 2. All-freed shrinks to minimum — empty span causes shrink to `PAGE_SIZE`.
/// 3. Anchors prevent full shrink — persistent small allocations limit tail reclamation.
/// 4. Repeated shrink-grow cycles — self-healing OOM→grow after each shrink.
/// 5. Small heap stays at minimum — heap at `PAGE_SIZE` triggers early-return in `try_reclaim`.
///
/// # Returns
///
/// `Ok(())` on success or an error if any allocation or integrity check fails.
///
pub fn run() -> Result<(), StressError> {
    scenario_bulk_free()?;
    scenario_shrink_to_minimum()?;
    scenario_anchors_prevent_full_shrink()?;
    scenario_shrink_grow_cycles()?;
    scenario_small_heap_minimum()?;
    Ok(())
}

//==================================================================================================
// Private Functions — Scenarios
//==================================================================================================

/// Scenario 1: Bulk-free triggers shrink.
///
/// Allocate several large blocks so the heap grows well beyond one page, then free them all.
/// Each free at capacity triggers `try_reclaim()` which calls `Heap::shrink()` to unmap tail
/// pages. Re-allocate the same total to confirm pages were reclaimed and can be re-mapped.
///
/// Exercises: `shrink()` normal unmap path, capacity-based `try_reclaim()` trigger.
fn scenario_bulk_free() -> Result<(), StressError> {
    let mut blocks: Vec<Box<[u8]>> = Vec::with_capacity(NUM_LARGE_BLOCKS);

    for i in 0..NUM_LARGE_BLOCKS {
        blocks.push(alloc_tagged_block(LARGE_BLOCK_SIZE, i)?);
    }

    // Verify integrity before freeing.
    verify_tags(&blocks)?;

    // Free all — cumulative freed (4 MB) exceeds heap.size()/4, triggering shrink.
    drop(blocks);

    // Re-allocate the same total to confirm reclaimed pages are re-mappable.
    let mut blocks2: Vec<Box<[u8]>> = Vec::with_capacity(NUM_LARGE_BLOCKS);
    for i in 0..NUM_LARGE_BLOCKS {
        blocks2.push(alloc_tagged_block(LARGE_BLOCK_SIZE, i)?);
    }
    verify_tags(&blocks2)?;
    drop(blocks2);

    Ok(())
}

/// Scenario 2: All-freed shrinks to minimum.
///
/// Allocate a single large block to force heap growth, then free it. With no live allocations,
/// `try_reclaim()` sees an empty span and shrinks the heap to `PAGE_SIZE`. Then allocate several
/// medium blocks to verify the heap can grow back from the minimum.
///
/// Exercises: `try_reclaim()` empty-span path, `shrink()` clamp to `PAGE_SIZE`.
fn scenario_shrink_to_minimum() -> Result<(), StressError> {
    // Grow the heap with a 2 MB allocation.
    let block: Box<[u8]> = alloc_tagged_block(2 * MEGABYTE, 0xAA)?;
    drop(block);
    // Heap should now be shrunk to PAGE_SIZE (empty span → minimum).

    // Verify regrowth from minimum by allocating multiple blocks.
    let mut regrowth: Vec<Box<[u8]>> = Vec::with_capacity(REGROWTH_BLOCKS);
    for i in 0..REGROWTH_BLOCKS {
        regrowth.push(alloc_tagged_block(REGROWTH_BLOCK_SIZE, i)?);
    }
    verify_tags(&regrowth)?;
    drop(regrowth);

    Ok(())
}

/// Scenario 3: Anchors prevent full shrink.
///
/// Allocate large blocks interleaved with small persistent anchors. Free the large blocks.
/// `try_reclaim()` can only reclaim tail pages above the highest live allocation (an anchor),
/// so the heap does not shrink to minimum. Then free anchors and reallocate large blocks to
/// verify full reclaimability.
///
/// Exercises: `try_reclaim()` partial-reclaim path where `alloc_end < current_size` but
/// `alloc_end > PAGE_SIZE`.
fn scenario_anchors_prevent_full_shrink() -> Result<(), StressError> {
    let mut blocks: Vec<Box<[u8]>> = Vec::with_capacity(NUM_LARGE_BLOCKS);
    let mut anchors: Vec<Box<[u8; ANCHOR_SIZE]>> = Vec::with_capacity(NUM_LARGE_BLOCKS);

    // Interleave: large block, anchor, large block, anchor, ...
    for i in 0..NUM_LARGE_BLOCKS {
        blocks.push(alloc_tagged_block(LARGE_BLOCK_SIZE, i)?);

        let mut anchor: Box<[u8; ANCHOR_SIZE]> = Box::new([0u8; ANCHOR_SIZE]);
        anchor[0] = u8::try_from(i & 0xFF).unwrap_or(0);
        anchors.push(anchor);
    }

    // Free large blocks; anchors persist and pin the heap above minimum.
    drop(blocks);

    // Free anchors — now the heap can fully reclaim.
    drop(anchors);

    // Verify full reclaimability by re-allocating large blocks.
    let mut blocks2: Vec<Box<[u8]>> = Vec::with_capacity(NUM_LARGE_BLOCKS);
    for i in 0..NUM_LARGE_BLOCKS {
        blocks2.push(alloc_tagged_block(LARGE_BLOCK_SIZE, i)?);
    }
    verify_tags(&blocks2)?;
    drop(blocks2);

    Ok(())
}

/// Scenario 4: Repeated shrink-grow cycles.
///
/// In a tight loop, allocate a large block then immediately free it. Each free triggers
/// `try_reclaim()` → `shrink()`, and the next allocation triggers the OOM handler → `grow()`.
/// This exercises the self-healing path where the Talc span is repeatedly truncated and
/// re-extended.
///
/// Exercises: shrink→OOM→grow cycle, capacity-based reclaim trigger.
fn scenario_shrink_grow_cycles() -> Result<(), StressError> {
    for i in 0..SHRINK_GROW_CYCLES {
        let block: Box<[u8]> = alloc_tagged_block(LARGE_BLOCK_SIZE, i)?;
        let expected_tag: u8 = u8::try_from(i & 0xFF).unwrap_or(0);
        if block[0] != expected_tag {
            return Err(Error::new(ErrorCode::InvalidArgument, "cycle tag mismatch"));
        }
        drop(block);
        // Each drop frees 1 MB; the heap is at capacity so dealloc triggers
        // try_reclaim() → shrink. The next iteration's alloc triggers OOM → grow.
    }

    Ok(())
}

/// Scenario 5: Small heap stays at minimum.
///
/// Allocate only small blocks whose total is well below a single page. Free them. The heap
/// should remain at `PAGE_SIZE` throughout because `try_reclaim()` returns early when
/// `heap.size() <= PAGE_SIZE`. Verify stability by allocating and freeing small blocks again.
///
/// Exercises: `try_reclaim()` early-return for minimum-sized heap.
fn scenario_small_heap_minimum() -> Result<(), StressError> {
    // Allocate small blocks that fit within the initial page.
    let mut small: Vec<Box<[u8]>> = Vec::with_capacity(NUM_SMALL_ALLOCS);
    for i in 0..NUM_SMALL_ALLOCS {
        small.push(alloc_tagged_block(SMALL_ALLOC_SIZE, i)?);
    }

    // Free all — heap should remain at PAGE_SIZE (no shrink below minimum).
    drop(small);

    // Reallocate to verify stability.
    let mut small2: Vec<Box<[u8]>> = Vec::with_capacity(NUM_SMALL_ALLOCS);
    for i in 0..NUM_SMALL_ALLOCS {
        small2.push(alloc_tagged_block(SMALL_ALLOC_SIZE, i)?);
    }
    drop(small2);

    Ok(())
}

//==================================================================================================
// Private Functions — Helpers
//==================================================================================================

/// Allocates a zeroed block of `size` bytes and writes a tag into the first byte.
fn alloc_tagged_block(size: usize, index: usize) -> Result<Box<[u8]>, Error> {
    let mut block: Box<[u8]> = ::alloc::vec![0u8; size].into_boxed_slice();
    let tag: u8 = u8::try_from(index & 0xFF).unwrap_or(0);
    block[0] = tag;
    if block[0] != tag {
        return Err(Error::new(ErrorCode::InvalidArgument, "block tag mismatch"));
    }
    Ok(block)
}

/// Verifies tag integrity on a slice of tagged blocks.
fn verify_tags(blocks: &[Box<[u8]>]) -> Result<(), StressError> {
    for (index, block) in blocks.iter().enumerate() {
        let expected_tag: u8 = u8::try_from(index & 0xFF).unwrap_or(0);
        if block[0] != expected_tag {
            return Err(Error::new(ErrorCode::InvalidArgument, "tag verification failed"));
        }
    }
    Ok(())
}
