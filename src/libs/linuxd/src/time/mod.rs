// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Configuration
//==================================================================================================

#![allow(non_camel_case_types)]

//==================================================================================================
// Modules
//==================================================================================================

pub mod message;

//==================================================================================================
// Imports
//==================================================================================================

use crate::sys::types::{
    clockid_t,
    time_t,
};
use ::core::mem;
use ::nvx::sys::error::{
    Error,
    ErrorCode,
};

//==================================================================================================
// Types
//==================================================================================================

/// The identifier of the system-wide clock measuring real time.
pub const CLOCK_REALTIME: clockid_t = 0;

/// The identifier for the system-wide monotonic clock.
pub const CLOCK_MONOTONIC: clockid_t = 1;

//==================================================================================================
// Structures
//==================================================================================================

/// Time spec structure.
#[derive(Default, Debug, Clone, Copy)]
#[repr(C, packed)]
pub struct timespec {
    /// Seconds.
    pub tv_sec: time_t,
    /// Nano-seconds.
    pub tv_nsec: i64,
}
::nvx::sys::static_assert_size!(timespec, timespec::SIZE);

impl timespec {
    /// Size of the seconds field.
    const SIZE_OF_TV_SEC: usize = mem::size_of::<time_t>();
    /// Size of the nano-seconds field.
    const SIZE_OF_TV_NSEC: usize = mem::size_of::<i64>();
    /// Offset of the seconds field.
    const OFFSET_OF_TV_SEC: usize = 0;
    /// Offset of the nano-seconds field.
    const OFFSET_OF_TV_NSEC: usize = Self::OFFSET_OF_TV_SEC + Self::SIZE_OF_TV_SEC;

    const SIZE: usize = Self::SIZE_OF_TV_SEC + Self::SIZE_OF_TV_NSEC;

    /// Converts a time spec structure to a byte array.
    pub fn to_bytes(&self) -> [u8; Self::SIZE] {
        let mut bytes: [u8; Self::SIZE] = [0; Self::SIZE];

        // Convert seconds field.
        bytes[Self::OFFSET_OF_TV_SEC..Self::OFFSET_OF_TV_SEC + Self::SIZE_OF_TV_SEC]
            .copy_from_slice(&self.tv_sec.to_ne_bytes());

        // Convert nano-seconds field.
        bytes[Self::OFFSET_OF_TV_NSEC..Self::OFFSET_OF_TV_NSEC + Self::SIZE_OF_TV_NSEC]
            .copy_from_slice(&self.tv_nsec.to_ne_bytes());

        bytes
    }

    /// Tries to convert a time spec structure from a byte array.
    pub fn try_from_bytes(bytes: &[u8]) -> Result<Self, Error> {
        // Check if the array has the correct size.
        if bytes.len() != Self::SIZE {
            return Err(Error::new(ErrorCode::InvalidArgument, "invalid array size"));
        }

        // Parse seconds field.
        let tv_sec: i64 = i64::from_ne_bytes(
            bytes[Self::OFFSET_OF_TV_SEC..Self::OFFSET_OF_TV_SEC + Self::SIZE_OF_TV_SEC]
                .try_into()
                .map_err(|_| Error::new(ErrorCode::InvalidArgument, "failed to parse tv_sec"))?,
        );

        // Parse nano-seconds field.
        let tv_nsec: i64 = i64::from_ne_bytes(
            bytes[Self::OFFSET_OF_TV_NSEC..Self::OFFSET_OF_TV_NSEC + Self::SIZE_OF_TV_NSEC]
                .try_into()
                .map_err(|_| Error::new(ErrorCode::InvalidArgument, "failed to parse tv_nsec"))?,
        );

        Ok(Self { tv_sec, tv_nsec })
    }
}

//==================================================================================================
// Standalone Functions
//==================================================================================================

cfg_if::cfg_if! {
    if #[cfg(feature = "syscall")] {
        mod syscall;
        pub use self::syscall::{
            clock_getres,
            clock_gettime,
        };
    }
}
