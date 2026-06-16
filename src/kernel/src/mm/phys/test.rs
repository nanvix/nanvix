// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::{
    hal::mem::FrameAddress,
    mm::phys::{
        frame,
        upool::UserFrame,
        PhysMemoryManager,
    },
};
use ::arch::mem::PAGE_SIZE;
use ::sys::error::ErrorCode;

//==================================================================================================
// Constants
//==================================================================================================

/// Total number of physical frames derived from `MEMORY_SIZE` and `FRAME_SIZE`.
const TOTAL_FRAMES: usize = config::kernel::MEMORY_SIZE / ::arch::mem::FRAME_SIZE;

/// Number of frames allocated per round. Matches the number of frames initially mapped for a
/// user-level stack at process creation (`USER_STACK_MIN_SIZE / FRAME_SIZE`), so each round
/// exercises the eager stack allocation pattern without depending on demand-paged growth.
const FRAMES_PER_ROUND: usize =
    ::config::memory_layout::USER_STACK_MIN_SIZE / ::arch::mem::FRAME_SIZE;

/// Number of alloc/free rounds. Each round allocates `FRAMES_PER_ROUND` frames and then drops them.
/// If any frames leak, later rounds will exhaust the frame allocator. Set to twice the number of
/// rounds needed to fill the pool so that even a single leaked frame per round causes exhaustion
/// before the test ends.
const ROUNDS: usize = TOTAL_FRAMES / FRAMES_PER_ROUND * 2;

/// Static backing store for one round's worth of [`UserFrame`]s. Avoids heap allocation so the
/// test cannot exhaust the kernel heap regardless of `FRAMES_PER_ROUND`.
///
/// # Safety
///
/// Accessed only from the single-threaded kernel test path; no concurrent access is possible.
static mut FRAMES: [Option<UserFrame>; FRAMES_PER_ROUND] = [const { None }; FRAMES_PER_ROUND];

/// Number of frames per kernel region exercised by the region tests. Greater than one so the
/// contiguity check is meaningful, yet small enough to be satisfiable this early in boot.
const REGION_FRAMES: usize = 4;

/// Number of alloc/free rounds for the kernel-region leak-detection test. Each round allocates and
/// frees one region, so a single leaked frame per round would visibly shrink the free pool.
const REGION_ROUNDS: usize = 64;

//==================================================================================================
// Tests
//==================================================================================================

///
/// # Description
///
/// Allocates a batch of [`UserFrame`]s and drops them, verifying that `Drop` returns every frame
/// to the allocator. Repeats for several rounds so that even a single leaked frame per round
/// eventually exhausts the pool and causes an allocation failure.
///
fn test_user_frame_drop_reclaims_frames() -> bool {
    for round in 0..ROUNDS {
        // SAFETY: single-threaded kernel; no concurrent access to `FRAMES`.
        let frames: &mut [Option<UserFrame>; FRAMES_PER_ROUND] =
            unsafe { &mut *core::ptr::addr_of_mut!(FRAMES) };

        for slot in frames.iter_mut() {
            match frame::alloc() {
                Ok(addr) => *slot = Some(UserFrame::new(addr)),
                Err(e) => {
                    error!("frame allocation failed at round {round} (error={e:?}), possible leak");
                    // Clear any slots populated in this round so their `UserFrame`s are dropped
                    // before returning on the error path.
                    for slot in frames.iter_mut() {
                        *slot = None;
                    }
                    return false;
                },
            }
        }
        // Setting each slot to `None` drops the `UserFrame`, which frees the underlying physical
        // frame via `UserFrame::Drop`.
        for slot in frames.iter_mut() {
            *slot = None;
        }
    }
    true
}

