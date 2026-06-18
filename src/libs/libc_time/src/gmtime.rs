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
/// Converts a calendar time (`time_t`) to a broken-down UTC time representation using an internal
/// static buffer.
///
/// # Parameters
///
/// - `timep`: Pointer to the calendar time to convert.
///
/// # Returns
///
/// On success, returns a pointer to an internal static `tm` structure. Returns a null pointer if
/// `timep` is null. The returned pointer is only valid until the next call to `gmtime`.
///
/// # Safety
///
/// This function is unsafe because it dereferences the raw pointer `timep` and accesses a shared
/// static buffer. The caller must ensure single-threaded access.
///
#[cfg_attr(not(feature = "std"), unsafe(no_mangle))]
pub unsafe extern "C" fn gmtime(timep: *const time_t) -> *mut tm {
    static mut GM_BUF: tm = tm::new();
    crate::gmtime_r::gmtime_r(timep, core::ptr::addr_of_mut!(GM_BUF))
}

//==================================================================================================
// Unit Tests
//==================================================================================================

#[cfg(all(test, feature = "std"))]
mod test {
    use super::gmtime;
    use crate::tm_struct::tm;
    use ::sysapi::sys_types::time_t;

    #[test]
    fn test_gmtime_returns_non_null() {
        let t: time_t = 0;
        let ret: *mut tm = unsafe { gmtime(&t) };
        assert!(!ret.is_null());
        assert_eq!(unsafe { (*ret).tm_year }, 70);
    }

    #[test]
    fn test_gmtime_null_input() {
        let ret: *mut tm = unsafe { gmtime(core::ptr::null()) };
        assert!(ret.is_null());
    }
}
