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
/// Converts a calendar time to a string representation using an internal static buffer.
///
/// Equivalent to `asctime(localtime(timep))`.
///
/// # Parameters
///
/// - `timep`: Pointer to the calendar time to convert.
///
/// # Returns
///
/// On success, returns a pointer to an internal static buffer containing the formatted string.
/// The returned pointer is only valid until the next call to `ctime`. Returns a null pointer on
/// error.
///
/// # Safety
///
/// This function is unsafe because it dereferences the raw pointer `timep` and accesses shared
/// static buffers. The caller must ensure single-threaded access.
///
#[cfg_attr(not(feature = "std"), unsafe(no_mangle))]
pub unsafe extern "C" fn ctime(timep: *const time_t) -> *mut c_char {
    if timep.is_null() {
        return core::ptr::null_mut();
    }

    static mut BUF: [c_char; 26] = [0; 26];

    let mut tmp: tm = tm::new();
    let ret: *mut tm = crate::localtime_r::localtime_r(timep, &mut tmp);
    if ret.is_null() {
        return core::ptr::null_mut();
    }

    crate::asctime_r::asctime_r(&tmp, core::ptr::addr_of_mut!(BUF).cast::<c_char>())
}

//==================================================================================================
// Unit Tests
//==================================================================================================

#[cfg(all(test, feature = "std"))]
mod test {
    use super::ctime;
    use ::sysapi::sys_types::time_t;

    #[test]
    fn test_ctime_returns_non_null() {
        let t: time_t = 0;
        let ret: *mut i8 = unsafe { ctime(&t) };
        assert!(!ret.is_null());
    }

    #[test]
    fn test_ctime_null_input() {
        let ret: *mut i8 = unsafe { ctime(core::ptr::null()) };
        assert!(ret.is_null());
    }
}
