// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Modules
//==================================================================================================

use crate::NanvixUserland;
use ::core::time::Duration;
use ::litebox::platform::{
    Instant,
    TimeProvider,
};
use ::posix::time::{
    clock_gettime,
    timespec,
    CLOCK_MONOTONIC,
};

//==================================================================================================
// Implementations
//==================================================================================================

pub struct NanvixInstant {
    inner: timespec,
}

impl Instant for NanvixInstant {
    ///
    /// # Description
    ///
    /// Computes the duration between two instants.
    ///
    /// # Parameters
    ///
    /// - `self`: The later instant.
    /// - `earlier`: The earlier instant.
    ///
    /// # Returns
    ///
    /// If `self` happened after `earlier`, returns the duration between them.  Otherwise, returns
    /// `None`.
    ///
    fn checked_duration_since(&self, earlier: &Self) -> Option<Duration> {
        // Check is `self` happened after `earlier`.
        if self.inner.tv_sec > earlier.inner.tv_sec
            || (self.inner.tv_sec == earlier.inner.tv_sec
                && self.inner.tv_nsec >= earlier.inner.tv_nsec)
        {
            // Self happened after `earlier`, compute duration.

            // Compute `self` - `earlier`.
            let secs: i64 = self.inner.tv_sec - earlier.inner.tv_sec;
            let nanos: i32 = if self.inner.tv_nsec >= earlier.inner.tv_nsec {
                self.inner.tv_nsec - earlier.inner.tv_nsec
            } else {
                self.inner.tv_nsec + 1_000_000_000 - earlier.inner.tv_nsec
            };

            Some(Duration::new(secs as u64, nanos as u32))
        } else {
            // `self` happened before `earlier`, don't compute duration.
            None
        }
    }
}

impl TimeProvider for NanvixUserland {
    type Instant = NanvixInstant;

    ///
    /// # Description
    ///
    /// Gets the current time.
    ///
    /// # Returns
    ///
    /// The current time.
    ///
    ///
    /// # Safety
    ///
    /// This function panics if we cannot get the current time.
    ///
    fn now(&self) -> Self::Instant {
        let mut ts: timespec = timespec {
            tv_sec: 0,
            tv_nsec: 0,
        };

        // Get current time and check for errors.
        if let Err(error) = clock_gettime(CLOCK_MONOTONIC, &mut Some(&mut ts)) {
            panic!("now(): {:?}", error);
        }

        NanvixInstant { inner: ts }
    }
}
