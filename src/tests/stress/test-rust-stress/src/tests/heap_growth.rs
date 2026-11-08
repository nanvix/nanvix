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
use ::config::{
    constants::{
        KILOBYTE,
        MEGABYTE,
    },
    kernel::MEMORY_SIZE,
    memory_layout::USER_HEAP_MIN_GROWTH,
};
use ::core::convert::TryFrom;
use ::sys::{
    error::{
        Error,
        ErrorCode,
    },
    kcall::{
        mm::{
            __kcall_mmap,
            __kcall_munmap,
        },
        pm::getpid_uncached,
    },
    mm::{
        AccessPermission,
        Address,
        VirtualAddress,
    },
    pm::ProcessIdentifier,
};
use ::sysalloc::{
    PAGE_SIZE,
    heap_capacity,
    heap_committed_size,
};
use ::syscall::sys::mman::mmap_reserve;

//==================================================================================================
// Constants
//==================================================================================================

/// Size of the blocks that drive bulk growth. Without bulk growth, the heap would grow by just a
/// few pages for each of these blocks that does not fit into it.
const SMALL_BLOCK_SIZE: usize = 16 * KILOBYTE;

/// Maximum number of full-chunk heap growths observed while allocating small blocks.
const NUM_BULK_GROWTHS: usize = 4;

/// Size of the blocks allocated while physical memory is short. The increment that fits one of
/// these blocks (the block plus an overhead page) must be satisfiable from `HOG_SLACK_PAGES`.
const MEDIUM_BLOCK_SIZE: usize = 64 * KILOBYTE;

/// Number of physical frames left free while hogging physical memory: enough for an increment
/// that fits a medium block, yet fewer than a growth chunk needs, unless the chunk is tiny.
const HOG_SLACK_PAGES: usize = 64;

/// Maximum number of pages mapped by a single kernel call while hogging physical memory.
const HOG_BATCH_PAGES: usize = 1024;

/// Size of the blocks used to fill the heap up to its capacity.
const LARGE_BLOCK_SIZE: usize = MEGABYTE;

// The slack must cover a fallback increment for a medium block.
const _: () = assert!(MEDIUM_BLOCK_SIZE / PAGE_SIZE + 1 < HOG_SLACK_PAGES);

//==================================================================================================
// Structures
//==================================================================================================

///
/// # Description
///
/// Physical memory mapped into a private region of the address space, so that other allocations
/// run short of physical frames. The pages are unmapped when the hog is released or dropped.
///
struct PhysicalMemoryHog {
    pid: ProcessIdentifier,
    base: VirtualAddress,
    npages: usize,
}

impl PhysicalMemoryHog {
    ///
    /// # Description
    ///
    /// Maps physical frames until the kernel refuses to map a single additional page, and then
    /// unmaps `slack` of them again.
    ///
    /// # Parameters
    ///
    /// - `slack`: Number of frames to leave free.
    ///
    /// # Returns
    ///
    /// The hog on success, or an error if reserving its address space or unmapping fails.
    ///
    fn new(slack: usize) -> Result<Self, Error> {
        let pid: ProcessIdentifier = getpid_uncached()?;

        // Physical memory never exceeds MEMORY_SIZE, so a region of that size fits every frame.
        let max_pages: usize = MEMORY_SIZE / PAGE_SIZE;
        let base: VirtualAddress = mmap_reserve(MEMORY_SIZE)?;
        let mut hog: Self = Self {
            pid,
            base,
            npages: 0,
        };

        // Map pages in large batches, halving the batch size whenever the kernel refuses it. The
        // kernel rolls back a batch that fails part-way, so a refused batch maps nothing.
        let mut batch: usize = HOG_BATCH_PAGES;
        while batch > 0 && hog.npages < max_pages {
            let count: usize = batch.min(max_pages - hog.npages);
            if __kcall_mmap(pid, hog.page(hog.npages), count, AccessPermission::RDWR).is_ok() {
                hog.npages += count;
            } else {
                batch /= 2;
            }
        }

        hog.unmap_tail(slack)?;
        Ok(hog)
    }

    ///
    /// # Description
    ///
    /// Unmaps every page of the hog, giving the physical frames back.
    ///
    /// # Returns
    ///
    /// `Ok(())` on success, or an error if unmapping a page fails.
    ///
    fn release(mut self) -> Result<(), Error> {
        self.unmap_tail(self.npages)
    }

    /// Returns the virtual address of the page at `index` in the hog region.
    fn page(&self, index: usize) -> VirtualAddress {
        VirtualAddress::from_raw_value(self.base.into_raw_value() + index * PAGE_SIZE)
    }

