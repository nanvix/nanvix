// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::mm::phys::{
    frame,
    upool::UserFrame,
};

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

    passed
}
