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
use ::spin::{
    Lazy,
    Mutex,
    MutexGuard,
};
use ::sys::{
    error::{
        Error,
        ErrorCode,
    },
    kcall::pm,
    mm::{
        align_up,
        Address,
        VirtualAddress,
    },
    pm::ProcessIdentifier,
};
use ::sysalloc::{
    map_range,
    unmap_range,
};

//==================================================================================================
// Constants
//==================================================================================================

/// Maximum capacity of the sbrk heap in bytes.
const SBRK_CAPACITY: usize = ::config::memory_layout::USER_SBRK_CAPACITY;

//==================================================================================================
// Structures
//==================================================================================================

/// State of the sbrk program break allocator.
struct SbrkState {
    /// Base address of the sbrk region (beginning of the reserved virtual address range).
    base: VirtualAddress,
    /// Current end of the program break (next byte to be allocated).
    current: VirtualAddress,
    /// Maximum end address (base + capacity).
    end: VirtualAddress,
}

//==================================================================================================
// Global Variables
//==================================================================================================

/// Lazily initialized sbrk state. The virtual address space is reserved from the unified mmap
/// region on first access. If reservation fails, the state is `None` and every subsequent `sbrk()`
/// call returns `OutOfMemory`.
static SBRK_STATE: Lazy<Mutex<Option<SbrkState>>> =
    Lazy::new(|| match ::sysalloc::vaddr::reserve(SBRK_CAPACITY) {
        Ok(base) => {
            let end_raw: usize = base.into_raw_value() + SBRK_CAPACITY;
            Mutex::new(Some(SbrkState {
                base,
                current: base,
                end: VirtualAddress::new(end_raw),
            }))
        },
        Err(e) => {
            ::syslog::error!(
                "SBRK_STATE: failed to reserve virtual address space (capacity={} bytes): {:?}",
                SBRK_CAPACITY,
                e
            );
            Mutex::new(None)
        },
    });

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

    let mut locked: MutexGuard<'_, Option<SbrkState>> = SBRK_STATE.lock();

    let state: &mut SbrkState = locked.as_mut().ok_or_else(|| {
        let reason: &str = "sbrk region was not initialized";
        ::syslog::error!("sbrk(): {reason}");
        Error::new(ErrorCode::OutOfMemory, reason)
    })?;

    // Check if querying the current program break.
    if size == 0 {
        return Ok(state.current.into_raw_value() as *mut u8);
    }

    let old_end: *mut u8 = {
        let old_end: *mut u8 = state.current.into_raw_value() as *mut u8;
        // Compute the new end of the program break.
        let new_end: *mut u8 = match state.current.into_raw_value().checked_add_signed(size) {
            Some(new_end) => new_end as *mut u8,
            None => {
                let reason: &'static str = "not enough memory";
                ::syslog::error!("sbrk(): {reason} (size={size:?}), old_end={old_end:x?}");
                return Err(Error::new(ErrorCode::OutOfMemory, reason));
            },
        };

        // Align the new end.
        let new_end: *mut u8 = match align_up(new_end as usize, PAGE_ALIGNMENT) {
            Some(aligned_new_end) => aligned_new_end as *mut u8,
            None => {
                let reason: &'static str = "align_up overflow";
                ::syslog::error!(
                    "sbrk(): {reason} (size={size:?}), old_end={old_end:x?}, new_end={new_end:x?}"
                );
                return Err(Error::new(ErrorCode::OutOfMemory, reason));
            },
        };

        // Check whether we should allocate or free memory.
        if size > 0 {
            // Allocate memory.

            // Check if we would exceed the sbrk capacity.
            if new_end as usize > state.end.into_raw_value() {
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

            // Check if we would free memory below the sbrk base address.
            if (new_end as usize) < state.base.into_raw_value() {
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

        state.current = VirtualAddress::from_raw_value(new_end as usize);
        old_end
    };

    Ok(old_end)
}
