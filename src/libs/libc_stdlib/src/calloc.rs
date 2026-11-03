// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::{
    block_header::BlockHeader,
    set_errno,
};
use ::core::ptr::null_mut;
use ::sysapi::{
    errno::{
        EINVAL,
        ENOMEM,
    },
    ffi::c_void,
    sys_types::c_size_t,
};

//==================================================================================================
// Standalone Functions
//==================================================================================================

///
/// # Description
///
/// Allocates memory and initializes all bits to zero.
///
/// This function allocates unused space in memory for an array of `nmemb` elements of `size` bytes
/// each and initializes all its bits to zero.
///
/// # Parameters
///
/// - `nmemb`: Number of elements.
/// - `size`: Size of each element in bytes.
///
/// # Returns
///
/// On success, this function returns a pointer to the allocated memory. On failure, it returns a
/// null pointer and sets `errno` to indicate the error.
///
/// # Safety
///
/// This function is unsafe because it interacts with the global memory allocator.
///
/// # References
///
/// - https://pubs.opengroup.org/onlinepubs/9799919799/functions/calloc.html
///
#[cfg_attr(not(feature = "std"), unsafe(no_mangle))]
pub unsafe extern "C" fn calloc(nmemb: c_size_t, size: c_size_t) -> *mut c_void {
    // Check for zero-size allocation.
    if nmemb == 0 || size == 0 {
        // Zero-size allocations have implementation-defined behavior,
        // thus log a warning message and return null.
        warn!("calloc(): zero-size allocation (nmemb={nmemb:?}, size={size:?})");
        set_errno(EINVAL);
        return null_mut();
    }

    // Compute total allocation size checking for overflows.
    let nmemb: usize = nmemb as usize;
    let size: usize = size as usize;
    let total: usize = match nmemb.checked_mul(size) {
        Some(t) => t,
        None => {
            warn!("calloc(): size overflow (nmemb={nmemb:?}, size={size:?})");
            set_errno(ENOMEM);
            return null_mut();
        },
    };

    // Allocate memory and check for errors.
    let ptr: *mut u8 = BlockHeader::alloc(total, None);
    if ptr.is_null() {
        warn!("calloc(): allocation failed (nmemb={nmemb:?}, size={size:?})");
        set_errno(ENOMEM);
        return null_mut();
    }

    // Zero initialization.
    for i in 0..total {
        ptr.add(i).write(0u8);
    }
    ptr.cast::<c_void>()
}

#[cfg(all(test, feature = "std"))]
mod tests;
