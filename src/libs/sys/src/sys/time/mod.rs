// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Modules
//==================================================================================================

#[cfg(all(test, feature = "std"))]
mod tests;

//==================================================================================================
// Imports
//==================================================================================================

use ::core::{
    cmp::Ordering,
    time::Duration,
};

//==================================================================================================
// Constants
//==================================================================================================

/// Nanoseconds in a second.
pub const NANOSECONDS_PER_SECOND: u32 = 1_000_000_000;

//==================================================================================================
// Structures
//==================================================================================================

///
/// # Description
///
/// This structure represents an instant in time.
///
#[derive(Default, Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub struct SystemTime {
    // The number of seconds since the epoch.
    seconds: u64,
    // The number of nanoseconds since the last second.
    nanoseconds: u32,
}

//==================================================================================================
// Implementations
//==================================================================================================

impl SystemTime {
    ///
    /// # Description
    ///
    /// The epoch (January 1, 1970).
    ///
    pub const EPOCH: Self = Self {
        seconds: 0,
        nanoseconds: 0,
    };

    ///
    /// # Description
    ///
    /// Creates a new `SystemTime` with the given seconds and nanoseconds.
    ///
    /// # Parameters
    ///
    /// * `seconds`:  The number of seconds since the epoch.
    /// * `nanoseconds`: The number of nanoseconds since the last second.
    ///
    /// # Returns
    ///
    /// If the parameters are valid, returns `Some(SystemTime)`. Otherwise, returns `None`.
    ///
    pub fn new(seconds: u64, nanoseconds: u32) -> Option<Self> {
        // Check if the nanoseconds are within the valid range.
        if nanoseconds >= NANOSECONDS_PER_SECOND {
            return None;
        }

        Some(Self {
            seconds,
            nanoseconds,
        })
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
        self.seconds
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
        self.nanoseconds
    }

    ///
    /// # Description
    ///
    /// Performs a checked subtraction of two `SystemTime` instances.
    ///
    /// # Parameters
    ///
    /// - `earlier`: The other `SystemTime` instance to subtract.
    ///
    /// # Returns
    ///
    /// If `self` happened after `earlier`, returns the duration between them as `Ok(Duration)`.
    /// Otherwise, returns the negative duration as `Err(Duration)`.
    ///
    pub fn checked_sub(&self, earlier: &SystemTime) -> Result<Duration, Duration> {
        // Check if `self` happened after `earlier`.
        if self.seconds >= earlier.seconds {
            // Compute the difference in seconds using checked subtraction.
            let seconds: u64 = self.seconds - earlier.seconds;

            // Compute the difference in nanoseconds using checked arithmetic.
            if self.nanoseconds >= earlier.nanoseconds {
                let nanoseconds: u32 = self.nanoseconds - earlier.nanoseconds;
                Ok(Duration::new(seconds, nanoseconds))
            } else {
                let nanoseconds: u32 = earlier.nanoseconds - self.nanoseconds;
                let adjusted_seconds: u64 = seconds - 1;
                Ok(Duration::new(adjusted_seconds, NANOSECONDS_PER_SECOND - nanoseconds))
            }
        } else {
            // `self` happened before `earlier`, so return `None`.
            match earlier.checked_sub(self) {
                Ok(duration) => Err(duration),
                Err(duration) => Ok(duration),
            }
        }
    }

    ///
    /// # Description
    ///
    /// Performs a checked addition of a `SystemTime` instance and a `Duration`.
    ///
    /// # Parameters
    ///
    /// - `duration`:  The `Duration` to add.
    ///
    /// # Returns
    ///
    /// If the addition does not overflow, returns the new `SystemTime`. Otherwise, returns `None`.
    ///
    pub fn checked_add_duration(&self, duration: &Duration) -> Option<SystemTime> {
        // Compute the new seconds and nanoseconds using checked arithmetic.
        let new_seconds: u64 = self.seconds.checked_add(duration.as_secs())?;
        let new_nanoseconds: u32 = self.nanoseconds.checked_add(duration.subsec_nanos())?;

        // Check for overflow in nanoseconds.
        if new_nanoseconds >= NANOSECONDS_PER_SECOND {
            new_seconds
                .checked_add(1)
                .and_then(|s| SystemTime::new(s, new_nanoseconds - NANOSECONDS_PER_SECOND))
        } else {
            SystemTime::new(new_seconds, new_nanoseconds)
        }
    }

    ///
    /// # Description
    ///
    /// Performs a checked subtraction of a `SystemTime` instance and a `Duration`.
    ///
    /// # Parameters
    ///
    /// - `duration`:  The `Duration` to subtract.
    ///
    /// # Returns
    ///
    /// If the subtraction does not underflow, returns the new `SystemTime`. Otherwise, returns
    /// `None`.
    ///
    pub fn checked_sub_duration(&self, duration: &Duration) -> Option<SystemTime> {
        // Compute the new seconds and nanoseconds using checked arithmetic.
        let new_seconds: u64 = self.seconds.checked_sub(duration.as_secs())?;
        let new_nanoseconds: u32 = self.nanoseconds.checked_sub(duration.subsec_nanos())?;

        // Check for underflow in nanoseconds.
        if new_nanoseconds >= NANOSECONDS_PER_SECOND {
            new_seconds
                .checked_sub(1)
                .and_then(|s| SystemTime::new(s, NANOSECONDS_PER_SECOND - new_nanoseconds))
        } else {
            SystemTime::new(new_seconds, new_nanoseconds)
        }
    }
}

impl PartialOrd for SystemTime {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for SystemTime {
    fn cmp(&self, other: &Self) -> Ordering {
        match self.seconds.cmp(&other.seconds) {
            Ordering::Equal => self.nanoseconds.cmp(&other.nanoseconds),
            other => other,
        }
    }
}
