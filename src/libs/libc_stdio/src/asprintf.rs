// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use ::sysapi::ffi::{
    c_char,
    c_int,
};

//==================================================================================================
// Standalone Functions
//==================================================================================================

///
/// # Description
///
/// Formats output according to `fmt` and the trailing variadic arguments, allocating a buffer large
/// enough to hold the result (including the terminating null byte). This is a variadic wrapper
/// around [`vasprintf`](`crate::vasprintf::vasprintf`).
///
/// # Parameters
///
/// - `strp`: Output pointer that receives the address of the allocated, formatted string.
/// - `fmt`: Pointer to a null-terminated printf format string.
/// - `...`: Arguments matching the format specifiers in `fmt`.
///
/// # Returns
///
/// The number of characters written (excluding the terminating null byte) on success, or `-1` on
/// error.
///
/// # Safety
///
/// This function is unsafe because it dereferences raw pointers. The caller must ensure that:
/// - `strp` points to a writable `char *` location.
/// - `fmt` points to a valid, null-terminated format string.
/// - The variadic arguments match the format specifiers.
///
/// # References
///
/// - <https://man7.org/linux/man-pages/man3/asprintf.3.html>
///
#[cfg_attr(not(feature = "std"), unsafe(no_mangle))]
pub unsafe extern "C" fn asprintf(strp: *mut *mut c_char, fmt: *const c_char, args: ...) -> c_int {
    // SAFETY: forwarding the variadic argument list to vasprintf.
    unsafe { crate::vasprintf::vasprintf(strp, fmt, args) }
}
