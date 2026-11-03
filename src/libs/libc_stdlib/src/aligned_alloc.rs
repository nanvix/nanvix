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
/// Allocates memory with a specified alignment.
///
/// This function allocates unused space in memory for an object whose alignment is specified by
/// `alignment`, whose size in bytes is specified by `size`, and whose value is indeterminate.
///
/// # Parameters
///
/// - `alignment`: Alignment in bytes.
/// - `size`: Size in bytes.
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
/// - https://pubs.opengroup.org/onlinepubs/9799919799/functions/aligned_alloc.html
///
#[cfg_attr(not(feature = "std"), unsafe(no_mangle))]
pub unsafe extern "C" fn aligned_alloc(alignment: c_size_t, size: c_size_t) -> *mut c_void {
    // Check for zero-size allocation.
    if size == 0 {
        // Zero-size allocations have implementation-defined behavior,
        // thus log a warning message and return null.
        warn!("aligned_alloc(): zero-size allocation (alignment={alignment:?}, size={size:?})");
        set_errno(EINVAL);
        return null_mut();
    }

    // Check for invalid alignment.
    if !BlockHeader::supports_alignment(alignment as usize) {
        warn!("aligned_alloc(): invalid alignment (alignment={alignment:?}, size={size:?})");
        set_errno(EINVAL);
        return null_mut();
    }

    if !size.is_multiple_of(alignment) {
        warn!(
            "aligned_alloc(): size is not a multiple of alignment (alignment={alignment:?}, \
             size={size:?})"
        );
        set_errno(EINVAL);
        return null_mut();
    }

    // Allocate memory and check for errors.
    let ptr: *mut u8 = BlockHeader::alloc(size as usize, Some(alignment as usize));
    if ptr.is_null() {
        warn!("aligned_alloc(): allocation failed (alignment={alignment:?}, size={size:?})");
        set_errno(ENOMEM);
        return null_mut();
    }

    ptr.cast::<c_void>()
}

#[cfg(all(test, feature = "std"))]
mod tests;
