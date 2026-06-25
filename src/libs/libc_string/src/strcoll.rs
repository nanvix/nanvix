// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::strcmp::strcmp;
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
/// Compares two null-terminated strings according to the collating sequence of the current locale.
///
/// Nanvix implements only the C/POSIX locale, in which the collating sequence is the numeric order
/// of the bytes. In that locale `strcoll()` is therefore equivalent to `strcmp()`, to which this
/// implementation delegates.
///
/// # Parameters
///
/// - `s1`: Pointer to the first null-terminated string.
/// - `s2`: Pointer to the second null-terminated string.
///
/// # Return Value
///
/// Returns an integer less than, equal to, or greater than zero if `s1` is found, respectively, to
/// be less than, to match, or be greater than `s2`.
///
/// # Safety
///
/// This function is unsafe because:
/// - It performs raw pointer dereferencing and arithmetic.
/// - It reads from the memory regions pointed to by `s1` and `s2` without bounds checking.
///
#[cfg_attr(not(feature = "std"), unsafe(no_mangle))]
pub unsafe extern "C" fn strcoll(s1: *const c_char, s2: *const c_char) -> c_int {
    // SAFETY: the caller upholds the same contract that strcmp() requires.
    unsafe { strcmp(s1, s2) }
}
