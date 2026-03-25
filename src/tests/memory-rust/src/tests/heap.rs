// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use ::core::ffi::c_void;
use ::sys::error::Error;
use ::sysapi::sys_types::c_size_t;

//==================================================================================================
// External C Functions
//==================================================================================================

unsafe extern "C" {
    fn malloc(size: c_size_t) -> *mut c_void;
    fn free(ptr: *mut c_void);
    fn memset(s: *mut c_void, c: i32, n: c_size_t) -> *mut c_void;
}

//==================================================================================================
// Constants
//==================================================================================================

const KB: c_size_t = 1024;
const MB: c_size_t = 1024 * KB;

/// Block size used in bulk-free, anchor, and cycle scenarios.
const LARGE_BLOCK_SIZE: c_size_t = MB;

/// Number of large blocks in bulk-free and anchor scenarios.
const NUM_LARGE_BLOCKS: usize = 4;

/// Size of small anchor allocations.
const ANCHOR_SIZE: c_size_t = 64;

/// Number of shrink-grow iterations.
const SHRINK_GROW_CYCLES: usize = 16;

/// Number of small allocations in the small-heap scenario.
const NUM_SMALL_ALLOCS: usize = 8;

/// Size of each small allocation (well below one page).
const SMALL_ALLOC_SIZE: c_size_t = 128;

/// Number of regrowth blocks after shrink-to-minimum.
const REGROWTH_BLOCKS: usize = 4;

/// Size of each regrowth block (64 KB).
const REGROWTH_BLOCK_SIZE: c_size_t = 64 * KB;

/// Number of alloc/free rounds for the reclaim test.
const RECLAIM_ROUNDS: usize = 32;

/// Number of allocations per round.
const ALLOCS_PER_ROUND: usize = 8;

/// Block sizes to cycle through in the reclaim test.
const BLOCK_SIZES: &[c_size_t] = &[512 * KB, 256 * KB, 128 * KB, 64 * KB];

/// xorshift PRNG seed.
const XORSHIFT_SEED: u32 = 0xDEAD_BEEF;

/// Size of each large block used to fill the heap.
const FILL_BLOCK_SIZE: c_size_t = MB;

/// Maximum number of large blocks to attempt (32 MB / 1 MB).
const MAX_FILL_BLOCKS: usize = 32;

/// Size of medium blocks used in the fragmentation phase.
const MEDIUM_BLOCK_SIZE: c_size_t = 256 * KB;

/// Maximum number of medium blocks (32 MB / 256 KB).
const MAX_MEDIUM_BLOCKS: usize = 128;

/// Number of allocations in the reuse-after-drain validation.
const REUSE_ROUNDS: usize = 8;

/// Size of blocks allocated in the reuse-after-drain phase.
const REUSE_BLOCK_SIZE: c_size_t = 512 * KB;

//==================================================================================================
// Private Functions
//==================================================================================================

/// Simple 32-bit xorshift PRNG.
fn xorshift32(state: u32) -> u32 {
    let mut s: u32 = state;
    s ^= s << 13;
    s ^= s >> 17;
    s ^= s << 5;
    s
}

/// Truncates a `usize` value (already masked to 0xFF) to `u8`.
#[allow(clippy::as_conversions)]
fn low_byte(val: usize) -> u8 {
    (val & 0xFF) as u8
}

/// Converts a `c_size_t` to `usize`.
#[allow(clippy::as_conversions)]
fn sz(v: c_size_t) -> usize {
    v as usize
}

//==================================================================================================
// Standalone Functions
//==================================================================================================

/// Executes all heap tests.
pub fn run() -> Result<(), Error> {
    test_heap_reclaim()?;
    test_heap_max_capacity()?;
    test_heap_shrink()?;
    Ok(())
}

//==================================================================================================
// Heap Reclaim Test
//==================================================================================================

