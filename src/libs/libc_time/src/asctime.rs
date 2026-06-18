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
mod test {
    use super::asctime;
    use crate::tm_struct::tm;

    #[test]
    fn test_asctime_returns_non_null() {
        let t: tm = tm {
            tm_sec: 0,
            tm_min: 0,
            tm_hour: 0,
            tm_mday: 1,
            tm_mon: 0,
            tm_year: 70,
            tm_wday: 4,
            tm_yday: 0,
            tm_isdst: 0,
        };
        let ret: *mut i8 = unsafe { asctime(&t) };
        assert!(!ret.is_null());
    }

    #[test]
    fn test_asctime_null_input() {
        let ret: *mut i8 = unsafe { asctime(core::ptr::null()) };
        assert!(ret.is_null());
    }
}
