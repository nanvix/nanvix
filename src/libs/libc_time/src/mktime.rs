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
    SECS_PER_DAY,
    SECS_PER_HOUR,
    SECS_PER_MIN,
    TM_YEAR_BASE,
};
use ::sysapi::sys_types::time_t;

//==================================================================================================
// Standalone Functions
//==================================================================================================

///
/// # Description
///
/// Converts a broken-down time structure (expressed as UTC) to a calendar time (`time_t`).
/// The `tm` structure is normalized so that its fields are within their proper ranges.
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
#[cfg_attr(not(feature = "std"), unsafe(no_mangle))]
pub unsafe extern "C" fn mktime(timeptr: *mut tm) -> time_t {
    if timeptr.is_null() {
        return -1;
    }

    let t: &mut tm = &mut *timeptr;

    // Normalize seconds → minutes.
    let mut total_min: i64 = i64::from(t.tm_min) + i64::from(t.tm_sec) / SECS_PER_MIN;
    let mut sec: i64 = i64::from(t.tm_sec) % SECS_PER_MIN;
    if sec < 0 {
        sec += SECS_PER_MIN;
        total_min -= 1;
    }

    // Normalize minutes → hours.
    let mut total_hour: i64 = i64::from(t.tm_hour) + total_min / 60;
    let mut min: i64 = total_min % 60;
    if min < 0 {
        min += 60;
        total_hour -= 1;
    }

    // Normalize hours → days.
    let mut extra_days: i64 = total_hour / 24;
    let mut hour: i64 = total_hour % 24;
    if hour < 0 {
        hour += 24;
        extra_days -= 1;
    }

    // Normalize months → years.
    let total_mon: i64 = i64::from(t.tm_mon);
    let mut year: i64 = i64::from(t.tm_year) + TM_YEAR_BASE + total_mon / 12;
    let mut mon: i64 = total_mon % 12;
    if mon < 0 {
        mon += 12;
        year -= 1;
    }

    // Compute total days since the epoch in O(1) from the normalized civil date.
    let days: i64 = tm_struct::days_from_civil(year, mon + 1, i64::from(t.tm_mday)) + extra_days;

    // The intra-day term is bounded, but `days * SECS_PER_DAY` can overflow for far-future or
    // far-past dates. Use checked arithmetic and report an error instead of wrapping.
    let secs_of_day: i64 = hour * SECS_PER_HOUR + min * SECS_PER_MIN + sec;
    let result: time_t = match days
        .checked_mul(SECS_PER_DAY)
        .and_then(|day_secs| day_secs.checked_add(secs_of_day))
    {
        Some(result) => result,
        None => return -1,
    };

    // Normalize the tm struct by converting back. If the back-conversion fails (e.g., the computed
    // year does not fit in `tm_year`), the value cannot be round-tripped through `struct tm`, so
    // report an error and leave the result unspecified per the C standard.
    if crate::gmtime_r::gmtime_r(&result, timeptr).is_null() {
        return -1;
    }

    result
}

//==================================================================================================
// Unit Tests
//==================================================================================================

#[cfg(all(test, feature = "std"))]
mod test {
    use super::mktime;
    use crate::tm_struct::tm;
    use ::sysapi::sys_types::time_t;

    #[test]
    fn test_mktime_epoch() {
        let mut t: tm = tm::new();
        t.tm_year = 70;
        t.tm_mon = 0;
        t.tm_mday = 1;
        let result: time_t = unsafe { mktime(&mut t) };
        assert_eq!(result, 0);
    }

    #[test]
    fn test_mktime_known_date() {
        // 2000-01-01 00:00:00 UTC.
        let mut t: tm = tm::new();
        t.tm_year = 100;
        t.tm_mon = 0;
        t.tm_mday = 1;
        let result: time_t = unsafe { mktime(&mut t) };
        assert_eq!(result, 946_684_800);
    }

    #[test]
    fn test_mktime_round_trip() {
        let original: time_t = 741_476_948;
        let mut broken: tm = tm::new();
        unsafe { crate::gmtime_r::gmtime_r(&original, &mut broken) };
        let result: time_t = unsafe { mktime(&mut broken) };
        assert_eq!(result, original);
    }

    #[test]
    fn test_mktime_normalizes() {
        // 1970-01-01 00:00:90 (90 seconds should normalize to 00:01:30).
        let mut t: tm = tm::new();
        t.tm_year = 70;
        t.tm_mon = 0;
        t.tm_mday = 1;
        t.tm_sec = 90;
        let result: time_t = unsafe { mktime(&mut t) };
        assert_eq!(result, 90);
        assert_eq!(t.tm_min, 1);
        assert_eq!(t.tm_sec, 30);
    }

    #[test]
    fn test_mktime_null() {
        let result: time_t = unsafe { mktime(core::ptr::null_mut()) };
        assert_eq!(result, -1);
    }
}
