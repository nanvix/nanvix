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
        error!("posix_memalign(): null storage (alignment={alignment:?}, size={size:?})");
        return EINVAL;
    }

    let size: usize = size as usize;
    let alignment: usize = alignment as usize;

    // Check for zero-size allocation.
    if size == 0 {
        // Zero-size allocations have implementation-defined behavior,
        // thus log a warning message and return error.
        error!("posix_memalign(): zero-size allocation (alignment={alignment:?}, size={size:?})");
        return EINVAL;
    }

    // Check for null alignment.
    if alignment == 0 {
        // Zero-size alignments have implementation-defined behavior,
        // thus log a warning message and return error.
        error!("posix_memalign(): zero-size alignment (alignment={alignment:?}, size={size:?})");
        return EINVAL;
    }

    // Check for invalid alignment.
    if !alignment.is_multiple_of(size_of::<*mut c_void>()) || !alignment.is_power_of_two() {
        error!("posix_memalign(): invalid alignment (alignment={alignment:?}, size={size:?})");
        return EINVAL;
    }

    // Allocate memory and check for errors.
    let ptr: *mut u8 = BlockHeader::alloc(size, Some(alignment));
    if ptr.is_null() {
        error!("posix_memalign(): allocation failed (alignment={alignment:?}, size={size:?})");
        return ENOMEM;
    }

    // Set allocated pointer.
    *memptr = ptr.cast::<c_void>();

    0
}

#[cfg(all(test, feature = "std"))]
mod tests {
    use super::posix_memalign;
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
        let r = unsafe { super::posix_memalign(core::ptr::null_mut(), 64, 128) };
        assert_eq!(r, EINVAL);
        assert_eq!(get_errno(), 0);
    }

    #[test]
    fn zero_size_allocation() {
        set_errno(0);
        let mut out: *mut c_void = core::ptr::null_mut();
        let r = unsafe { super::posix_memalign(&mut out as *mut *mut c_void, 64, 0) };
        assert_eq!(r, EINVAL);
        assert_eq!(get_errno(), 0);
    }

    #[test]
    fn zero_size_alignment() {
        set_errno(0);
        let mut out: *mut c_void = core::ptr::null_mut();
        let r = unsafe { super::posix_memalign(&mut out as *mut *mut c_void, 0, 128) };
        assert_eq!(r, EINVAL);
        assert_eq!(get_errno(), 0);
    }

    #[test]
    fn non_power_of_two_alignment() {
        set_errno(0);
        let mut out: *mut c_void = core::ptr::null_mut();
        let r = unsafe { super::posix_memalign(&mut out as *mut *mut c_void, 24, 128) };
        assert_eq!(r, EINVAL);
        assert_eq!(get_errno(), 0);
    }

    #[test]
    fn alignment_not_multiple_of_pointer_size() {
        set_errno(0);
        let mut out: *mut c_void = core::ptr::null_mut();
        let r = unsafe { super::posix_memalign(&mut out as *mut *mut c_void, 20, 128) };
        assert_eq!(r, EINVAL);
        assert_eq!(get_errno(), 0);
    }

    #[test]
    fn valid_allocation() {
        let mut out: *mut c_void = core::ptr::null_mut();
        let alignment: c_size_t = 128;
        let size: c_size_t = 512;
        let ret = unsafe { super::posix_memalign(&mut out as *mut *mut c_void, alignment, size) };
        assert_eq!(ret, 0);
        assert!(!out.is_null());
        let addr = out as usize;
        assert_eq!(addr & (alignment as usize - 1), 0);
        unsafe {
            crate::free(out);
        }
    }
}
