// Copyright (c) The Maintainers of Nanvix.
// Licensed under the MIT license.

//! Time provider and epoch conversions for FAT filesystem operations.

//==================================================================================================
// Imports
//==================================================================================================

use ::fatfs::{
    Date,
    DateTime,
    Time,
    TimeProvider,
};
#[cfg(not(feature = "std"))]
use ::sys::{
    kcall::pm::__kcall_gettime,
    time::SystemTime,
};

//==================================================================================================
// Constants
//==================================================================================================

/// Seconds between the Unix epoch (1970-01-01) and the FAT epoch (1980-01-01).
pub const FAT_EPOCH_SECS: i64 = 315_532_800;

/// Lowest year representable in a FAT timestamp.
const MIN_FAT_YEAR: i64 = 1980;

/// Highest year representable in a FAT timestamp.
const MAX_FAT_YEAR: i64 = 2107;

const SECS_PER_DAY: i64 = 86_400;

//==================================================================================================
// Structures
//==================================================================================================

/// A time provider backed by the wall clock.
///
/// FAT stamps create/modify/access times from this provider on file I/O. Guest
/// builds read the real clock via `gettime`; host (`std`) test builds fall back
/// to the FAT epoch so results stay deterministic.
#[derive(Debug, Clone, Copy, Default)]
pub struct NanvixTimeProvider;

//==================================================================================================
// Trait Implementations
//==================================================================================================

impl TimeProvider for NanvixTimeProvider {
    fn get_current_date(&self) -> Date {
        unix_to_datetime(now_secs()).date
    }

    fn get_current_date_time(&self) -> DateTime {
        unix_to_datetime(now_secs())
    }
}

//==================================================================================================
// Standalone Functions
//==================================================================================================

/// Current wall-clock time in Unix seconds.
///
/// Falls back to the FAT epoch when no clock is available.
#[cfg(not(feature = "std"))]
fn now_secs() -> i64 {
    let mut now: SystemTime = SystemTime::default();
    match __kcall_gettime(&mut now) {
        Ok(()) => now.seconds() as i64,
        Err(_) => FAT_EPOCH_SECS,
    }
}

/// Host test builds have no kernel clock; use the FAT epoch.
#[cfg(feature = "std")]
fn now_secs() -> i64 {
    FAT_EPOCH_SECS
}

/// Converts Unix seconds to a FAT `DateTime`, clamped to the FAT year range.
///
/// Uses Howard Hinnant's civil-from-days algorithm. Sub-second precision is
/// dropped (FAT has none).
pub(crate) fn unix_to_datetime(secs: i64) -> DateTime {
    let days: i64 = secs.div_euclid(SECS_PER_DAY);
    let rem: i64 = secs.rem_euclid(SECS_PER_DAY);
    let (year, month, day) = civil_from_days(days);

    // Clamp to what a FAT date can hold.
    if year < MIN_FAT_YEAR {
        return DateTime::new(Date::new(MIN_FAT_YEAR as u16, 1, 1), Time::new(0, 0, 0, 0));
    }
    if year > MAX_FAT_YEAR {
        return DateTime::new(Date::new(MAX_FAT_YEAR as u16, 12, 31), Time::new(23, 59, 58, 0));
    }

    let hour: u16 = (rem / 3600) as u16;
    let min: u16 = ((rem % 3600) / 60) as u16;
    let sec: u16 = (rem % 60) as u16;
    DateTime::new(Date::new(year as u16, month as u16, day as u16), Time::new(hour, min, sec, 0))
}

/// Converts a FAT `DateTime` back to Unix seconds.
pub(crate) fn datetime_to_unix(dt: DateTime) -> i64 {
    let days: i64 =
        days_from_civil(i64::from(dt.date.year), i64::from(dt.date.month), i64::from(dt.date.day));
    days * SECS_PER_DAY
        + i64::from(dt.time.hour) * 3600
        + i64::from(dt.time.min) * 60
        + i64::from(dt.time.sec)
}

/// Converts a FAT `Date` (no time part) to Unix seconds at midnight.
pub(crate) fn date_to_unix(date: Date) -> i64 {
    days_from_civil(i64::from(date.year), i64::from(date.month), i64::from(date.day)) * SECS_PER_DAY
}

/// Civil (year, month, day) from days since the Unix epoch.
///
/// Howard Hinnant, <http://howardhinnant.github.io/date_algorithms.html>.
fn civil_from_days(z: i64) -> (i64, i64, i64) {
    let z: i64 = z + 719_468;
    let era: i64 = if z >= 0 { z } else { z - 146_096 } / 146_097;
    let doe: i64 = z - era * 146_097;
    let yoe: i64 = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let year: i64 = yoe + era * 400;
    let doy: i64 = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp: i64 = (5 * doy + 2) / 153;
    let day: i64 = doy - (153 * mp + 2) / 5 + 1;
    let month: i64 = if mp < 10 { mp + 3 } else { mp - 9 };
    (year + i64::from(month <= 2), month, day)
}

/// Days since the Unix epoch from a civil (year, month, day).
///
/// Howard Hinnant, <http://howardhinnant.github.io/date_algorithms.html>.
fn days_from_civil(year: i64, month: i64, day: i64) -> i64 {
    let year: i64 = year - i64::from(month <= 2);
    let era: i64 = if year >= 0 { year } else { year - 399 } / 400;
    let yoe: i64 = year - era * 400;
    let doy: i64 = (153 * (if month > 2 { month - 3 } else { month + 9 }) + 2) / 5 + day - 1;
    let doe: i64 = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    era * 146_097 + doe - 719_468
}

//==================================================================================================
// Unit Tests
//==================================================================================================

#[cfg(all(test, feature = "std"))]
mod tests {
    use super::*;

    /// Round-trips a known timestamp through DateTime and back.
    #[test]
    fn unix_datetime_roundtrip() {
        // 2024-01-01T00:00:00Z.
        let secs: i64 = 1_704_067_200;
        let dt: DateTime = unix_to_datetime(secs);
        assert_eq!(dt.date.year, 2024, "year");
        assert_eq!(dt.date.month, 1, "month");
        assert_eq!(dt.date.day, 1, "day");
        assert_eq!(datetime_to_unix(dt), secs, "roundtrip");
    }

    /// Time-of-day survives the conversion.
    #[test]
    fn unix_datetime_time_of_day() {
        // 2026-08-11T21:02:05Z.
        let secs: i64 = 1_786_568_525;
        let dt: DateTime = unix_to_datetime(secs);
        assert_eq!(dt.time.hour, 21, "hour");
        assert_eq!(dt.time.min, 2, "min");
        // FAT stores modification seconds at 2s resolution; readback tolerates it.
        assert!((datetime_to_unix(dt) - secs).abs() <= 2, "within 2s");
    }

    /// Pre-1980 timestamps clamp to the FAT epoch.
    #[test]
    fn unix_datetime_clamps_low() {
        let dt: DateTime = unix_to_datetime(0);
        assert_eq!(dt.date.year, 1980, "clamped to min FAT year");
    }

    /// FAT epoch constant matches 1980-01-01.
    #[test]
    fn fat_epoch_constant() {
        let dt: DateTime = unix_to_datetime(FAT_EPOCH_SECS);
        assert_eq!(dt.date.year, 1980, "year");
        assert_eq!(dt.date.month, 1, "month");
        assert_eq!(dt.date.day, 1, "day");
    }
}
