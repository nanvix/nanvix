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
/// Size-bounded string concatenation.
///
/// This function appends the string `src` to the end of `dest`. It will append at most
/// `size - strlen(dest) - 1` bytes, null-terminating the result.
///
/// # Parameters
///
/// - `dest`: Pointer to the destination null-terminated string.
/// - `src`: Pointer to the source null-terminated string.
/// - `size`: Full size of the destination buffer.
///
/// # Return Value
///
/// Returns `strlen(dest) + strlen(src)` (initial lengths), representing the total length of the
/// string it tried to create. If the return value is >= `size`, truncation occurred.
///
/// # Safety
///
/// This function is unsafe because:
/// - It performs raw pointer dereferencing and arithmetic.
/// - It writes to the memory region pointed to by `dest` without bounds checking.
///
#[cfg_attr(not(feature = "std"), unsafe(no_mangle))]
pub unsafe extern "C" fn strlcat(
    dest: *mut c_char,
    src: *const c_char,
    size: c_size_t,
) -> c_size_t {
    debug_assert!(!dest.is_null(), "strlcat(): null destination pointer");
    debug_assert!(!src.is_null(), "strlcat(): null source pointer");
    debug_assert!(
        (dest as usize).is_multiple_of(align_of::<c_char>()),
        "strlcat(): destination pointer is not properly aligned"
    );
    debug_assert!(
        (src as usize).is_multiple_of(align_of::<c_char>()),
        "strlcat(): source pointer is not properly aligned"
    );

    let size: usize = size as usize;

    // Find current length of dest (up to size), using c_size_t to avoid truncation casts.
    let mut dest_len: c_size_t = 0;
    while (dest_len as usize) < size && *dest.add(dest_len as usize) != 0 {
        dest_len += 1;
    }

    // Calculate strlen(src).
    let mut src_len: c_size_t = 0;
    while *src.add(src_len as usize) != 0 {
        src_len += 1;
    }

    // If dest_len >= size, dest is not null-terminated within size.
    if (dest_len as usize) >= size {
        return dest_len + src_len;
    }

    // Append at most size - dest_len - 1 bytes from src.
    let remaining: usize = size - (dest_len as usize) - 1;
    let copy_len: usize = if (src_len as usize) < remaining {
        src_len as usize
    } else {
        remaining
    };
    let mut i: usize = 0;
    while i < copy_len {
        *dest.add((dest_len as usize) + i) = *src.add(i);
        i += 1;
    }
    *dest.add((dest_len as usize) + copy_len) = 0 as c_char;

    dest_len + src_len
}

#[cfg(all(test, feature = "std"))]
mod test;