/// Stresses the heap allocator with many alloc/free cycles of varying sizes under fragmentation
/// pressure. Each round allocates blocks of pseudo-random sizes, interleaves them with small
/// persistent anchors, and frees the large blocks while keeping the anchors.
fn test_heap_reclaim() -> Result<(), Error> {
    // Anchors survive across rounds to create fragmentation.
    let mut anchors: [*mut c_void; RECLAIM_ROUNDS * ALLOCS_PER_ROUND] =
        [core::ptr::null_mut(); RECLAIM_ROUNDS * ALLOCS_PER_ROUND];
    let mut num_anchors: usize = 0;
    let mut rng: u32 = XORSHIFT_SEED;

    for round in 0..RECLAIM_ROUNDS {
        let mut blocks: [*mut c_void; ALLOCS_PER_ROUND] = [core::ptr::null_mut(); ALLOCS_PER_ROUND];

        for (slot, block) in blocks.iter_mut().enumerate() {
            // Pick a block size pseudo-randomly.
            rng = xorshift32(rng);
            let size_index: usize = sz(rng) % BLOCK_SIZES.len();
            let block_sz: c_size_t = BLOCK_SIZES[size_index];

            *block = unsafe { malloc(block_sz) };
            assert!(!block.is_null(), "malloc failed at round {round}, slot {slot}");

            // Tag the first byte so the allocation is not optimized away.
            let tag: u8 = low_byte(round * ALLOCS_PER_ROUND + slot);
            unsafe { *block.cast::<u8>() = tag };
            assert_eq!(unsafe { *block.cast::<u8>() }, tag);

            // Place an anchor after every second block.
            if slot % 2 == 1 {
                anchors[num_anchors] = unsafe { malloc(ANCHOR_SIZE) };
                assert!(!anchors[num_anchors].is_null());
                unsafe {
                    *anchors[num_anchors].cast::<u8>() = low_byte(round + slot);
                }
                num_anchors += 1;
            }
        }

        // Free all large blocks from this round; anchors persist.
        for block in blocks.iter() {
            unsafe { free(*block) };
        }
    }

    // Clean up all anchors.
    for anchor in anchors.iter().take(num_anchors) {
        unsafe { free(*anchor) };
    }

    Ok(())
}

//==================================================================================================
// Heap Max Capacity Test
//==================================================================================================

