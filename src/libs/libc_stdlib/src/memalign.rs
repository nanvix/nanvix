// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::{
    aligned_alloc::aligned_alloc,
    set_errno,
};
use ::core::ptr::null_mut;
use ::sysapi::{
    errno::EINVAL,
    ffi::c_void,
    sys_types::c_size_t,
};
use ::syslog::warn;

//==================================================================================================
// Standalone Functions
//==================================================================================================

///
/// # Description
///
/// Allocates `size` bytes of memory aligned to `alignment`, returning a pointer that may be
/// released with [`crate::free()`].
///
/// `memalign()` is the legacy SVID / BSD alignment allocator. Unlike `aligned_alloc()` it does
/// not require `size` to be a multiple of `alignment`; the only constraint enforced here is that
/// `alignment` is a power of two, which the backing allocator requires.
///
/// # Parameters
///
/// - `alignment`: Alignment in bytes. Must be a power of two.
/// - `size`: Size in bytes.
///
/// # Returns
///
/// On success, this function returns a pointer to the allocated memory. On failure, it returns a
/// null pointer and sets `errno` to indicate the error.
///
/// # Notes
///
/// Nanvix builds newlib with `-DMALLOC_PROVIDED`, so the C runtime's allocation primitives come
/// from this crate rather than newlib. `memalign()` is provided here for the same reason: it is
/// referenced by libstdc++'s aligned `operator new` (`new_opa.cc`) and must resolve against the
/// embedded runtime when extension `.so`s are loaded.
///
/// The allocation itself is delegated to [`aligned_alloc()`], which shares the same backing
/// allocator. `memalign()` keeps its own argument validation up front so that an invalid
/// (non-power-of-two) alignment fails with `EINVAL` -- the error `memalign()` is specified to
/// return -- rather than the `ENOMEM` that the allocator would surface for the same input, and so
/// that diagnostics are attributed to `memalign()`.
///
/// # Safety
///
/// This function is unsafe because it interacts with the global memory allocator.
///
/// # References
///
/// - https://man7.org/linux/man-pages/man3/posix_memalign.3.html
///
#[cfg_attr(not(feature = "std"), unsafe(no_mangle))]
pub unsafe extern "C" fn memalign(alignment: c_size_t, size: c_size_t) -> *mut c_void {
    // Check for zero-size allocation.
    if size == 0 {
        // Zero-size allocations have implementation-defined behavior,
        // thus log a warning message and return null.
        warn!("memalign(): zero-size allocation (alignment={alignment:?}, size={size:?})");
        set_errno(EINVAL);
        return null_mut();
    }

    // Check for null alignment.
    if alignment == 0 {
        // Zero-size alignments have implementation-defined behavior,
        // thus log a warning message and return null.
        warn!("memalign(): zero-size alignment (alignment={alignment:?}, size={size:?})");
        set_errno(EINVAL);
        return null_mut();
    }

    // Check for invalid alignment. memalign() requires a power-of-two alignment and is specified
    // to fail with EINVAL otherwise; validating here (instead of letting the allocator reject it
    // with ENOMEM) preserves that contract.
    if !(alignment as usize).is_power_of_two() {
        warn!("memalign(): invalid alignment (alignment={alignment:?}, size={size:?})");
        set_errno(EINVAL);
        return null_mut();
    }

    // Delegate the allocation to aligned_alloc(), which shares the same backing allocator. The
    // arguments are already validated above, so any failure here is a genuine out-of-memory
    // condition for which aligned_alloc() sets errno appropriately.
    aligned_alloc(alignment, size)
}

#[cfg(all(test, feature = "std"))]
mod tests {
    use super::memalign;
    use crate::set_errno;
    use ::sysapi::{
        errno::EINVAL,
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
        let p = unsafe { memalign(64, 0) };
        assert!(p.is_null());
        assert_eq!(get_errno(), EINVAL);
    }

    #[test]
    fn null_alignment() {
        set_errno(0);
        let p = unsafe { memalign(0, 64) };
        assert!(p.is_null());
        assert_eq!(get_errno(), EINVAL);
    }

    #[test]
    fn non_power_of_two_alignment() {
        set_errno(0);
        let p = unsafe { memalign(24, 128) };
        assert!(p.is_null());
        assert_eq!(get_errno(), EINVAL);
    }

    #[test]
    fn valid_allocation() {
        let alignment: c_size_t = 128;
        let size: c_size_t = 130; // not a multiple of alignment
        let p = unsafe { memalign(alignment, size) } as *mut u8;
        assert!(!p.is_null());
        let addr = p as usize;
        assert_eq!(addr & (alignment as usize - 1), 0, "pointer {addr:#x} not {alignment}-aligned");
        unsafe {
            crate::free(p.cast());
        }
    }
}