///
/// # Description
///
/// Verifies that [`UserFrame::leak`] prevents `Drop` from freeing the frame, then manually frees
/// the leaked frame to confirm it was still allocated.
///
fn test_user_frame_leak_prevents_drop() -> bool {
    // Allocate a frame and leak it.
    let addr = match frame::alloc() {
        Ok(addr) => addr,
        Err(e) => {
            error!("frame allocation failed (error={e:?})");
            return false;
        },
    };
    let uframe: UserFrame = UserFrame::new(addr);
    let leaked_addr = uframe.leak();

    // The frame should still be allocated (not freed). Freeing it manually must succeed.
    match frame::free(leaked_addr) {
        Ok(()) => true,
        Err(e) => {
            error!("failed to free leaked frame (error={e:?}), leak() may have run Drop");
            false
        },
    }
}

///
/// # Description
///
/// Verifies that a normal drop of [`UserFrame`] actually frees the frame by confirming a
/// double-free fails.
///
fn test_user_frame_drop_frees_frame() -> bool {
    let addr = match frame::alloc() {
        Ok(addr) => addr,
        Err(e) => {
            error!("frame allocation failed (error={e:?})");
            return false;
        },
    };

    // Drop the `UserFrame`, which should free the underlying frame.
    let uframe: UserFrame = UserFrame::new(addr);
    drop(uframe);

    // Attempting to free the same frame again must fail because Drop already freed it.
    match frame::free(addr) {
        Ok(()) => {
            error!("double-free succeeded, Drop did not free the frame");
            false
        },
        Err(_) => true,
    }
}

///
/// # Description
///
/// Verifies that [`UserFrame::share`] increments the reference count on the underlying
/// physical frame, so the frame remains allocated until all aliases are dropped.
///
fn test_user_frame_share_keeps_frame_alive() -> bool {
    let addr = match frame::alloc() {
        Ok(addr) => addr,
        Err(e) => {
            error!("frame allocation failed (error={e:?})");
            return false;
        },
    };

    let first: UserFrame = UserFrame::new(addr);

    // Create a second owner via `share`. The underlying frame is now shared
    // between `first` and `second`.
    let second: UserFrame = match first.share() {
        Ok(handle) => handle,
        Err(e) => {
            error!("share failed (error={e:?})");
            return false;
        },
    };

    // Dropping `first` decrements the reference count to 1; the frame must
    // still be allocated.
    drop(first);

    // Re-sharing from `second` must succeed because the frame is still alive.
    let third: UserFrame = match second.share() {
        Ok(handle) => handle,
        Err(e) => {
            error!("share after partial drop failed (error={e:?})");
            return false;
        },
    };

    // Drop the remaining handles. The last drop reclaims the frame.
    drop(second);
    drop(third);

    // After the final drop, the frame must be free; an explicit `free` of the
    // same address must therefore fail.
    match frame::free(addr) {
        Ok(()) => {
            error!("frame was not reclaimed after all shared owners dropped");
            false
        },
        Err(_) => true,
    }
}

///
/// # Description
///
/// Verifies that [`PhysMemoryManager::alloc_kernel_region`] rejects a zero-length request with
/// [`ErrorCode::InvalidArgument`] instead of allocating or panicking.
///
fn test_alloc_kernel_region_rejects_zero_count() -> bool {
    // SAFETY: the kernel runs single-threaded with interrupts disabled on the init-time test
    // path, and the physical memory manager is initialized before tests run.
    let pmm: &mut PhysMemoryManager = unsafe { PhysMemoryManager::get_mut() };
    match pmm.alloc_kernel_region(0) {
        Ok(base) => {
            error!("alloc_kernel_region(0) unexpectedly succeeded (base={base:?})");
            false
        },
        Err(e) if e.code == ErrorCode::InvalidArgument => true,
        Err(e) => {
            error!("alloc_kernel_region(0) returned unexpected error (error={e:?})");
            false
        },
    }
}

