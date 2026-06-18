// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::tm_struct::tm;
use ::sysapi::sys_types::time_t;

//==================================================================================================
// Standalone Functions
//==================================================================================================

///
/// # Description
///
/// Converts a calendar time (`time_t`) to a broken-down local time representation.
///
/// In Nanvix (no timezone support), this function is equivalent to [`gmtime_r`].
///
/// # Parameters
///
/// - `timep`: Pointer to the calendar time to convert.
/// - `result`: Pointer to a `tm` structure where the broken-down time will be stored.
///
/// # Returns
///
/// On success, returns `result`. Returns a null pointer if `timep` or `result` is null.
///
/// # Safety
///
/// This function is unsafe because it dereferences the raw pointers `timep` and `result`.
///
#[cfg_attr(not(feature = "std"), unsafe(no_mangle))]
pub unsafe extern "C" fn localtime_r(timep: *const time_t, result: *mut tm) -> *mut tm {
    crate::gmtime_r::gmtime_r(timep, result)
}

//==================================================================================================
// Unit Tests
//==================================================================================================

#[cfg(all(test, feature = "std"))]
mod test {
    use super::localtime_r;
    use crate::tm_struct::tm;
    use ::sysapi::sys_types::time_t;

    #[test]
    fn test_localtime_r_epoch() {
        let t: time_t = 0;
        let mut result: tm = tm::new();
        let ret: *mut tm = unsafe { localtime_r(&t, &mut result) };
        assert!(!ret.is_null());
        assert_eq!(result.tm_year, 70);
        assert_eq!(result.tm_mon, 0);
        assert_eq!(result.tm_mday, 1);
    }

    #[test]
    fn test_localtime_r_known_date() {
        // 2000-01-01 00:00:00 UTC.
        let t: time_t = 946_684_800;
        let mut result: tm = tm::new();
        let ret: *mut tm = unsafe { localtime_r(&t, &mut result) };
        assert!(!ret.is_null());
        assert_eq!(result.tm_year, 100);
        assert_eq!(result.tm_mon, 0);
        assert_eq!(result.tm_mday, 1);
    }

    #[test]
    fn test_localtime_r_null_pointers() {
        let t: time_t = 0;
        let mut result: tm = tm::new();
        assert!(unsafe { localtime_r(core::ptr::null(), &mut result) }.is_null());
        assert!(unsafe { localtime_r(&t, core::ptr::null_mut()) }.is_null());
    }
}
