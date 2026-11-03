// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::{
    realloc,
    set_errno,
};
use ::core::ptr::null_mut;
use ::sysapi::{
    errno::ENOMEM,
    ffi::c_void,
    sys_types::c_size_t,
};

//==================================================================================================
// Standalone Functions
//==================================================================================================

///
/// # Description
///
/// Reallocates memory for an array, failing if `nelem * elsize` overflows.
///
/// # Parameters
///
/// - `ptr`: Pointer to the memory block to be reallocated.
/// - `nelem`: Number of elements.
/// - `elsize`: Size of each element in bytes.
///
/// # Returns
///
/// On success, this function returns a pointer to the reallocated memory. On failure, it returns a
/// null pointer and leaves the original block of memory pointed to by `ptr` unchanged.
///
/// # Safety
///
/// This function is unsafe because it interacts with the global memory allocator.
///
/// # References
///
/// - https://pubs.opengroup.org/onlinepubs/9799919799/functions/realloc.html
///
#[cfg_attr(not(feature = "std"), unsafe(no_mangle))]
pub unsafe extern "C" fn reallocarray(
    ptr: *mut c_void,
    nelem: c_size_t,
    elsize: c_size_t,
) -> *mut c_void {
    let size: c_size_t = match nelem.checked_mul(elsize) {
        Some(size) => size,
        None => {
            set_errno(ENOMEM);
            return null_mut();
        },
    };

    realloc(ptr, size)
}

#[cfg(all(test, feature = "std"))]
mod tests;