///
/// # Description
///
/// Verifies that [`PhysMemoryManager::alloc_kernel_region`] returns a page-aligned base backed by
/// `REGION_FRAMES` physically contiguous, currently-allocated frames. Each frame in the range is
/// released individually with [`frame::free`] (which succeeds only for an allocated frame), which
/// both proves the range was contiguous and cleans up without relying on `free_kernel_region`.
///
fn test_alloc_kernel_region_allocates_contiguous_frames() -> bool {
    // SAFETY: single-threaded kernel init path; the physical memory manager is initialized.
    let pmm: &mut PhysMemoryManager = unsafe { PhysMemoryManager::get_mut() };
    let base: FrameAddress = match pmm.alloc_kernel_region(REGION_FRAMES) {
        Ok(base) => base,
        Err(e) => {
            error!("alloc_kernel_region({REGION_FRAMES}) failed (error={e:?})");
            return false;
        },
    };

    let base_raw: usize = base.into_raw_value();
    let mut passed: bool = true;

    // The base of the region must be page-aligned.
    if !base_raw.is_multiple_of(PAGE_SIZE) {
        error!("region base is not page-aligned (base={base_raw:#x})");
        passed = false;
    }

    // Each frame in the contiguous range must currently be allocated. Freeing it must succeed; a
    // failure means the frame was never allocated, i.e. the returned range was not contiguous.
    for i in 0..REGION_FRAMES {
        let raw: usize = base_raw + i * PAGE_SIZE;
        let addr: FrameAddress = match FrameAddress::from_raw_value(raw) {
            Ok(addr) => addr,
            Err(e) => {
                error!("invalid frame address in region (raw={raw:#x}, error={e:?})");
                passed = false;
                continue;
            },
        };
        if let Err(e) = frame::free(addr) {
            error!("frame {i} of region was not allocated (addr={raw:#x}, error={e:?})");
            passed = false;
        }
    }

    passed
}

///
/// # Description
///
/// Verifies that [`PhysMemoryManager::free_kernel_region`] rejects a zero-length request with
/// [`ErrorCode::InvalidArgument`], mirroring [`PhysMemoryManager::alloc_kernel_region`] instead of
/// silently treating it as a successful no-op.
///
fn test_free_kernel_region_rejects_zero_count() -> bool {
    // SAFETY: the kernel runs single-threaded with interrupts disabled on the init-time test
    // path, and the physical memory manager is initialized before tests run.
    let pmm: &mut PhysMemoryManager = unsafe { PhysMemoryManager::get_mut() };

    // Allocate a real region so the base is a valid frame address; the zero-length free request
    // must still be rejected without releasing anything.
    let base: FrameAddress = match pmm.alloc_kernel_region(REGION_FRAMES) {
        Ok(base) => base,
        Err(e) => {
            error!("alloc_kernel_region({REGION_FRAMES}) failed (error={e:?})");
            return false;
        },
    };

    let passed: bool = match pmm.free_kernel_region(base, 0) {
        Ok(()) => {
            error!("free_kernel_region(_, 0) unexpectedly succeeded");
            false
        },
        Err(e) if e.code == ErrorCode::InvalidArgument => true,
        Err(e) => {
            error!("free_kernel_region(_, 0) returned unexpected error (error={e:?})");
            false
        },
    };

    // Release the region that was allocated for the test regardless of the assertion outcome.
    if let Err(e) = pmm.free_kernel_region(base, REGION_FRAMES) {
        error!("cleanup free_kernel_region failed (error={e:?})");
        return false;
    }

    passed
}

