// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use super::common::StressError;
use ::config::constants::KILOBYTE;
use ::core::convert::TryFrom;
use ::sys::error::{
    Error,
    ErrorCode,
};

//==================================================================================================
// Constants
//==================================================================================================

/// Number of grow/shrink cycles.
const CYCLES: usize = 64;

/// Increments to cycle through (all page-aligned).
const INCREMENTS: [usize; 4] = [4 * KILOBYTE, 8 * KILOBYTE, 16 * KILOBYTE, 32 * KILOBYTE];

/// Number of consecutive grow operations before shrinking back.
const GROW_BURST: usize = 4;

//==================================================================================================
// Public Functions
//==================================================================================================

///
/// # Description
///
/// Stresses the sbrk allocator with many grow/shrink cycles of varying sizes. This exercises
/// the lazy page mapping and unmapping logic that backs the unified mmap region.
///
/// The test proceeds in cycles. Each cycle grows the program break by `GROW_BURST` consecutive
/// increments of varying sizes, writes a tag byte to each newly mapped page, then shrinks back
/// by the same total amount. This verifies that:
/// - The sbrk region can grow and shrink repeatedly without leaking address space.
/// - Freshly mapped pages are writable and readable.
/// - The program break returns to its original position after each cycle.
///
/// # Returns
///
/// `Ok(())` on success or an error if any sbrk call fails or data integrity checks fail.
///
pub fn run() -> Result<(), StressError> {
    let mut rng: u32 = 0xBAAD_F00D;

    // Record the initial break position.
    let initial_brk: *mut u8 = sbrk_checked(0)?;

    for cycle in 0..CYCLES {
        let mut total_grown: usize = 0;

        // Grow phase: multiple consecutive increments.
        for burst in 0..GROW_BURST {
            rng = super::common::xorshift32(rng);
            let inc_index: usize = usize::try_from(rng).unwrap_or(0) % INCREMENTS.len();
            let increment: usize = INCREMENTS[inc_index];

            let sbrk_inc: isize = isize::try_from(increment)
                .map_err(|_| Error::new(ErrorCode::InvalidArgument, "sbrk increment overflow"))?;
            let old_brk: *mut u8 = sbrk_checked(sbrk_inc)?;

            // Write a tag to the first byte of the newly mapped region.
            let tag: u8 = u8::try_from((cycle * GROW_BURST + burst) & 0xFF).unwrap_or(0);
            // SAFETY: `old_brk` points to the start of a freshly sbrk-mapped region of
            // `increment` bytes. The region is writable because sbrk mapped it with read/write
            // permissions. We write and read a single byte, which is within bounds.
            unsafe {
                old_brk.write_volatile(tag);
                let readback: u8 = old_brk.read_volatile();
                if readback != tag {
                    return Err(Error::new(
                        ErrorCode::InvalidArgument,
                        "sbrk grow data integrity check failed",
                    ));
                }
            }

            total_grown += increment;
        }

        // Verify the break position advanced by the expected total.
        let after_grow_brk: *mut u8 = sbrk_checked(0)?;
        let expected_brk: usize = super::common::raw_pointer_address(initial_brk) + total_grown;
        if super::common::raw_pointer_address(after_grow_brk) != expected_brk {
            return Err(Error::new(
                ErrorCode::InvalidArgument,
                "sbrk break position mismatch after grow",
            ));
        }

        // Shrink phase: give back all the grown memory.
        let shrink_amount: isize = isize::try_from(total_grown)
            .map_err(|_| Error::new(ErrorCode::InvalidArgument, "sbrk shrink amount overflow"))?;
        sbrk_checked(-shrink_amount)?;

        // Verify the break position is back to the initial position.
        let after_shrink_brk: *mut u8 = sbrk_checked(0)?;
        if !core::ptr::eq(after_shrink_brk, initial_brk) {
            return Err(Error::new(
                ErrorCode::InvalidArgument,
                "sbrk break position mismatch after shrink",
            ));
        }
    }

    Ok(())
}

//==================================================================================================
// Private Functions
//==================================================================================================

/// Calls sbrk and converts the error into a `StressError`.
fn sbrk_checked(size: isize) -> Result<*mut u8, StressError> {
    ::syscall::unistd::sbrk(size)
}
