// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use ::sysapi::ffi::c_int;

//==================================================================================================
// Standalone Functions
//==================================================================================================

///
/// # Description
///
/// Reads the next character from standard input. Equivalent to
/// [`fgetc`](`crate::fgetc::fgetc`) called with [`crate::stdin`].
///
/// # Returns
///
/// The next character as an `unsigned char` promoted to [`c_int`], or `EOF` (`-1`) on end-of-file
/// or error.
///
/// # Safety
///
/// This function is unsafe because it accesses the global standard input stream.
///
/// # References
///
/// - <https://pubs.opengroup.org/onlinepubs/9799919799/functions/getchar.html>
///
#[cfg_attr(not(feature = "std"), unsafe(no_mangle))]
pub unsafe extern "C" fn getchar() -> c_int {
    // SAFETY: stdin() returns a valid pointer to the standard input FILE.
    unsafe { crate::fgetc::fgetc(crate::stdin()) }
}
