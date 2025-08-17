// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Lint Configuration
//==================================================================================================

#![forbid(clippy::unwrap_used)]
#![forbid(clippy::expect_used)]
#![forbid(clippy::cast_possible_truncation)]
#![forbid(clippy::cast_possible_wrap)]
#![forbid(clippy::cast_precision_loss)]
#![forbid(clippy::cast_sign_loss)]
#![forbid(clippy::char_lit_as_u8)]
#![forbid(clippy::fn_to_numeric_cast)]
#![forbid(clippy::fn_to_numeric_cast_with_truncation)]
#![forbid(clippy::ptr_as_ptr)]
#![forbid(clippy::unnecessary_cast)]
#![forbid(invalid_reference_casting)]
#![forbid(clippy::panic)]
#![forbid(clippy::unimplemented)]
#![forbid(clippy::todo)]
#![forbid(clippy::unreachable)]
#![forbid(unsafe_code)]

//==================================================================================================
// Imports
//==================================================================================================

use ::arch::mem::PAGE_ALIGNMENT;
use ::spin::mutex::{
    SpinMutex,
    SpinMutexGuard,
};
use ::sys::{
    error::{
        Error,
        ErrorCode,
    },
    kcall::pm,
    mm::{
        align_up,
        VirtualAddress,
    },
    pm::ProcessIdentifier,
};
use ::sysalloc::{
    map_range,
    unmap_range,
    BREAK_BASE_RAW,
};

//==================================================================================================
// Global Variables
//==================================================================================================

/// Program break base address.
static BREAK_BASE: SpinMutex<usize> = SpinMutex::new(BREAK_BASE_RAW);

//==================================================================================================
// Standalone Functions
//==================================================================================================

///
/// # Description
///
/// The `sbrk()` increments the location of the program break by `size` bytes. The program break
/// defines the end of the process's data segment and is the address of the first location after the
/// end of the uninitialized data segment. Increasing the program break has the effect of allocating
/// memory to the process; decreasing the break deallocates memory. Calling `sbrk()` with a zero
/// size increment can be used to find the current location of the program break.
///
/// # Parameters
///
/// - `size`: Number of bytes to increment the program break.
///
/// # Returns
///
/// Upon successful completion, the `sbrk()` function returns the address of the start of the newly
/// allocated memory. Otherwise, it returns an error code.
///
pub fn sbrk(size: isize) -> Result<*mut u8, Error> {
    ::syslog::trace!("sbrk(): size={size:?}");

    let mut locked_base: SpinMutexGuard<'_, usize> = BREAK_BASE.lock();

    // Check if querying the current program break.
    if size == 0 {
        return Ok(*locked_base as *mut u8);
    }

    let old_end: *mut u8 = {
        let old_end: *mut u8 = *locked_base as *mut u8;
        // Compute the new end of the program break.
        let new_end: *mut u8 = match locked_base.checked_add_signed(size) {
            Some(new_end) => new_end as *mut u8,
            None => {
                let reason: &'static str = "not enough memory";
                ::syslog::error!("sbrk(): {reason} (size={size:?}), old_end={old_end:x?}");
                return Err(Error::new(ErrorCode::OutOfMemory, reason));
            },
        };

        // Align the new end.
        let new_end: *mut u8 = align_up(new_end as usize, PAGE_ALIGNMENT) as *mut u8;

        // Check whether we should allocate or free memory.
        if size > 0 {
            // Allocate memory.

            // Check if we would exceed the heap size.
            if new_end >= (sysalloc::BREAK_BASE_RAW + sysalloc::C_HEAP_SIZE) as *mut u8 {
                let reason: &'static str = "out of memory";
                ::syslog::error!(
                    "sbrk(): {reason} (size={size:?}), old_end={old_end:x?}, new_end={new_end:x?}"
                );
                return Err(Error::new(ErrorCode::OutOfMemory, reason));
            }

            let pid: ProcessIdentifier = pm::getpid()?;

            // Allocate memory.
            map_range(
                pid,
                VirtualAddress::from_raw_value(old_end as usize),
                VirtualAddress::from_raw_value(new_end as usize),
            )?;
        } else {
            // Free memory.

            // Check if we would free memory below the heap base address.
            if new_end < sysalloc::BREAK_BASE_RAW as *mut u8 {
                let reason: &'static str = "invalid allocation size";
                ::syslog::error!(
                    "sbrk(): {reason} (size={size:?}), old_end={old_end:x?}, new_end={new_end:x?}"
                );
                return Err(Error::new(ErrorCode::InvalidArgument, reason));
            }

            let pid: ProcessIdentifier = pm::getpid()?;

            // Free memory.
            unmap_range(
                pid,
                VirtualAddress::from_raw_value(new_end as usize),
                VirtualAddress::from_raw_value(old_end as usize),
            )?;
        }

        *locked_base = new_end as usize;
        old_end
    };

    Ok(old_end)
}
