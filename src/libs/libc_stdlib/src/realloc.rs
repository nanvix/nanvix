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
use ::syslog::error;

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
        error!("realloc(): reallocation failed (ptr={ptr:?}, size={size:?})");
        set_errno(ENOMEM);
        return null_mut();
    }

    new_ptr.cast::<c_void>()
}

#[cfg(all(test, feature = "std"))]
mod tests {
    use super::realloc;
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
    fn realloc_null_behaves_like_malloc() {
        set_errno(0);
        let size: c_size_t = 128;
        let p = unsafe { realloc(core::ptr::null_mut(), size) };
        assert!(!p.is_null());
        assert_eq!(get_errno(), 0);
        unsafe {
            crate::free(p);
        }
    }

    #[test]
    fn realloc_zero_behaves_like_free() {
        let size: c_size_t = 128;
        let p = unsafe { realloc(core::ptr::null_mut(), size) };
        assert!(!p.is_null());
        set_errno(0);
        let p2 = unsafe { realloc(p, 0) };
        assert!(p2.is_null());
        assert_eq!(get_errno(), 0);
    }

    #[test]
    fn realloc_grow_and_shrink() {
        // Allocate initial block.
        let initial: c_size_t = 32;
        let p = unsafe { realloc(core::ptr::null_mut(), initial) } as *mut u8;
        assert!(!p.is_null());
        unsafe {
            for i in 0..initial {
                p.add(i).write(i as u8);
            }
        }
        // Grow block.
        let grown: c_size_t = 64;
        let p2 = unsafe { realloc(p as *mut c_void, grown) } as *mut u8;
        assert!(!p2.is_null());
        unsafe {
            for i in 0..initial {
                assert_eq!(p2.add(i).read(), i as u8, "realloc data mismatch at {i} after grow");
            }
            for i in initial..grown {
                p2.add(i).write(i as u8);
            }
        }
        // Shrink block.
        let shrunk: c_size_t = 16;
        let p3 = unsafe { realloc(p2 as *mut c_void, shrunk) } as *mut u8;
        assert!(!p3.is_null());
        unsafe {
            for i in 0..shrunk {
                assert_eq!(p3.add(i).read(), i as u8, "realloc data mismatch at {i} after shrink");
            }
        }
        unsafe {
            crate::free(p3 as *mut c_void);
        }
    }
}