    /// Unmaps the last `count` mapped pages of the hog region.
    fn unmap_tail(&mut self, count: usize) -> Result<(), Error> {
        for _ in 0..count.min(self.npages) {
            __kcall_munmap(self.pid, self.page(self.npages - 1))?;
            self.npages -= 1;
        }
        Ok(())
    }
}

impl Drop for PhysicalMemoryHog {
    fn drop(&mut self) {
        // Best effort: release() reports errors, this only covers early returns.
        let _ = self.unmap_tail(self.npages);
    }
}

//==================================================================================================
// Public Functions
//==================================================================================================

///
/// # Description
///
/// Exercises bulk heap growth in the out-of-memory handler of the system allocator:
///
/// 1. Small allocations grow the heap by doubling it, then in chunks of `USER_HEAP_MIN_GROWTH`.
/// 2. Short physical memory makes the heap fall back to a smaller increment.
///
/// The scenarios adapt to the configured `USER_HEAP_MIN_GROWTH` and heap capacity, and skip what a
/// configuration does not leave room for. The first scenario covers the doubling of a small heap
/// only if the heap is still smaller than a chunk, so this test should run before any other test
/// grows the heap.
///
/// # Returns
///
/// `Ok(())` on success or an error if any allocation or growth check fails.
///
pub fn run_bulk_growth() -> Result<(), StressError> {
    scenario_small_allocations_grow_in_chunks()?;
    scenario_low_physical_memory_falls_back()?;
    Ok(())
}

///
/// # Description
///
/// Exercises the out-of-memory handler of the system allocator close to the heap capacity, where
/// a heap that can no longer grow by a full chunk falls back to smaller increments and fills up.
///
/// The heap must keep growing by chunks before it gets close to its capacity for this test to
/// cover the switch to smaller increments, and this test leaves the heap committed close to its
/// capacity.
///
/// # Returns
///
/// `Ok(())` on success or an error if the heap stops growing short of its capacity.
///
pub fn run_near_capacity() -> Result<(), StressError> {
    scenario_near_capacity_falls_back()
}

//==================================================================================================
// Private Functions — Scenarios
//==================================================================================================

/// Scenario 1: Small allocations grow the heap in chunks.
///
/// Allocates small blocks and checks the heap growths they trigger, until the heap has grown by
/// `NUM_BULK_GROWTHS` full chunks, or until a further growth would leave less than a chunk of
/// headroom below the heap capacity, which the next scenario needs. Every growth must commit at
/// least as many bytes as the heap held, up to `USER_HEAP_MIN_GROWTH` bytes, so a single growth
/// serves many subsequent allocations rather than the heap growing again for each block that does
/// not fit. Conversely, a heap smaller than a chunk must not commit a full chunk, so that programs
/// with small heaps do not pay for mapping memory they never use.
fn scenario_small_allocations_grow_in_chunks() -> Result<(), StressError> {
    let limit: usize = heap_capacity().saturating_sub(USER_HEAP_MIN_GROWTH);

    // The heap does not grow beyond the limit, and every block takes up at least SMALL_BLOCK_SIZE
    // bytes of it, so this many blocks are always enough to observe every growth.
    let max_blocks: usize = limit / SMALL_BLOCK_SIZE + 1;
    let mut blocks: Vec<Box<[u8]>> = try_vec_with_capacity(max_blocks)?;

    let mut chunk_growths: usize = 0;
    while chunk_growths < NUM_BULK_GROWTHS {
        // The heap only grows at the end of grow_with_blocks(), so this is its size before then.
        let committed: usize = heap_committed_size();
        let chunk: usize = committed.min(USER_HEAP_MIN_GROWTH);
        if committed + chunk.max(SMALL_BLOCK_SIZE + PAGE_SIZE) > limit {
            break;
        }

        let increment: usize = grow_with_blocks(&mut blocks, SMALL_BLOCK_SIZE)?;
        if increment < chunk {
            return Err(Error::new(
                ErrorCode::InvalidArgument,
                "heap grew by less than its growth chunk",
            ));
        }

        if committed >= USER_HEAP_MIN_GROWTH {
            chunk_growths += 1;
        } else if increment > committed.max(SMALL_BLOCK_SIZE + PAGE_SIZE) {
            return Err(Error::new(
                ErrorCode::InvalidArgument,
                "heap smaller than a chunk grew by more than its size",
            ));
        }
    }

    verify_tags(&blocks)
}

