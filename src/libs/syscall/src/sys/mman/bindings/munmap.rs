// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::sys::mman;
use ::sys::{
    error::ErrorCode,
    mm::VirtualAddress,
};
use ::sysapi::{
    errno::__errno_location,
    ffi::c_void,
    sys_types::c_size_t,
};
use ::syslog::trace_syscall;

//==================================================================================================
// Standalone Functions
//==================================================================================================

///
/// # Description
///
/// Unmaps pages of memory. The `munmap()` function removes any mappings for the entire pages
/// covering the address range that starts at `addr` and spans `length` bytes. After a successful
/// call, further references to those pages shall result in a fault.
///
/// # Parameters
///
/// - `addr`: Start address of the region to unmap.
/// - `length`: Number of bytes to unmap; must be greater than zero.
///
/// # Returns
///
/// On success, returns `0`. On failure, returns `-1` and sets `errno` to indicate the error.
///
/// # Safety
///
/// This function is unsafe because it may modify global state.
///
/// It is safe to call this function if and only if the following conditions are met:
/// - Access to `errno` is synchronized with other threads that may modify it.
///
/// # Known Limitations (Nanvix)
///
/// - The `addr` must match the base address previously returned by `mmap()`.
/// - Partial unmapping is not supported.
///
#[unsafe(no_mangle)]
#[trace_syscall]
pub unsafe extern "C" fn munmap(addr: *mut c_void, length: c_size_t) -> isize {
    // Check if address is invalid.
    if addr.is_null() {
        ::syslog::error!("munmap(): invalid base address (addr={addr:?}, length={length})");
        unsafe {
            *__errno_location() = ErrorCode::InvalidArgument.get();
        }
        return -1;
    }

    // Check if mapping length is invalid.
    if length == 0 {
        ::syslog::error!("munmap(): invalid mapping length (addr={addr:?}, length={length})");
        unsafe {
            *__errno_location() = ErrorCode::InvalidArgument.get();
        }
        return -1;
    }

    // Attempt to convert length.
    let length: usize = match TryFrom::try_from(length) {
        Ok(length) => length,
        Err(error) => {
            ::syslog::error!(
                "munmap(): invalid mapping length (addr={addr:?}, length={length}), \
                 error={error:?}"
            );
            unsafe {
                *__errno_location() = ErrorCode::InvalidArgument.get();
            }
            return -1;
        },
    };

    // Convert base pointer to virtual address.
    let base: VirtualAddress = VirtualAddress::from_raw_value(addr as usize);

    // Unmap memory segment and check for errors.
    match mman::munmap(base, length) {
        Ok(()) => {
            ::syslog::trace!("munmap(): success (addr={addr:?}, length={length})");
            0
        },
        Err(error) => {
            ::syslog::error!("munmap(): failed (addr={addr:?}, length={length}), error={error:?}");
            unsafe {
                *__errno_location() = error.code.get();
            }
            -1
        },
    }
}
