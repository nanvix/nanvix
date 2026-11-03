// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::block_header::BlockHeader;
use ::sysapi::{
    errno::{
        EINVAL,
        ENOMEM,
    },
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
/// Allocates memory with a specified alignment.
///
/// This function allocates unused space in memory for an object whose alignment is specified by
/// `alignment`, whose size in bytes is specified by `size`, and whose value is indeterminate.
///
/// The value of `alignment` must be a power of two and a multiple of `sizeof(c_void *)`.
///
/// # Parameters
///
/// - `memptr`: Pointer to the allocated memory pointer.
/// - `alignment`: Alignment in bytes.
/// - `size`: Size in bytes.
///
/// # Returns
///
/// On success, this function returns `0` and sets `memptr` to the allocated memory. On failure, it
/// returns an error code and leaves `memptr` unmodified.
///
/// # Safety
///
/// This function is unsafe because it interacts with the global memory allocator.
///
/// # References
///
/// - https://pubs.opengroup.org/onlinepubs/9799919799/functions/posix_memalign.html
///
#[cfg_attr(not(feature = "std"), unsafe(no_mangle))]
pub unsafe extern "C" fn posix_memalign(
    memptr: *mut *mut c_void,
    alignment: c_size_t,
    size: c_size_t,
) -> c_int {
    // Check if `memptr` is null.
    if memptr.is_null() {
        warn!("posix_memalign(): null storage (alignment={alignment:?}, size={size:?})");
        return EINVAL;
    }

    let size: usize = size as usize;
    let alignment: usize = alignment as usize;

    // Check for zero-size allocation.
    if size == 0 {
        // Zero-size allocations have implementation-defined behavior,
        // thus log a warning message and return error.
        warn!("posix_memalign(): zero-size allocation (alignment={alignment:?}, size={size:?})");
        return EINVAL;
    }

    // Check for null alignment.
    if alignment == 0 {
        // Zero-size alignments have implementation-defined behavior,
        // thus log a warning message and return error.
        warn!("posix_memalign(): zero-size alignment (alignment={alignment:?}, size={size:?})");
        return EINVAL;
    }

    // Check for invalid alignment.
    if !alignment.is_multiple_of(size_of::<*mut c_void>()) || !alignment.is_power_of_two() {
        warn!("posix_memalign(): invalid alignment (alignment={alignment:?}, size={size:?})");
        return EINVAL;
    }

    // Allocate memory and check for errors.
    let ptr: *mut u8 = BlockHeader::alloc(size, Some(alignment));
    if ptr.is_null() {
        warn!("posix_memalign(): allocation failed (alignment={alignment:?}, size={size:?})");
        return ENOMEM;
    }

    // Set allocated pointer.
    *memptr = ptr.cast::<c_void>();

    0
}

#[cfg(all(test, feature = "std"))]
mod tests;
