// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::tm_struct::tm;
use ::sysapi::ffi::c_char;

//==================================================================================================
// Standalone Functions
//==================================================================================================

///
/// # Description
///
/// Converts the broken-down time to a string representation using an internal static buffer.
///
/// # Parameters
///
/// - `timeptr`: Pointer to the broken-down time structure.
///
/// # Returns
///
/// On success, returns a pointer to the internal static buffer containing the formatted string.
/// The returned pointer is only valid until the next call to `asctime`. Returns a null pointer if
/// `timeptr` is null.
///
/// # Safety
///
/// This function is unsafe because it dereferences the raw pointer `timeptr` and accesses a shared
/// static buffer. The caller must ensure single-threaded access.
///
#[cfg_attr(not(feature = "std"), unsafe(no_mangle))]
pub unsafe extern "C" fn asctime(timeptr: *const tm) -> *mut c_char {
    static mut BUF: [c_char; 26] = [0; 26];
    crate::asctime_r::asctime_r(timeptr, core::ptr::addr_of_mut!(BUF).cast::<c_char>())
}

//==================================================================================================
// Unit Tests
//==================================================================================================

#[cfg(all(test, feature = "std"))]
mod test;
