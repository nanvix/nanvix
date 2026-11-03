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
    ffi::c_char,
    sys_types::c_size_t,
};

//==================================================================================================
// Standalone Functions
//==================================================================================================

///
/// # Description
///
/// Size-bounded string copying.
///
/// This function copies up to `size - 1` characters from the string `src` to `dest`,
/// null-terminating the result if `size` is not zero. Unlike `strncpy`, `strlcpy` always
/// null-terminates the result (as long as `size > 0`).
///
/// # Parameters
///
/// - `dest`: Pointer to the destination buffer.
/// - `src`: Pointer to the source null-terminated string.
/// - `size`: Size of the destination buffer.
///
/// # Return Value
///
/// Returns the total length of the string that would have been created if there was unlimited
/// space (i.e., `strlen(src)`). If the return value is >= `size`, truncation occurred.
///
/// # Safety
///
/// This function is unsafe because:
/// - It performs raw pointer dereferencing and arithmetic.
/// - It writes to the memory region pointed to by `dest` without bounds checking.
///
#[cfg_attr(not(feature = "std"), unsafe(no_mangle))]
pub unsafe extern "C" fn strlcpy(
    dest: *mut c_char,
    src: *const c_char,
    size: c_size_t,
) -> c_size_t {
    debug_assert!(!dest.is_null(), "strlcpy(): null destination pointer");
    debug_assert!(!src.is_null(), "strlcpy(): null source pointer");
    debug_assert!(
        (dest as usize).is_multiple_of(align_of::<c_char>()),
        "strlcpy(): destination pointer is not properly aligned"
    );
    debug_assert!(
        (src as usize).is_multiple_of(align_of::<c_char>()),
        "strlcpy(): source pointer is not properly aligned"
    );

    let size: usize = size as usize;

    // Calculate strlen(src) using c_size_t to avoid usize->c_size_t cast.
    let mut src_len: c_size_t = 0;
    while *src.add(src_len as usize) != 0 {
        src_len += 1;
    }

    if size > 0 {
        let copy_len: usize = if (src_len as usize) < size {
            src_len as usize
        } else {
            size - 1
        };
        let mut i: usize = 0;
        while i < copy_len {
            *dest.add(i) = *src.add(i);
            i += 1;
        }
        *dest.add(copy_len) = 0 as c_char;
    }

    src_len
}

#[cfg(all(test, feature = "std"))]
mod test;
