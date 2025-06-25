// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Configuration
//==================================================================================================

#![allow(non_camel_case_types)]

//==================================================================================================
// Imports
//==================================================================================================

use crate::sys_types::{
    suseconds_t,
    time_t,
};

//==================================================================================================
// Constants
//==================================================================================================

/// Microseconds in a second.
const MICROSECONDS_PER_SECOND: i32 = 1_000_000;

/// Nanoseconds in a second.
const NANOSECONDS_PER_SECOND: i32 = 1_000_000_000;

//==================================================================================================
// Structures
//==================================================================================================

#[derive(Default, Debug, Clone, Copy)]
#[repr(C, packed)]
pub struct timeval {
    /// Seconds.
    pub tv_sec: time_t,
    /// Nano-seconds.
    pub tv_usec: suseconds_t,
}

/// Errors that can occur when converting a `timeval` to a `timespec`.
#[derive(Debug, Clone, Copy)]
pub enum TimevalToTimespecParseError {
    /// Error code indicating failure to parse `tv_sec` field.
    FailedToParseTvSec,
    /// Error code indicating failure to parse `tv_usec` field.
    FailedToParseTvUsec,
}

impl TryFrom<timeval> for crate::time::timespec {
    type Error = TimevalToTimespecParseError;

    fn try_from(tv: timeval) -> Result<Self, Self::Error> {
        // Check if `tv_sec` is valid.
        if tv.tv_sec < 0 {
            return Err(TimevalToTimespecParseError::FailedToParseTvSec);
        }

        // Check if `tv_usec` is valid.
        if tv.tv_usec < 0 || tv.tv_usec >= MICROSECONDS_PER_SECOND {
            return Err(TimevalToTimespecParseError::FailedToParseTvUsec);
        }

        // Handle wrap around for nanoseconds.
        let mut sec: time_t = tv.tv_sec;
        let mut nsec: suseconds_t = tv.tv_usec * (NANOSECONDS_PER_SECOND / MICROSECONDS_PER_SECOND);
        if nsec >= NANOSECONDS_PER_SECOND {
            sec += 1;
            nsec -= NANOSECONDS_PER_SECOND;
        }

        Ok(crate::time::timespec {
            tv_sec: sec,
            tv_nsec: nsec,
        })
    }
}
