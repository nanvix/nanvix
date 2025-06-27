// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Modules
//==================================================================================================

use ::core::time::Duration;
use ::sys::{
    error::{
        Error,
        ErrorCode,
    },
    kcall,
    time::SystemTime,
};
use sysapi::time::timespec;

//==================================================================================================
// Time
//==================================================================================================

///
/// # Description
///
/// This structure represents an instant in time.
///
#[derive(Default, Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Time(SystemTime);

impl Time {
    ///
    /// # Description
    ///
    /// Represents the epoch time, which is the time at which the system clock started.
    ///
    pub const EPOCH: Time = Time(SystemTime::EPOCH);

    ///
    /// # Description
    ///
    /// Converts the `Time` instance into a `SystemTime`.
    ///
    /// # Returns
    ///
    /// Returns the underlying `SystemTime` instance.
    ///
    pub fn into_system_time(self) -> SystemTime {
        self.0
    }

    ///
    /// # Description
    ///
    /// Gets the current system time.
    ///
    /// # Returns
    ///
    /// Upon successful completion, the current system time is returned.  Otherwise, an error is
    /// returned instead.
    ///
    pub fn now() -> Result<Time, Error> {
        let mut now: SystemTime = SystemTime::default();
        kcall::pm::gettime(&mut now)?;
        Ok(Time(now))
    }

    ///
    /// # Description
    ///
    /// Performs a checked subtraction of two times.
    ///
    /// # Parameters
    ///
    /// - `other`: The time to be subtracted from `self`.
    ///
    /// # Returns
    ///
    /// if the subtraction is successful, returns `Ok(Duration)`. Otherwise, returns `Err(Duration)`.
    ///
    pub fn checked_sub(&self, other: &Time) -> Result<Duration, Duration> {
        self.0.checked_sub(&other.0)
    }

    ///
    /// # Description
    ///
    /// Performs a checked addition of a time with a duration.
    ///
    /// # Parameters
    ///
    /// - `duration`: The duration to be added.
    ///
    /// # Returns
    ///
    /// If the addition is successful, returns `Some(Time)`. Otherwise, returns `None`.
    ///
    pub fn checked_add_duration(&self, duration: &Duration) -> Option<Self> {
        self.0.checked_add_duration(duration).map(Time)
    }

    ///
    /// # Description
    ///
    /// Performs a checked subtraction of a time with a duration.
    ///
    /// # Parameters
    ///
    /// - `duration`: The duration to be subtracted.
    ///
    /// # Returns
    ///
    /// If the subtraction is successful, returns `Some(Time)`. Otherwise, returns `None`.
    ///
    pub fn checked_sub_duration(&self, duration: &Duration) -> Option<Self> {
        self.0.checked_sub_duration(duration).map(Time)
    }

    ///
    /// # Description
    ///
    /// Returns the number of nanoseconds since the last second.
    ///
    /// # Returns
    ///
    /// The number of nanoseconds since the last second.
    ///
    pub fn nanoseconds(&self) -> u32 {
        self.0.nanoseconds()
    }

    ///
    /// # Description
    ///
    /// Returns the number of seconds since the epoch.
    ///
    /// # Returns
    ///
    /// The number of seconds since the epoch.
    ///
    pub fn seconds(&self) -> u64 {
        self.0.seconds()
    }
}

impl TryFrom<timespec> for Time {
    type Error = Error;

    fn try_from(ts: timespec) -> Result<Self, Self::Error> {
        let seconds: u64 = ts.tv_sec.try_into().map_err(|_| {
            Error::new(ErrorCode::InvalidArgument, "failed to convert tv_sec to u64")
        })?;

        let nanoseconds: u32 = ts.tv_nsec.try_into().map_err(|_| {
            Error::new(ErrorCode::InvalidArgument, "failed to convert tv_nsec to u32")
        })?;

        match SystemTime::new(seconds, nanoseconds) {
            Some(time) => Ok(Time(time)),
            None => {
                Err(Error::new(ErrorCode::InvalidArgument, "failed to convert timespec to Time"))
            },
        }
    }
}

impl From<Time> for timespec {
    fn from(time: Time) -> Self {
        timespec {
            tv_sec: time.0.seconds() as i64,
            tv_nsec: time.0.nanoseconds() as i32,
        }
    }
}