///
/// # Description
///
/// Verifies that [`PhysMemoryManager::free_kernel_region`] releases every frame of a region
/// previously obtained from [`PhysMemoryManager::alloc_kernel_region`]. After the region is freed,
/// a redundant [`frame::free`] of each constituent frame must fail, confirming the frame had
/// already been returned to the allocator.
///
fn test_free_kernel_region_releases_all_frames() -> bool {
    // SAFETY: single-threaded kernel init path; the physical memory manager is initialized.
    let pmm: &mut PhysMemoryManager = unsafe { PhysMemoryManager::get_mut() };
    let base: FrameAddress = match pmm.alloc_kernel_region(REGION_FRAMES) {
        Ok(base) => base,
        Err(e) => {
            error!("alloc_kernel_region({REGION_FRAMES}) failed (error={e:?})");
            return false;
        },
    };

    // Release the whole region. This is the function under test.
    if let Err(e) = pmm.free_kernel_region(base, REGION_FRAMES) {
        error!("free_kernel_region failed (error={e:?})");
        return false;
    }

    // Every frame must now be free: a redundant free must fail (double-free detection). If a frame
    // was not actually released, the redundant free instead succeeds here, which both flags the
    // bug and reclaims the frame so the test does not leak.
    let base_raw: usize = base.into_raw_value();
    let mut passed: bool = true;
    for i in 0..REGION_FRAMES {
        let raw: usize = base_raw + i * PAGE_SIZE;
        let addr: FrameAddress = match FrameAddress::from_raw_value(raw) {
            Ok(addr) => addr,
            Err(e) => {
                error!("invalid frame address in region (raw={raw:#x}, error={e:?})");
                passed = false;
                continue;
            },
        };
        if frame::free(addr).is_ok() {
            error!("frame {i} still allocated after free_kernel_region (addr={raw:#x})");
            passed = false;
        }
    }

    passed
}

///
/// # Description
///
/// Verifies that repeated [`PhysMemoryManager::alloc_kernel_region`] /
/// [`PhysMemoryManager::free_kernel_region`] round-trips conserve the frame pool exactly: the free
/// count drops by `REGION_FRAMES` while a region is held and is fully restored once it is released,
/// with no net change across many rounds. A single leaked frame per round would show up as a
/// shrinking free count.
///
fn test_kernel_region_roundtrip_conserves_frames() -> bool {
    // SAFETY: single-threaded kernel init path; the physical memory manager is initialized.
    let pmm: &mut PhysMemoryManager = unsafe { PhysMemoryManager::get_mut() };

    let initial_free: usize = frame::free_count();

    for round in 0..REGION_ROUNDS {
        let before: usize = frame::free_count();

        let base: FrameAddress = match pmm.alloc_kernel_region(REGION_FRAMES) {
            Ok(base) => base,
            Err(e) => {
                error!("alloc_kernel_region failed at round {round} (error={e:?})");
                return false;
            },
        };

        // Holding the region must consume exactly REGION_FRAMES frames. The comparison is written
        // additively to avoid any risk of unsigned underflow.
        let held: usize = frame::free_count();
        if held + REGION_FRAMES != before {
            error!(
                "unexpected free count after alloc at round {round} (before={before}, held={held})"
            );
            // Release before bailing so the failure path does not leak the region.
            let _ = pmm.free_kernel_region(base, REGION_FRAMES);
            return false;
        }

        if let Err(e) = pmm.free_kernel_region(base, REGION_FRAMES) {
            error!("free_kernel_region failed at round {round} (error={e:?})");
            return false;
        }

        // Releasing the region must restore the free count exactly.
        let after: usize = frame::free_count();
        if after != before {
            error!("free count not restored after round {round} (before={before}, after={after})");
            return false;
        }
    }

    // No net change in the free pool across all rounds.
    let final_free: usize = frame::free_count();
    if final_free != initial_free {
        error!("net frame leak across rounds (initial={initial_free}, final={final_free})");
        return false;
    }

    true
}

//==================================================================================================
// Standalone Functions
//==================================================================================================

/// Runs all user-frame physical memory tests.
pub fn test() -> bool {
    let mut passed: bool = true;

    passed &= run_test!(test_user_frame_drop_reclaims_frames);
    passed &= run_test!(test_user_frame_leak_prevents_drop);
    passed &= run_test!(test_user_frame_drop_frees_frame);
    passed &= run_test!(test_user_frame_share_keeps_frame_alive);
    passed &= run_test!(test_alloc_kernel_region_rejects_zero_count);
    passed &= run_test!(test_alloc_kernel_region_allocates_contiguous_frames);
    passed &= run_test!(test_free_kernel_region_rejects_zero_count);
    passed &= run_test!(test_free_kernel_region_releases_all_frames);
    passed &= run_test!(test_kernel_region_roundtrip_conserves_frames);

    passed
}
