// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::block_header::BlockHeader;
use ::sysapi::{
    ffi::c_void,
    sys_types::c_size_t,
};

//==================================================================================================
// Standalone Functions
//==================================================================================================

///
/// # Description
///
/// Returns the number of usable bytes in the allocation pointed to by `ptr`.
///
/// This function returns the size (in bytes) of the memory block that was originally requested
/// when the allocation function (e.g., `malloc`, `calloc`, `realloc`, `aligned_alloc`, or
/// `posix_memalign`) returned the pointer `ptr`. The returned size is the logical usable size and
/// may be less than or equal to the actual size reserved internally by the allocator.
///
/// If `ptr` is a null pointer, this function returns `0`.
///
/// # Parameters
///
/// - `ptr`: Pointer to a memory block previously allocated by one of the allocation functions.
///
/// # Returns
///
/// On success, this function returns the usable size of the allocation in bytes. If `ptr` is a
/// null pointer, it returns `0`.
///
/// # Safety
///
/// This function is unsafe because it dereferences internal allocator metadata. The caller must
/// ensure that `ptr` either is null or was returned by a successful allocation from this library
/// and has not yet been freed. Passing any other pointer results in undefined behavior.
///
/// # Notes
///
/// This is a non-standard (GNU extension–like) function provided for diagnostic and optimization
/// purposes. Applications should not rely on the returned size for writing beyond the originally
/// requested bounds.
///
/// # References
///
/// https://www.man7.org/linux/man-pages/man3/malloc_usable_size.3.html
///
#[cfg_attr(not(feature = "std"), unsafe(no_mangle))]
pub unsafe extern "C" fn malloc_usable_size(ptr: *mut c_void) -> c_size_t {
    // Check for null pointer.
    if ptr.is_null() {
        return 0;
    }

    let size: usize = BlockHeader::usable_size(ptr.cast::<u8>());

    cfg_if::cfg_if! {
        if #[cfg(target_pointer_width = "32")] {
        // On all 32-bit platforms, the following cast is safe.
            #[allow(clippy::cast_possible_truncation)]
            size as c_size_t
        } else if #[cfg(test)] {
            // When testing, the following cast is acceptable.
            size.try_into().unwrap_or(c_size_t::MAX)
        } else {
            compile_error!("malloc_usable_size is only supported on 32-bit platforms");
        }
    }
}

#[cfg(all(test, feature = "std"))]
mod tests {
    use super::malloc_usable_size;
    use ::sysapi::{
        ffi::c_void,
        sys_types::c_size_t,
    };

    #[test]
    fn null_pointer_returns_zero() {
        let sz = unsafe { malloc_usable_size(core::ptr::null_mut()) };
        assert_eq!(sz, 0);
    }

    #[test]
    fn malloc_and_query_size() {
        let req: c_size_t = 128;
        let p = unsafe { crate::malloc(req) };
        assert!(!p.is_null());
        let sz = unsafe { malloc_usable_size(p) };
        assert!(sz as usize >= req as usize);
        assert_eq!(sz, req, "usable size should be exactly requested size");
        unsafe { crate::free(p) };
    }

    #[test]
    fn realloc_grow_and_query() {
        let p = unsafe { crate::malloc(64) };
        assert!(!p.is_null());
        let p2 = unsafe { crate::realloc(p, 200) };
        assert!(!p2.is_null());
        let sz2 = unsafe { malloc_usable_size(p2) };
        assert!(sz2 as usize >= 200);
        assert_eq!(sz2, 200);
        unsafe { crate::free(p2) };
    }

    #[test]
    fn aligned_alloc_and_query() {
        let p = unsafe { crate::aligned_alloc(128, 300) };
        assert!(!p.is_null());
        let sz = unsafe { malloc_usable_size(p) };
        assert!(sz as usize >= 300);
        assert_eq!(sz, 300);
        unsafe { crate::free(p) };
    }
}
