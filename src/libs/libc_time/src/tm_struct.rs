// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Lint Configuration
//==================================================================================================

#![allow(non_camel_case_types)]

//==================================================================================================
// Imports
//==================================================================================================

use ::sysapi::ffi::c_int;

//==================================================================================================
// Constants
//==================================================================================================

/// Seconds per minute.
pub const SECS_PER_MIN: i64 = 60;

/// Seconds per hour.
pub const SECS_PER_HOUR: i64 = 3600;

/// Seconds per day.
pub const SECS_PER_DAY: i64 = 86400;

/// Year that the `tm_year` field is offset from.
pub const TM_YEAR_BASE: i64 = 1900;

/// Number of years in a full Gregorian era (one complete leap-year cycle).
pub const YEARS_PER_ERA: i64 = 400;

/// Number of days in a full Gregorian era of 400 years.
pub const DAYS_PER_ERA: i64 = 146_097;

/// Number of days in a common (non-leap) year.
pub const DAYS_PER_COMMON_YEAR: i64 = 365;

/// Number of days in a 4-year cycle, used for leap-year correction.
pub const DAYS_PER_4_YEARS: i64 = 1_460;

/// Number of days in a 100-year cycle, used for leap-year correction.
pub const DAYS_PER_100_YEARS: i64 = 36_524;

/// Number of days in a 5-month block (March-July or August-December), used to convert between the
/// day of the year and the month.
pub const DAYS_PER_5_MONTHS: i64 = 153;

/// Number of days from 0000-03-01 to the Unix epoch (1970-01-01), used to align the era-based
/// civil-date algorithm with the epoch.
pub const EPOCH_DAYS_OFFSET: i64 = 719_468;

/// Days in each month (non-leap year).
pub const DAYS_IN_MONTH: [i64; 12] = [31, 28, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31];

/// Abbreviated day names.
pub const DAY_NAMES: [&[u8; 3]; 7] = [b"Sun", b"Mon", b"Tue", b"Wed", b"Thu", b"Fri", b"Sat"];

/// Abbreviated month names.
pub const MONTH_NAMES: [&[u8; 3]; 12] = [
    b"Jan", b"Feb", b"Mar", b"Apr", b"May", b"Jun", b"Jul", b"Aug", b"Sep", b"Oct", b"Nov", b"Dec",
];

//==================================================================================================
// Structures
//==================================================================================================

/// Broken-down time structure, as defined by POSIX.
#[derive(Clone, Copy)]
#[repr(C)]
pub struct tm {
    /// Seconds after the minute `[0, 60]`.
    pub tm_sec: c_int,
    /// Minutes after the hour `[0, 59]`.
    pub tm_min: c_int,
    /// Hours since midnight `[0, 23]`.
    pub tm_hour: c_int,
    /// Day of the month `[1, 31]`.
    pub tm_mday: c_int,
    /// Months since January `[0, 11]`.
    pub tm_mon: c_int,
    /// Years since 1900.
    pub tm_year: c_int,
    /// Days since Sunday `[0, 6]`.
    pub tm_wday: c_int,
    /// Days since January 1 `[0, 365]`.
    pub tm_yday: c_int,
    /// Daylight Saving Time flag.
    pub tm_isdst: c_int,
}

impl tm {
    /// Creates a zeroed broken-down time structure.
    pub const fn new() -> Self {
        Self {
            tm_sec: 0,
            tm_min: 0,
            tm_hour: 0,
            tm_mday: 0,
            tm_mon: 0,
            tm_year: 0,
            tm_wday: 0,
            tm_yday: 0,
            tm_isdst: 0,
        }
    }
}

impl Default for tm {
    fn default() -> Self {
        Self::new()
    }
}

//==================================================================================================
// Standalone Functions
//==================================================================================================

/// Returns `true` if the given year is a leap year.
pub fn is_leap_year(year: i64) -> bool {
    year % 4 == 0 && (year % 100 != 0 || year % 400 == 0)
}

/// Returns the number of days in the given month of the given year.
///
/// `month` is expressed in `[0, 11]`. Returns `None` if `month` is out of range.
pub fn days_in_month(month: usize, year: i64) -> Option<i64> {
    if month == 1 && is_leap_year(year) {
        Some(29)
    } else if month < 12 {
        Some(DAYS_IN_MONTH[month])
    } else {
        None
    }
}

/// Returns the number of days in the given year.
pub fn days_in_year(year: i64) -> i64 {
    if is_leap_year(year) {
        366
    } else {
        365
    }
}

/// Returns the number of days from the Unix epoch (1970-01-01) to the given civil date.
///
/// `month` is expressed in `[1, 12]` and `day` in `[1, 31]`. The computation is `O(1)` and uses
/// the algorithm by Howard Hinnant, so it stays bounded regardless of how far the date is from the
/// epoch.
pub fn days_from_civil(year: i64, month: i64, day: i64) -> i64 {
    // Shift so that the leap day falls at the end of the (internal) year.
    let y: i64 = if month <= 2 { year - 1 } else { year };
    let era: i64 = if y >= 0 { y } else { y - (YEARS_PER_ERA - 1) } / YEARS_PER_ERA;
    let yoe: i64 = y - era * YEARS_PER_ERA; // Year of era, in [0, 399].
    let mp: i64 = if month > 2 { month - 3 } else { month + 9 }; // Month index, in [0, 11].
    let doy: i64 = (DAYS_PER_5_MONTHS * mp + 2) / 5 + day - 1; // Day of internal year, in [0, 365].
    let doe: i64 = yoe * DAYS_PER_COMMON_YEAR + yoe / 4 - yoe / 100 + doy; // Day of era.
    era * DAYS_PER_ERA + doe - EPOCH_DAYS_OFFSET
}
