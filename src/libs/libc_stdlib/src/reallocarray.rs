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
mod tests {
    use super::reallocarray;
    use crate::set_errno;
    use ::sysapi::{
        errno::ENOMEM,
        ffi::{
            c_int,
            c_void,
        },
        sys_types::c_size_t,
    };

    fn get_errno() -> c_int {
        unsafe { *::sysapi::errno::__errno_location() }
    }

    #[test]
    fn reallocarray_allocates_product_size() {
        let ptr: *mut c_void = unsafe { reallocarray(core::ptr::null_mut(), 4, 16) };
        assert!(!ptr.is_null());
        assert_eq!(unsafe { crate::malloc_usable_size(ptr) }, 64);
        unsafe { crate::free(ptr) };
    }

    #[test]
    fn reallocarray_overflow_fails() {
        set_errno(0);
        let ptr: *mut c_void = unsafe { reallocarray(core::ptr::null_mut(), c_size_t::MAX, 2) };
        assert!(ptr.is_null());
        assert_eq!(get_errno(), ENOMEM);
    }
}
