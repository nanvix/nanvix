// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::{
    block_header::BlockHeader,
    free,
    malloc,
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
/// Reallocates memory.
///
/// This function deallocate the old object pointed to by `ptr` and return a pointer to a new object
/// that has the size specified by `size`. The contents of the new object shall be the same as that
/// of the old object prior to deallocation, up to the lesser of the new and old sizes. Any bytes in
/// the new object beyond the size of the old object have indeterminate values.
///
/// # Parameters
///
/// - `ptr`: Pointer to the memory block to be reallocated.
/// - `size`: New size in bytes.
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
pub unsafe extern "C" fn realloc(ptr: *mut c_void, size: c_size_t) -> *mut c_void {
    // Check for alias call to `malloc()`.
    if ptr.is_null() {
        return malloc(size);
    }

    // Check for alias call to `free()`.
    if size == 0 {
        free(ptr);
        return null_mut();
    }

    // Reallocate memory and check for errors.
    let new_ptr: *mut u8 = BlockHeader::realloc(ptr.cast::<u8>(), size as usize);
    if new_ptr.is_null() {
        warn!("realloc(): reallocation failed (ptr={ptr:?}, size={size:?})");
        set_errno(ENOMEM);
        return null_mut();
    }

    new_ptr.cast::<c_void>()
}

#[cfg(all(test, feature = "std"))]
mod tests;
