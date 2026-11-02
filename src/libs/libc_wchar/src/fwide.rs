// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use ::libc_stdio::FILE;
use ::sysapi::ffi::c_int;

//==================================================================================================
// Standalone Functions
//==================================================================================================

///
/// # Description
///
/// Reports, and possibly sets, the orientation of the stream `stream`. A positive `mode` attempts
/// to make an unoriented stream wide-oriented, a negative `mode` attempts to make it byte-oriented,
/// and a zero `mode` only queries the current orientation. Once a stream is oriented, subsequent
/// calls do not change its orientation.
///
/// # Parameters
///
/// - `stream`: Pointer to the [`FILE`] stream to query.
/// - `mode`: Requested orientation: `> 0` for wide, `< 0` for byte, `0` to query.
///
/// # Return Value
///
/// A value greater than zero if the stream is wide-oriented, less than zero if it is byte-oriented,
/// and zero if it has no orientation.
///
/// # Safety
///
/// This function is unsafe because it dereferences raw pointers. The caller must ensure that
/// `stream` is either null or points to a valid, open [`FILE`] structure.
///
/// # References
///
/// - <https://pubs.opengroup.org/onlinepubs/9799919799/functions/fwide.html>
///
#[cfg_attr(not(feature = "std"), unsafe(no_mangle))]
pub unsafe extern "C" fn fwide(stream: *mut FILE, mode: c_int) -> c_int {
    if stream.is_null() {
        return 0;
    }

    if (*stream).orientation == 0 {
        if mode > 0 {
            (*stream).orientation = 1;
        } else if mode < 0 {
            (*stream).orientation = -1;
        }
    }

    (*stream).orientation
}

//==================================================================================================
// Tests
//==================================================================================================

#[cfg(all(test, feature = "std"))]
mod test;
