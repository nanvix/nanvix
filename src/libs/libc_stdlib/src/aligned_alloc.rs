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
use ::syslog::error;

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
        error!("aligned_alloc(): zero-size allocation (alignment={alignment:?}, size={size:?})");
        set_errno(EINVAL);
        return null_mut();
    }

    // Check for null alignment.
    if alignment == 0 {
        // Zero-size alignments have implementation-defined behavior,
        // thus log a warning message and return null.
        error!("aligned_alloc(): zero-size alignment (alignment={alignment:?}, size={size:?})");
        set_errno(EINVAL);
        return null_mut();
    }

    // Allocate memory and check for errors.
    let ptr: *mut u8 = BlockHeader::alloc(size as usize, Some(alignment as usize));
    if ptr.is_null() {
        error!("aligned_alloc(): allocation failed (alignment={alignment:?}, size={size:?})");
        set_errno(ENOMEM);
        return null_mut();
    }

    ptr.cast::<c_void>()
}

#[cfg(all(test, feature = "std"))]
mod tests {
    use super::aligned_alloc;
    use crate::set_errno;
    use ::sysapi::{
        errno::{
            EINVAL,
            ENOMEM,
        },
        ffi::c_int,
        sys_types::c_size_t,
    };

    // Helper to read errno safely.
    fn get_errno() -> c_int {
        unsafe { *sysapi::errno::__errno_location() }
    }

    #[test]
    fn zero_size_allocation() {
        set_errno(0);
        let p = unsafe { aligned_alloc(64, 0) };
        assert!(p.is_null());
        assert_eq!(get_errno(), EINVAL);
    }

    #[test]
    fn null_alignment() {
        set_errno(0);
        let p = unsafe { aligned_alloc(0, 64) };
        assert!(p.is_null());
        assert_eq!(get_errno(), EINVAL);
    }

    #[test]
    fn valid_allocation_multiple() {
        let alignment: c_size_t = 64;
        let size: c_size_t = 256; // multiple of alignment
        let p = unsafe { aligned_alloc(alignment, size) } as *mut u8;
        assert!(!p.is_null());
        let addr = p as usize;
        assert_eq!(addr & (alignment as usize - 1), 0, "pointer {addr:#x} not {alignment}-aligned");
        unsafe {
            crate::free(p.cast());
        }
    }

    #[test]
    fn valid_allocation_non_multiple() {
        let alignment: c_size_t = 64;
        let size: c_size_t = 130; // not a multiple of alignment
        let p = unsafe { aligned_alloc(alignment, size) } as *mut u8;
        assert!(!p.is_null());
        let addr = p as usize;
        assert_eq!(addr & (alignment as usize - 1), 0, "pointer {addr:#x} not {alignment}-aligned");
        unsafe {
            crate::free(p.cast());
        }
    }
}