/// Tests the allocator by attempting to consume the maximum allowed heap capacity. Proceeds in
/// four phases: fill, drain, fragmentation, and reuse-after-drain.
fn test_heap_max_capacity() -> Result<(), Error> {
    // Phase 1: Fill the heap to near-capacity with large blocks.
    let mut large_blocks: [*mut c_void; MAX_FILL_BLOCKS] = [core::ptr::null_mut(); MAX_FILL_BLOCKS];
    let mut num_large: usize = 0;
    let mut total_allocated: usize = 0;

    for i in 0..MAX_FILL_BLOCKS {
        let p: *mut c_void = unsafe { malloc(FILL_BLOCK_SIZE) };
        if p.is_null() {
            break; // OOM — heap is full.
        }
        let tag: u8 = low_byte(i);
        unsafe { memset(p, 0, FILL_BLOCK_SIZE) };
        unsafe { *p.cast::<u8>() = tag };
        large_blocks[num_large] = p;
        num_large += 1;
        total_allocated += sz(FILL_BLOCK_SIZE);
    }

    // We should have allocated at least one block.
    assert!(total_allocated >= sz(FILL_BLOCK_SIZE), "failed to allocate even one block");

    // Verify data integrity on all blocks.
    for (i, block) in large_blocks.iter().enumerate().take(num_large) {
        let expected: u8 = low_byte(i);
        assert_eq!(unsafe { *block.cast::<u8>() }, expected);
    }

    // Phase 2: Drain and verify reclaimability.
    for block in large_blocks.iter().take(num_large) {
        unsafe { free(*block) };
    }

    // Verify reclaimability with a multi-block allocation.
    let reclaim_size: c_size_t = if num_large >= 2 {
        2 * FILL_BLOCK_SIZE
    } else {
        FILL_BLOCK_SIZE
    };
    let reclaim: *mut c_void = unsafe { malloc(reclaim_size) };
    assert!(!reclaim.is_null(), "reclaimability check failed");
    unsafe { memset(reclaim, 0xAA, reclaim_size) };
    unsafe { free(reclaim) };

    // Phase 3: Fragmentation near capacity.
    let mut medium_blocks: [*mut c_void; MAX_MEDIUM_BLOCKS] =
        [core::ptr::null_mut(); MAX_MEDIUM_BLOCKS];
    let mut num_medium: usize = 0;

    for i in 0..MAX_MEDIUM_BLOCKS {
        let p: *mut c_void = unsafe { malloc(MEDIUM_BLOCK_SIZE) };
        if p.is_null() {
            break; // OOM — heap is full.
        }
        let tag: u8 = low_byte(i);
        unsafe { memset(p, 0, MEDIUM_BLOCK_SIZE) };
        unsafe { *p.cast::<u8>() = tag };
        medium_blocks[num_medium] = p;
        num_medium += 1;
    }

    // Retain odd-indexed blocks as anchors, free even-indexed ones.
    let mut anchor_count: usize = 0;
    let mut frag_anchors: [*mut c_void; MAX_MEDIUM_BLOCKS / 2] =
        [core::ptr::null_mut(); MAX_MEDIUM_BLOCKS / 2];
    for (i, block) in medium_blocks.iter().enumerate().take(num_medium) {
        if i % 2 == 1 {
            frag_anchors[anchor_count] = *block;
            anchor_count += 1;
        } else {
            unsafe { free(*block) };
        }
    }

    // Allocate into the freed gaps.
    let mut num_gap: usize = 0;
    let mut gap_blocks: [*mut c_void; MAX_MEDIUM_BLOCKS / 2] =
        [core::ptr::null_mut(); MAX_MEDIUM_BLOCKS / 2];
    for _i in 0..anchor_count {
        let p: *mut c_void = unsafe { malloc(MEDIUM_BLOCK_SIZE) };
        if p.is_null() {
            break; // No more room.
        }
        let tag: u8 = low_byte(num_gap);
        unsafe { *p.cast::<u8>() = tag };
        gap_blocks[num_gap] = p;
        num_gap += 1;
    }

    // We should have been able to fit at least some blocks into the gaps.
    if anchor_count > 0 {
        assert!(num_gap > 0, "failed to allocate any gap blocks");
    }

    // Verify gap block integrity.
    for (i, block) in gap_blocks.iter().enumerate().take(num_gap) {
        let expected: u8 = low_byte(i);
        assert_eq!(unsafe { *block.cast::<u8>() }, expected);
    }

    // Free all gap and anchor blocks.
    for block in gap_blocks.iter().take(num_gap) {
        unsafe { free(*block) };
    }
    for anchor in frag_anchors.iter().take(anchor_count) {
        unsafe { free(*anchor) };
    }

    // Phase 4: Reuse after full drain.
    for round in 0..REUSE_ROUNDS {
        let p: *mut c_void = unsafe { malloc(REUSE_BLOCK_SIZE) };
        assert!(!p.is_null(), "reuse allocation failed at round {round}");
        let tag: u8 = low_byte(round);
        unsafe { *p.cast::<u8>() = tag };
        assert_eq!(unsafe { *p.cast::<u8>() }, tag);
        unsafe { free(p) };
    }

    Ok(())
}

//==================================================================================================
// Heap Shrink Test
//==================================================================================================

/// Exercises the heap shrink path through five scenarios that cover bulk-free, shrink-to-minimum,
/// anchor-pinned partial shrink, repeated shrink-grow cycles, and small-heap early-return.
fn test_heap_shrink() -> Result<(), Error> {
    scenario_bulk_free()?;
    scenario_shrink_to_minimum()?;
    scenario_anchors_prevent_full_shrink()?;
    scenario_shrink_grow_cycles()?;
    scenario_small_heap_minimum()?;
    Ok(())
}

