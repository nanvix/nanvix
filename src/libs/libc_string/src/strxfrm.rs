// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::strlen::strlen;
use ::core::mem::align_of;
use ::sysapi::{
    ffi::c_char,
    sys_types::c_size_t,
};

//==================================================================================================
// Standalone Functions
//==================================================================================================

///
/// # Description
///
/// Transforms the null-terminated string `src` into a form whose byte-wise comparison with
/// `strcmp()` yields the same ordering that `strcoll()` would produce for the original strings, and
/// stores the result in `dest`.
///
/// Nanvix implements only the C/POSIX locale, in which the collating sequence is the numeric order
/// of the bytes. In that locale the transformation is the identity, so this implementation simply
/// copies `src` to `dest`, mirroring the relationship between `strcoll()` and `strcmp()`.
///
/// # Parameters
///
/// - `dest`: Destination buffer that receives the transformed string.
/// - `src`: Pointer to the null-terminated source string.
/// - `n`: Capacity of `dest`, in bytes, including the null terminator.
///
/// # Return Value
///
/// Returns the length of the transformed string, excluding the null terminator. If this value is
/// greater than or equal to `n`, truncation occurred.
///
/// If `n` is non-zero, this implementation always writes a null-terminated result to `dest`.
///
/// # Safety
///
/// This function is unsafe because:
/// - It performs raw pointer dereferencing and arithmetic.
/// - It reads from `src` until the first null terminator (not bounded by `n`).
/// - It writes up to `n` bytes to `dest` without bounds checking.
/// - If `n` is non-zero, `dest` must point to a valid writable buffer of at least `n` bytes.
/// - If bytes are copied, the `src` and `dest` regions must not overlap.
///
#[cfg_attr(not(feature = "std"), unsafe(no_mangle))]
pub unsafe extern "C" fn strxfrm(dest: *mut c_char, src: *const c_char, n: c_size_t) -> c_size_t {
    debug_assert!(!src.is_null(), "strxfrm(): null source pointer");
    debug_assert!(n == 0 || !dest.is_null(), "strxfrm(): null destination pointer");
    debug_assert!(
        (src as usize).is_multiple_of(align_of::<c_char>()),
        "strxfrm(): source pointer is not properly aligned"
    );
    debug_assert!(
        n == 0 || (dest as usize).is_multiple_of(align_of::<c_char>()),
        "strxfrm(): destination pointer is not properly aligned"
    );

    // SAFETY: the caller guarantees that `src` is a valid null-terminated string.
    let len: c_size_t = unsafe { strlen(src) };

    if n > 0 && !dest.is_null() {
        let copy_len: c_size_t = if len < n { len } else { n - 1 };

        // SAFETY: the caller guarantees that `dest` has room for at least `n` bytes, and `copy_len`
        // never exceeds `n - 1`, leaving space for the null terminator.
        unsafe {
            core::ptr::copy_nonoverlapping(src, dest, copy_len as usize);
            *dest.add(copy_len as usize) = 0;
        }
    }

    len
}

#[cfg(all(test, feature = "std"))]
mod test;
