// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use ::sysapi::{
    ffi::{
        c_char,
        c_int,
    },
    sys_types::c_size_t,
};

//==================================================================================================
// Standalone Functions
//==================================================================================================

///
/// # Description
///
/// Writes at most `size - 1` characters of formatted output to `buf`, followed by a null
/// terminator. This is a variadic wrapper around [`crate::vsnprintf::vsnprintf`].
///
/// # Parameters
///
/// - `buf`: Pointer to the destination buffer.
/// - `size`: Size of the destination buffer in bytes.
/// - `fmt`: Pointer to a null-terminated printf format string.
/// - `...`: Arguments matching the format specifiers in `fmt`.
///
/// # Returns
///
/// The number of characters that would have been written (excluding the null terminator) had the
/// buffer been large enough. A return value of `size` or more indicates truncation.
///
/// # Safety
///
/// This function is unsafe because it dereferences raw pointers. The caller must ensure that:
/// - `buf` points to a writable buffer of at least `size` bytes (when `size > 0`).
/// - `fmt` points to a valid, null-terminated format string.
/// - The variadic arguments match the format specifiers.
///
/// # References
///
/// - <https://pubs.opengroup.org/onlinepubs/9799919799/functions/snprintf.html>
///
#[cfg_attr(not(feature = "std"), unsafe(no_mangle))]
pub unsafe extern "C" fn snprintf(
    buf: *mut c_char,
    size: c_size_t,
    fmt: *const c_char,
    args: ...
) -> c_int {
    // SAFETY: forwarding the variadic argument list to vsnprintf.
    unsafe { crate::vsnprintf::vsnprintf(buf, size, fmt, args) }
}
