// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

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
/// Copies at most `n` bytes of a string and returns a pointer to its end.
///
/// This function copies at most `n` bytes from the null-terminated string pointed to by `src` into
/// the buffer pointed to by `dest`, exactly like `strncpy()` (padding `dest` with null bytes when
/// `src` is shorter than `n`), but returns a pointer to the first null byte written in `dest`, or
/// to `dest + n` when no null byte was written. It is a POSIX extension convenient for chaining
/// successive copies.
///
/// Note that, as with `strncpy()`, if there is no null byte among the first `n` bytes of `src`, the
/// string written to `dest` is not null-terminated.
///
/// # Parameters
///
/// - `dest`: Destination buffer where the bytes will be written.
/// - `src`: Pointer to the null-terminated source string.
/// - `n`: Maximum number of bytes to write.
///
/// # Return Value
///
/// Returns a pointer to the first null byte written in `dest`, or `dest + n` if no null byte was
/// written.
///
/// # Safety
///
/// This function is unsafe because:
/// - It performs raw pointer dereferencing and arithmetic.
/// - It performs unchecked writes to the memory region pointed to by `dest`.
///
/// It is safe to call this function if and only if the following conditions are met:
/// - `src` points to a valid, null-terminated string.
/// - `dest` points to a writable memory region of at least `n` bytes.
/// - The source and destination regions do not overlap.
///
/// Violating any of these conditions results in undefined behavior.
///
#[cfg_attr(not(feature = "std"), unsafe(no_mangle))]
pub unsafe extern "C" fn stpncpy(
    dest: *mut c_char,
    src: *const c_char,
    n: c_size_t,
) -> *mut c_char {
    debug_assert!(!dest.is_null(), "stpncpy(): null destination pointer");
    debug_assert!(!src.is_null(), "stpncpy(): null source pointer");
    debug_assert!(
        (dest as usize).is_multiple_of(align_of::<c_char>()),
        "stpncpy(): destination pointer is not properly aligned"
    );
    debug_assert!(
        (src as usize).is_multiple_of(align_of::<c_char>()),
        "stpncpy(): source pointer is not properly aligned"
    );

    let n: usize = n as usize;
    let mut i: usize = 0;

    // Copy bytes from src until null terminator or n bytes.
    while i < n {
        let c: c_char = *src.add(i);
        *dest.add(i) = c;
        if c == 0 {
            break;
        }
        i += 1;
    }

    // The end pointer is the position of the first null byte written, or dest + n when src is at
    // least n bytes long (no null byte was written).
    let end: *mut c_char = dest.add(i);

    // Pad the remainder of the buffer with null bytes.
    while i < n {
        *dest.add(i) = 0 as c_char;
        i += 1;
    }

    end
}

#[cfg(all(test, feature = "std"))]
mod test;