/// Scenario 2: Short physical memory falls back to a smaller increment.
///
/// Leaves just a few free physical frames, so that a growth chunk can no longer be committed even
/// though it fits below the heap capacity. Then allocates medium blocks until the heap grows: the
/// growth must fall back to an increment that fits the block instead of failing the allocation.
/// The scenario is skipped if the chunk is small enough to fit into the frames left free, or too
/// large to fit below the heap capacity.
fn scenario_low_physical_memory_falls_back() -> Result<(), StressError> {
    // Allocate bookkeeping before physical memory gets short. Free space inside the heap is
    // bounded by its capacity, so this many blocks are always enough to force a growth.
    let max_blocks: usize = heap_capacity() / MEDIUM_BLOCK_SIZE + 2;
    let mut blocks: Vec<Box<[u8]>> = try_vec_with_capacity(max_blocks)?;

    // The heap only grows at the end of grow_with_blocks(), so this is the chunk it then attempts.
    let committed: usize = heap_committed_size();
    let chunk: usize = committed.min(USER_HEAP_MIN_GROWTH);
    if chunk / PAGE_SIZE <= HOG_SLACK_PAGES || heap_capacity().saturating_sub(committed) < chunk {
        return Ok(());
    }

    let hog: PhysicalMemoryHog = PhysicalMemoryHog::new(HOG_SLACK_PAGES)?;
    let result: Result<usize, Error> = grow_with_blocks(&mut blocks, MEDIUM_BLOCK_SIZE);
    hog.release()?;

    if result? >= chunk {
        return Err(Error::new(
            ErrorCode::InvalidArgument,
            "heap grew by a full chunk while physical memory was short",
        ));
    }

    verify_tags(&blocks)
}

/// Scenario 3: A heap close to its capacity falls back to smaller increments.
///
/// Fills the heap with large blocks until an allocation fails. Once a full chunk no longer fits
/// below the capacity, the heap must keep growing by what each block needs, so it fills up to
/// less than one block short of its capacity.
fn scenario_near_capacity_falls_back() -> Result<(), StressError> {
    let capacity: usize = heap_capacity();
    let max_blocks: usize = capacity / LARGE_BLOCK_SIZE + 1;
    let mut blocks: Vec<Box<[u8]>> = try_vec_with_capacity(max_blocks)?;

    while blocks.len() < blocks.capacity() {
        match try_alloc_tagged_block(LARGE_BLOCK_SIZE, blocks.len()) {
            Ok(block) => blocks.push(block),
            // The heap is full.
            Err(_) => break,
        }
    }

    if capacity.saturating_sub(heap_committed_size()) >= LARGE_BLOCK_SIZE {
        return Err(Error::new(
            ErrorCode::InvalidArgument,
            "heap stopped growing more than one block short of its capacity",
        ));
    }

    verify_tags(&blocks)
}

//==================================================================================================
// Private Functions — Helpers
//==================================================================================================

/// Allocates blocks of `size` bytes into `blocks` until the heap grows, without ever reallocating
/// `blocks`. Returns the number of bytes by which the heap grew.
fn grow_with_blocks(blocks: &mut Vec<Box<[u8]>>, size: usize) -> Result<usize, Error> {
    while blocks.len() < blocks.capacity() {
        let committed: usize = heap_committed_size();
        blocks.push(try_alloc_tagged_block(size, blocks.len())?);
        let grown: usize = heap_committed_size().saturating_sub(committed);
        if grown > 0 {
            return Ok(grown);
        }
    }

    Err(Error::new(ErrorCode::InvalidArgument, "heap did not grow"))
}

/// Allocates an empty vector with room for `capacity` elements, failing instead of aborting.
fn try_vec_with_capacity<T>(capacity: usize) -> Result<Vec<T>, Error> {
    let mut v: Vec<T> = Vec::new();
    v.try_reserve_exact(capacity)
        .map_err(|_| Error::new(ErrorCode::OutOfMemory, "allocation failed"))?;
    Ok(v)
}

/// Allocates a zeroed block of `size` bytes tagged with `index`, failing instead of aborting.
fn try_alloc_tagged_block(size: usize, index: usize) -> Result<Box<[u8]>, Error> {
    let mut v: Vec<u8> = try_vec_with_capacity(size)?;

    // SAFETY: the vector has capacity for `size` bytes, which are zeroed before being exposed.
    unsafe {
        ::core::ptr::write_bytes(v.as_mut_ptr(), 0u8, size);
        v.set_len(size);
    }

    let mut block: Box<[u8]> = v.into_boxed_slice();
    block[0] = tag(index);
    Ok(block)
}

/// Checks that every block still carries the tag of its index.
fn verify_tags(blocks: &[Box<[u8]>]) -> Result<(), Error> {
    for (index, block) in blocks.iter().enumerate() {
        if block[0] != tag(index) {
            return Err(Error::new(ErrorCode::InvalidArgument, "block tag mismatch"));
        }
    }
    Ok(())
}

/// Returns the tag byte of the block at `index`.
fn tag(index: usize) -> u8 {
    u8::try_from(index & 0xFF).unwrap_or(0)
}
