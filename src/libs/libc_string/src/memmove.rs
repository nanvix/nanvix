// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Lint Configuration
//==================================================================================================

#![forbid(clippy::cast_sign_loss)]

//==================================================================================================
// Imports
//==================================================================================================

use ::core::mem::align_of;
use ::sysapi::{
    ffi::{
        c_uchar,
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
/// Copies bytes in memory with overlapping areas.
///
/// This function copies `len` bytes from the memory area pointed to by `src` to the memory area
/// pointed to by `dest`. The memory areas may overlap: copying takes place as though the bytes in
/// `src` are first copied into a temporary array that does not overlap `src` or `dest`, and the
/// bytes are then copied from the temporary array to `dest`.
///
/// # Parameters
///
/// - `dest`: Pointer to the destination memory area where bytes will be copied.
/// - `src`: Pointer to the source memory area from which bytes will be copied.
/// - `len`: Number of bytes to copy.
///
/// # Return Value
///
/// This function returns the original destination pointer `dest`.
///
/// # Safety
///
/// This function is unsafe because:
/// - It performs raw pointer dereferencing and arithmetic.
/// - It performs unchecked writes to the memory region pointed to by `dest`.
///
/// It is safe to call this function if and only if the following conditions are met:
/// - `dest` points to a valid and writable memory region of at least `len` bytes.
/// - `src` points to a valid and readable memory region of at least `len` bytes.
/// - `len` does not exceed `isize::MAX`.
/// - `dest` and `src` are properly aligned for `c_uchar` access.
///
#[cfg_attr(not(feature = "std"), unsafe(no_mangle))]
pub unsafe extern "C" fn memmove(
    dest: *mut c_void,
    src: *const c_void,
    len: c_size_t,
) -> *mut c_void {
    debug_assert!(!dest.is_null(), "memmove(): null destination pointer");
    debug_assert!(!src.is_null(), "memmove(): null source pointer");
    debug_assert!((len as usize) < isize::MAX as usize, "memmove(): length too large");
    debug_assert!(
        (dest as usize).is_multiple_of(align_of::<c_uchar>()),
        "memmove(): destination pointer is not properly aligned"
    );

    let d: *mut c_uchar = dest.cast::<c_uchar>();
    let s: *const c_uchar = src.cast::<c_uchar>();
    let len: usize = len as usize;

    // Check for zero-length copy.
    if len == 0 {
        return dest;
    }

    // Check for self-copy.
    if core::ptr::eq(d, s) {
        return dest;
    }

    // Check whether or not to handle overlapping copy.
    if (d as usize) < (s as usize) || (d as usize) >= (s as usize + len) {
        // Non-overlapping or dest entirely before src: forward copy.
        let mut i: usize = 0;
        while i < len {
            *d.add(i) = *s.add(i);
            i += 1;
        }
    } else {
        // Overlapping and dest after src: backward copy.
        let mut i: usize = len;
        while i != 0 {
            i -= 1;
            *d.add(i) = *s.add(i);
        }
    }
    dest
}

//==================================================================================================
// Tests
//==================================================================================================

#[cfg(test)]
mod test;
