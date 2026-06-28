// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::{
    mktime::mktime,
    tm_struct::tm,
};
use ::sysapi::sys_types::time_t;

//==================================================================================================
// Standalone Functions
//==================================================================================================

///
/// # Description
///
/// Converts a broken-down time structure, interpreted as Coordinated Universal Time (UTC), to a
/// calendar time (`time_t`). The `tm` structure is normalized so that its fields are within their
/// proper ranges.
///
/// Nanvix keeps all time in UTC and applies no timezone offset, so `timegm` is equivalent to
/// [`mktime`].
///
/// # Parameters
///
/// - `timeptr`: Pointer to the broken-down time to convert.
///
/// # Returns
///
/// On success, the calendar time in seconds since the epoch. On error, `(time_t)(-1)`.
///
/// # Safety
///
/// This function is unsafe because it dereferences and writes to the raw pointer `timeptr`.
///
/// # References
///
/// - <https://www.man7.org/linux/man-pages/man3/timegm.3.html>
///
#[cfg_attr(not(feature = "std"), unsafe(no_mangle))]
pub unsafe extern "C" fn timegm(timeptr: *mut tm) -> time_t {
    mktime(timeptr)
}

//==================================================================================================
// Unit Tests
//==================================================================================================

#[cfg(all(test, feature = "std"))]
mod test {
    use super::timegm;
    use crate::tm_struct::tm;
    use ::sysapi::sys_types::time_t;

    #[test]
    fn test_timegm_epoch() {
        let mut t: tm = tm::new();
        t.tm_year = 70;
        t.tm_mon = 0;
        t.tm_mday = 1;
        let result: time_t = unsafe { timegm(&mut t) };
        assert_eq!(result, 0);
    }

    #[test]
    fn test_timegm_known_date() {
        // 2000-01-01 00:00:00 UTC.
        let mut t: tm = tm::new();
        t.tm_year = 100;
        t.tm_mon = 0;
        t.tm_mday = 1;
        let result: time_t = unsafe { timegm(&mut t) };
        assert_eq!(result, 946_684_800);
    }

    #[test]
    fn test_timegm_normalizes() {
        // 1970-01-01 00:00:90 (90 seconds should normalize to 00:01:30).
        let mut t: tm = tm::new();
        t.tm_year = 70;
        t.tm_mon = 0;
        t.tm_mday = 1;
        t.tm_sec = 90;
        let result: time_t = unsafe { timegm(&mut t) };
        assert_eq!(result, 90);
        assert_eq!(t.tm_min, 1);
        assert_eq!(t.tm_sec, 30);
    }

    #[test]
    fn test_timegm_null() {
        let result: time_t = unsafe { timegm(core::ptr::null_mut()) };
        assert_eq!(result, -1);
    }
}
