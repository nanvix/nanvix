// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

use super::*;
use ::fatfs::NullTimeProvider;

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

/// Zero-valued FAT dates represent an unknown timestamp.
#[test]
fn null_date_uses_fat_epoch() {
    let provider: NullTimeProvider = NullTimeProvider::new();
    assert_eq!(date_to_unix(provider.get_current_date()), FAT_EPOCH_SECS);
    assert_eq!(datetime_to_unix(provider.get_current_date_time()), FAT_EPOCH_SECS);
}
