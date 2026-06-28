// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Lint Configuration
//==================================================================================================

#![allow(clippy::cast_possible_truncation)]
#![allow(clippy::cast_possible_wrap)]

//==================================================================================================
// Imports
//==================================================================================================

use crate::tm_struct::{
    self,
    tm,
    DAYS_PER_100_YEARS,
    DAYS_PER_4_YEARS,
    DAYS_PER_5_MONTHS,
    DAYS_PER_COMMON_YEAR,
    DAYS_PER_ERA,
    EPOCH_DAYS_OFFSET,
    SECS_PER_DAY,
    SECS_PER_HOUR,
    SECS_PER_MIN,
    TM_YEAR_BASE,
    YEARS_PER_ERA,
};
use ::sysapi::{
    ffi::c_int,
    sys_types::time_t,
};

//==================================================================================================
// Standalone Functions
//==================================================================================================

///
/// # Description
///
/// Converts a calendar time (`time_t`) to a broken-down UTC time representation and stores the
/// result in the caller-provided `tm` structure.
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
pub unsafe extern "C" fn gmtime_r(timep: *const time_t, result: *mut tm) -> *mut tm {
    if timep.is_null() || result.is_null() {
        return core::ptr::null_mut();
    }

    let total_secs: time_t = *timep;

    // Extract time-of-day components using Euclidean division for correct negative handling.
    let day_secs: i64 = total_secs.rem_euclid(SECS_PER_DAY);
    let days: i64 = total_secs.div_euclid(SECS_PER_DAY);

    let hours: c_int = (day_secs / SECS_PER_HOUR) as c_int;
    let minutes: c_int = ((day_secs % SECS_PER_HOUR) / SECS_PER_MIN) as c_int;
    let seconds: c_int = (day_secs % SECS_PER_MIN) as c_int;

    // Weekday: January 1, 1970 was a Thursday (wday = 4).
    let wday: c_int = ((days + 4).rem_euclid(7)) as c_int;

    // Convert days-since-epoch to a civil (year, month, day) date in O(1) using the algorithm by
    // Howard Hinnant. `month` is in [1, 12] and `mday` in [1, 31].
    let z: i64 = days + EPOCH_DAYS_OFFSET;
    let era: i64 = if z >= 0 { z } else { z - (DAYS_PER_ERA - 1) } / DAYS_PER_ERA;
    let doe: i64 = z - era * DAYS_PER_ERA; // Day of era, in [0, DAYS_PER_ERA - 1].
    let yoe: i64 = (doe - doe / DAYS_PER_4_YEARS + doe / DAYS_PER_100_YEARS
        - doe / (DAYS_PER_ERA - 1))
        / DAYS_PER_COMMON_YEAR; // Year of era, in [0, 399].
    let civil_year: i64 = yoe + era * YEARS_PER_ERA;
    let doy: i64 = doe - (DAYS_PER_COMMON_YEAR * yoe + yoe / 4 - yoe / 100); // Day of year, [0, 365].
    let mp: i64 = (5 * doy + 2) / DAYS_PER_5_MONTHS; // Internal month index, in [0, 11].
    let mday: i64 = doy - (DAYS_PER_5_MONTHS * mp + 2) / 5 + 1; // Day of month, in [1, 31].
    let month: i64 = if mp < 10 { mp + 3 } else { mp - 9 }; // Calendar month, in [1, 12].
    let year: i64 = if month <= 2 {
        civil_year + 1
    } else {
        civil_year
    };

    // The `tm_year` field stores `year - TM_YEAR_BASE` as a `c_int`; reject dates that do not fit it
    // instead of silently wrapping.
    let tm_year: i64 = year - TM_YEAR_BASE;
    if tm_year < i64::from(c_int::MIN) || tm_year > i64::from(c_int::MAX) {
        return core::ptr::null_mut();
    }

    // Day of the year, in [0, 365].
    let yday: i64 = days - tm_struct::days_from_civil(year, 1, 1);

    (*result).tm_sec = seconds;
    (*result).tm_min = minutes;
    (*result).tm_hour = hours;
    (*result).tm_mday = mday as c_int;
    (*result).tm_mon = (month - 1) as c_int;
    (*result).tm_year = tm_year as c_int;
    (*result).tm_wday = wday;
    (*result).tm_yday = yday as c_int;
    (*result).tm_isdst = 0;
    (*result).tm_gmtoff = 0;

    result
}

