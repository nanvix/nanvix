// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

#[cfg(feature = "syscall")]
use crate::{
    errno::__errno_location,
    sys::mman,
};
#[cfg(feature = "syscall")]
use ::sys::mm::VirtualAddress;
#[cfg(feature = "syscall")]
use ::sysapi::{
    ffi::{
        c_int,
        c_void,
    },
    sys_types::c_size_t,
};

//==================================================================================================
// Standalone Functions
//==================================================================================================

///
/// # Description
///
/// Unlocks the whole pages containing any part of the address range that starts at `addr` and
/// spans `length` bytes.
///
/// Nanvix never swaps or pages out memory, so every mapped page is permanently resident. Unlocking
/// a resident page therefore has no effect, and this function only validates its arguments.
///
/// # Parameters
///
/// - `addr`: Start address of the region to unlock.
/// - `length`: Number of bytes to unlock.
///
/// # Returns
///
/// On success, returns `0`. On failure, returns `-1` and sets `errno` to indicate the error:
/// - `EINVAL`: `addr` is not a multiple of the page size.
/// - `ENOMEM`: the range extends past the end of the address space, or some page in the range is
///   not mapped.
///
/// # Safety
///
/// This function is unsafe because it may modify `errno`. It is safe to call provided that access
/// to `errno` is synchronized with other threads that may modify it.
///
/// # Known Limitations (Nanvix)
///
/// - Unlocking is a no-op: mapped pages are always resident.
///
#[cfg(feature = "syscall")]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn munlock(addr: *const c_void, length: c_size_t) -> c_int {
    match super::validate_lock_range(addr as usize, length) {
        Ok(None) => 0,
        Ok(Some((addr, length))) => {
            let addr: VirtualAddress = VirtualAddress::from_raw_value(addr);
            match mman::munlock(addr, length) {
                Ok(()) => 0,
                Err(error) => {
                    unsafe {
                        *__errno_location() = error.code.get();
                    }
                    -1
                },
            }
        },
        Err(code) => {
            unsafe {
                *__errno_location() = code.get();
            }
            -1
        },
    }
}
