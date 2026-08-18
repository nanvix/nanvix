// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//! Wall-clock helper shared across VFS backends.

use ::fat32::FAT_EPOCH_SECS;
#[cfg(not(feature = "std"))]
use ::sys::{
    kcall::pm::__kcall_gettime,
    time::SystemTime,
};

/// Current wall-clock time in Unix seconds, falling back to the FAT epoch.
#[cfg(not(feature = "std"))]
pub(crate) fn wall_clock_secs() -> i64 {
    let mut now: SystemTime = SystemTime::default();
    match __kcall_gettime(&mut now) {
        Ok(()) => now.seconds() as i64,
        Err(_) => FAT_EPOCH_SECS,
    }
}

/// Host test builds have no kernel clock; use the FAT epoch.
#[cfg(feature = "std")]
pub(crate) fn wall_clock_secs() -> i64 {
    FAT_EPOCH_SECS
}