//==================================================================================================
// Unit Tests
//==================================================================================================

#[cfg(all(test, feature = "std"))]
mod test {
    use super::gmtime_r;
    use crate::tm_struct::tm;
    use ::sysapi::sys_types::time_t;

    #[test]
    fn test_gmtime_r_epoch() {
        // 1970-01-01 00:00:00 UTC.
        let t: time_t = 0;
        let mut result: tm = tm::new();
        let ret: *mut tm = unsafe { gmtime_r(&t, &mut result) };
        assert!(!ret.is_null());
        assert_eq!(result.tm_sec, 0);
        assert_eq!(result.tm_min, 0);
        assert_eq!(result.tm_hour, 0);
        assert_eq!(result.tm_mday, 1);
        assert_eq!(result.tm_mon, 0);
        assert_eq!(result.tm_year, 70);
        assert_eq!(result.tm_wday, 4); // Thursday.
        assert_eq!(result.tm_yday, 0);
    }

    #[test]
    fn test_gmtime_r_known_date() {
        // 2000-01-01 00:00:00 UTC = 946684800 seconds since epoch.
        let t: time_t = 946_684_800;
        let mut result: tm = tm::new();
        let ret: *mut tm = unsafe { gmtime_r(&t, &mut result) };
        assert!(!ret.is_null());
        assert_eq!(result.tm_sec, 0);
        assert_eq!(result.tm_min, 0);
        assert_eq!(result.tm_hour, 0);
        assert_eq!(result.tm_mday, 1);
        assert_eq!(result.tm_mon, 0); // January.
        assert_eq!(result.tm_year, 100); // 2000 - 1900.
        assert_eq!(result.tm_wday, 6); // Saturday.
        assert_eq!(result.tm_yday, 0);
    }

    #[test]
    fn test_gmtime_r_leap_year() {
        // 2000-02-29 12:00:00 UTC = 951825600 seconds since epoch.
        let t: time_t = 951_825_600;
        let mut result: tm = tm::new();
        let ret: *mut tm = unsafe { gmtime_r(&t, &mut result) };
        assert!(!ret.is_null());
        assert_eq!(result.tm_sec, 0);
        assert_eq!(result.tm_min, 0);
        assert_eq!(result.tm_hour, 12);
        assert_eq!(result.tm_mday, 29);
        assert_eq!(result.tm_mon, 1); // February.
        assert_eq!(result.tm_year, 100); // 2000 - 1900.
        assert_eq!(result.tm_yday, 59); // Day 60, zero-indexed = 59.
    }

    #[test]
    fn test_gmtime_r_null_timep() {
        let mut result: tm = tm::new();
        let ret: *mut tm = unsafe { gmtime_r(core::ptr::null(), &mut result) };
        assert!(ret.is_null());
    }

    #[test]
    fn test_gmtime_r_null_result() {
        let t: time_t = 0;
        let ret: *mut tm = unsafe { gmtime_r(&t, core::ptr::null_mut()) };
        assert!(ret.is_null());
    }

    #[test]
    fn test_gmtime_r_year_out_of_range() {
        // Extreme calendar times yield a year that does not fit `tm_year` (a `c_int`); these must
        // be rejected in O(1) rather than looping or silently wrapping the year.
        let mut result: tm = tm::new();
        assert!(unsafe { gmtime_r(&time_t::MAX, &mut result) }.is_null());
        assert!(unsafe { gmtime_r(&time_t::MIN, &mut result) }.is_null());
    }

    #[test]
    fn test_gmtime_r_specific_time() {
        // 1993-06-30 21:49:08 UTC = 741476948 seconds since epoch.
        let t: time_t = 741_476_948;
        let mut result: tm = tm::new();
        let ret: *mut tm = unsafe { gmtime_r(&t, &mut result) };
        assert!(!ret.is_null());
        assert_eq!(result.tm_sec, 8);
        assert_eq!(result.tm_min, 49);
        assert_eq!(result.tm_hour, 21);
        assert_eq!(result.tm_mday, 30);
        assert_eq!(result.tm_mon, 5); // June (0-indexed).
        assert_eq!(result.tm_year, 93); // 1993 - 1900.
        assert_eq!(result.tm_wday, 3); // Wednesday.
    }
}
