// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::tm_struct::tm;
use ::sysapi::{
    ffi::c_char,
    sys_types::time_t,
};

//==================================================================================================
// Standalone Functions
//==================================================================================================

///
/// # Description
///
/// Converts a calendar time to a string representation of the form
/// `"Wed Jun 30 21:49:08 1993\n"` using a caller-provided buffer.
///
/// Equivalent to `asctime_r(localtime_r(timep, &tmp), buf)`.
///
/// # Parameters
///
/// - `timep`: Pointer to the calendar time to convert.
/// - `buf`: Pointer to a character buffer of at least 26 bytes.
///
/// # Returns
///
/// On success, returns `buf`. Returns a null pointer on error.
///
/// # Safety
///
/// This function is unsafe because it dereferences the raw pointers `timep` and `buf`. The caller
/// must ensure `buf` has room for at least 26 bytes.
///
#[cfg_attr(not(feature = "std"), unsafe(no_mangle))]
pub unsafe extern "C" fn ctime_r(timep: *const time_t, buf: *mut c_char) -> *mut c_char {
    if timep.is_null() || buf.is_null() {
        return core::ptr::null_mut();
    }

    let mut tmp: tm = tm::new();
    let ret: *mut tm = crate::localtime_r::localtime_r(timep, &mut tmp);
    if ret.is_null() {
        return core::ptr::null_mut();
    }

    crate::asctime_r::asctime_r(&tmp, buf)
}

//==================================================================================================
// Unit Tests
//==================================================================================================

#[cfg(all(test, feature = "std"))]
mod test;