/// Scenario 1: Bulk-free triggers shrink.
///
/// Allocate several large blocks so the heap grows well beyond one page, then free them all.
/// Each free at capacity triggers try_reclaim() → Heap::shrink() to unmap tail pages.
/// Re-allocate the same total to confirm pages were reclaimed and can be re-mapped.
fn scenario_bulk_free() -> Result<(), Error> {
    let mut blocks: [*mut c_void; NUM_LARGE_BLOCKS] = [core::ptr::null_mut(); NUM_LARGE_BLOCKS];

    // Allocate large blocks.
    for (i, block) in blocks.iter_mut().enumerate() {
        *block = unsafe { malloc(LARGE_BLOCK_SIZE) };
        assert!(!block.is_null());
        unsafe { memset(*block, 0, LARGE_BLOCK_SIZE) };
        unsafe { *block.cast::<u8>() = low_byte(i) };
    }

    // Verify integrity.
    for (i, block) in blocks.iter().enumerate() {
        assert_eq!(unsafe { *block.cast::<u8>() }, low_byte(i));
    }

    // Free all — each free at capacity triggers try_reclaim() → shrink.
    for block in blocks.iter() {
        unsafe { free(*block) };
    }

    // Re-allocate to confirm reclaimed pages are re-mappable.
    let mut blocks2: [*mut c_void; NUM_LARGE_BLOCKS] = [core::ptr::null_mut(); NUM_LARGE_BLOCKS];
    for (i, block) in blocks2.iter_mut().enumerate() {
        *block = unsafe { malloc(LARGE_BLOCK_SIZE) };
        assert!(!block.is_null());
        unsafe { memset(*block, 0, LARGE_BLOCK_SIZE) };
        unsafe { *block.cast::<u8>() = low_byte(i) };
    }

    // Verify integrity.
    for (i, block) in blocks2.iter().enumerate() {
        assert_eq!(unsafe { *block.cast::<u8>() }, low_byte(i));
    }

    // Cleanup.
    for block in blocks2.iter() {
        unsafe { free(*block) };
    }

    Ok(())
}

/// Scenario 2: All-freed shrinks to minimum.
///
/// Allocate a single large block (2 MB) to force heap growth, free it. With no live allocations,
/// try_reclaim() sees an empty span and shrinks the heap to PAGE_SIZE. Then allocate multiple
/// medium blocks to verify the heap grows back from minimum.
fn scenario_shrink_to_minimum() -> Result<(), Error> {
    // Grow the heap with a 2 MB allocation.
    let block: *mut c_void = unsafe { malloc(2 * MB) };
    assert!(!block.is_null());
    unsafe { memset(block, 0, 2 * MB) };
    unsafe { *block.cast::<u8>() = 0xAA };
    unsafe { free(block) };

    // Verify regrowth from minimum.
    let mut regrowth: [*mut c_void; REGROWTH_BLOCKS] = [core::ptr::null_mut(); REGROWTH_BLOCKS];
    for (i, block) in regrowth.iter_mut().enumerate() {
        *block = unsafe { malloc(REGROWTH_BLOCK_SIZE) };
        assert!(!block.is_null());
        unsafe { memset(*block, 0, REGROWTH_BLOCK_SIZE) };
        unsafe { *block.cast::<u8>() = low_byte(i) };
    }

    // Verify integrity.
    for (i, block) in regrowth.iter().enumerate() {
        assert_eq!(unsafe { *block.cast::<u8>() }, low_byte(i));
    }

    // Cleanup.
    for block in regrowth.iter() {
        unsafe { free(*block) };
    }

    Ok(())
}

