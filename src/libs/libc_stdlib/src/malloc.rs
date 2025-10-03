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
use ::syslog::{
    error,
    warn,
};

//==================================================================================================
// Standalone Functions
//==================================================================================================

///
/// # Description
///
/// Allocates memory.
///
/// This function allocates unused space in memory for an object whose size in bytes is specified by
/// `size` and whose value is unspecified.
///
/// # Parameters
///
/// - `size`: Number of bytes to allocate.
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
/// - https://pubs.opengroup.org/onlinepubs/9799919799/functions/malloc.html
///
#[cfg_attr(not(feature = "std"), unsafe(no_mangle))]
pub unsafe extern "C" fn malloc(size: c_size_t) -> *mut c_void {
    // Check for zero-size allocation.
    if size == 0 {
        // Zero-size allocations have implementation-defined behavior,
        // thus log a warning message and return null.
        warn!("malloc(): zero-size allocation (size={size:?})");
        set_errno(EINVAL);
        return null_mut();
    }

    // Allocate memory and check for errors.
    let ptr: *mut u8 = BlockHeader::alloc(size as usize, None);
    if ptr.is_null() {
        error!("malloc(): allocation failed (size={size:?})");
        set_errno(ENOMEM);
        return null_mut();
    }

    ptr.cast::<c_void>()
}

#[cfg(all(test, feature = "std"))]
mod tests {
    use super::malloc;
    use crate::set_errno;
    use ::sysapi::{
        errno::EINVAL,
        ffi::{
            c_int,
            c_void,
        },
        sys_types::c_size_t,
    };

    // Helper to read errno safely.
    fn get_errno() -> c_int {
        unsafe { *sysapi::errno::__errno_location() }
    }

    #[test]
    fn null_storage() {
        set_errno(0);
        let p = unsafe { malloc(0) };
        assert!(p.is_null());
        assert_eq!(get_errno(), EINVAL);
    }

    #[test]
    fn valid_allocation() {
        let size: c_size_t = 128;
        let p = unsafe { malloc(size) };
        assert!(!p.is_null());
        unsafe {
            crate::free(p);
        }
    }
}
