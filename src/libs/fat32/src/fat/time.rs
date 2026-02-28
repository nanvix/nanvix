// Copyright (c) The Maintainers of Nanvix.
// Licensed under the MIT license.

//! Time provider for FAT filesystem operations.

//==================================================================================================
// Structures
//==================================================================================================

/// A time provider that always returns the FAT epoch (1980-01-01 00:00:00).
///
/// Guest VMs have no reliable clock, so a fixed timestamp is used.
/// The FAT epoch is the earliest representable FAT timestamp and clearly
/// indicates "timestamp not meaningful."
#[derive(Debug, Clone, Copy, Default)]
pub struct NanvixTimeProvider;

//==================================================================================================
// Trait Implementations
//==================================================================================================

impl ::fatfs::TimeProvider for NanvixTimeProvider {
    fn get_current_date(&self) -> ::fatfs::Date {
        ::fatfs::Date::new(1980, 1, 1)
    }

    fn get_current_date_time(&self) -> ::fatfs::DateTime {
        ::fatfs::DateTime::new(::fatfs::Date::new(1980, 1, 1), ::fatfs::Time::new(0, 0, 0, 0))
    }
}