/// Scenario 3: Anchors prevent full shrink.
///
/// Allocate large blocks interleaved with small persistent anchors. Free the large blocks.
/// try_reclaim() can only reclaim tail pages above the highest live anchor. Then free anchors
/// and reallocate large blocks to verify full reclaimability.
fn scenario_anchors_prevent_full_shrink() -> Result<(), Error> {
    let mut blocks: [*mut c_void; NUM_LARGE_BLOCKS] = [core::ptr::null_mut(); NUM_LARGE_BLOCKS];
    let mut anchors: [*mut c_void; NUM_LARGE_BLOCKS] = [core::ptr::null_mut(); NUM_LARGE_BLOCKS];

    // Interleave: large block, anchor, large block, anchor, ...
    for i in 0..NUM_LARGE_BLOCKS {
        blocks[i] = unsafe { malloc(LARGE_BLOCK_SIZE) };
        assert!(!blocks[i].is_null());
        unsafe { memset(blocks[i], 0, LARGE_BLOCK_SIZE) };
        unsafe { *blocks[i].cast::<u8>() = low_byte(i) };

        anchors[i] = unsafe { malloc(ANCHOR_SIZE) };
        assert!(!anchors[i].is_null());
        unsafe { memset(anchors[i], 0, ANCHOR_SIZE) };
        unsafe { *anchors[i].cast::<u8>() = low_byte(i) };
    }

    // Free large blocks; anchors persist and pin the heap above minimum.
    for block in blocks.iter() {
        unsafe { free(*block) };
    }

    // Free anchors — now the heap can fully reclaim.
    for anchor in anchors.iter() {
        unsafe { free(*anchor) };
    }

    // Verify full reclaimability by re-allocating large blocks.
    let mut blocks2: [*mut c_void; NUM_LARGE_BLOCKS] = [core::ptr::null_mut(); NUM_LARGE_BLOCKS];
    for (i, block) in blocks2.iter_mut().enumerate() {
        *block = unsafe { malloc(LARGE_BLOCK_SIZE) };
        assert!(!block.is_null());
        unsafe { memset(*block, 0, LARGE_BLOCK_SIZE) };
        unsafe { *block.cast::<u8>() = low_byte(i) };
    }

    // Verify integrity.
    for (i, block) in blocks2.iter().enumerate() {
        assert_eq!(unsafe { *block.cast::<u8>() }, low_byte(i));
    }

    // Cleanup.
    for block in blocks2.iter() {
        unsafe { free(*block) };
    }

    Ok(())
}

/// Scenario 4: Repeated shrink-grow cycles.
///
/// In a tight loop, allocate a large block then immediately free it. Each free triggers
/// try_reclaim() → shrink(), and the next allocation triggers the OOM handler → grow().
fn scenario_shrink_grow_cycles() -> Result<(), Error> {
    for i in 0..SHRINK_GROW_CYCLES {
        let block: *mut c_void = unsafe { malloc(LARGE_BLOCK_SIZE) };
        assert!(!block.is_null());
        unsafe { memset(block, 0, LARGE_BLOCK_SIZE) };
        let tag: u8 = low_byte(i);
        unsafe { *block.cast::<u8>() = tag };
        assert_eq!(unsafe { *block.cast::<u8>() }, tag);
        unsafe { free(block) };
    }

    Ok(())
}

/// Scenario 5: Small heap stays at minimum.
///
/// Allocate only small blocks whose total is well below a single page. Free them. The heap
/// should remain at PAGE_SIZE because try_reclaim() returns early when heap.size() <= PAGE_SIZE.
fn scenario_small_heap_minimum() -> Result<(), Error> {
    let mut small: [*mut c_void; NUM_SMALL_ALLOCS] = [core::ptr::null_mut(); NUM_SMALL_ALLOCS];

    // Allocate small blocks that fit within the initial page.
    for (i, block) in small.iter_mut().enumerate() {
        *block = unsafe { malloc(SMALL_ALLOC_SIZE) };
        assert!(!block.is_null());
        unsafe { memset(*block, 0, SMALL_ALLOC_SIZE) };
        unsafe { *block.cast::<u8>() = low_byte(i) };
    }

    // Free all — heap remains at PAGE_SIZE (no shrink below minimum).
    for block in small.iter() {
        unsafe { free(*block) };
    }

    // Reallocate to verify stability.
    let mut small2: [*mut c_void; NUM_SMALL_ALLOCS] = [core::ptr::null_mut(); NUM_SMALL_ALLOCS];
    for (i, block) in small2.iter_mut().enumerate() {
        *block = unsafe { malloc(SMALL_ALLOC_SIZE) };
        assert!(!block.is_null());
        unsafe { memset(*block, 0, SMALL_ALLOC_SIZE) };
        unsafe { *block.cast::<u8>() = low_byte(i) };
    }

    // Cleanup.
    for block in small2.iter() {
        unsafe { free(*block) };
    }

    Ok(())
}
