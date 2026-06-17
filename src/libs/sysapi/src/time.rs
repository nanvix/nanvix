// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Configuration
//==================================================================================================

#![allow(non_camel_case_types)]

//==================================================================================================
// Imports
//==================================================================================================

use crate::{
    ffi::c_long,
    sys_types::time_t,
};
use ::core::mem::size_of;

//==================================================================================================
// Types
//==================================================================================================

/// Clock IDs to be used with the `clock_gettime()` and `clock_getres()` system calls.
pub mod clock_ids {
    use crate::sys_types::clockid_t;

    /// The identifier of the system-wide clock measuring real time.
    // TODO: gate this behind the CX extension.
    pub const CLOCK_REALTIME: clockid_t = 1;

    /// The identifier of the CPU-time clock associated with the process.
    // TODO: gate this behind the CPT extension.
    pub const CLOCK_PROCESS_CPUTIME_ID: clockid_t = 2;

    /// The identifier of the CPU-time clock associated with the thread.
    /// TODO: gate this behind the TCT extension.
    pub const CLOCK_THREAD_CPUTIME_ID: clockid_t = 3;

    /// The identifier for the system-wide monotonic clock.
    // TODO: gate this behind the CX extension.
    pub const CLOCK_MONOTONIC: clockid_t = 4;
}

//==================================================================================================
// Structures
//==================================================================================================

// TODO: define tms structure here.

/// Errors for the `timespec` structure.
#[derive(Debug)]
pub enum TimespecError {
    /// Error code indicating an invalid array size.
    InvalidArraySize,
    /// Error code indicating failure to parse `tv_sec` field.
    FailedToParseTvSec,
    /// Error code indicating failure to parse `tv_nsec` field.
    FailedToParseTvNsec,
}

/// Time spec structure.
#[derive(Default, Debug, Clone, Copy)]
#[repr(C, packed)]
pub struct timespec {
    /// Seconds.
    pub tv_sec: time_t,
    /// Nano-seconds.
    pub tv_nsec: c_long,
}
::static_assert::assert_eq_size!(timespec, timespec::SIZE);

impl timespec {
    /// Size of the seconds field.
    const SIZE_OF_TV_SEC: usize = size_of::<time_t>();
    /// Size of the nano-seconds field.
    const SIZE_OF_TV_NSEC: usize = size_of::<c_long>();
    /// Offset of the seconds field.
    const OFFSET_OF_TV_SEC: usize = 0;
    /// Offset of the nano-seconds field.
    const OFFSET_OF_TV_NSEC: usize = Self::OFFSET_OF_TV_SEC + Self::SIZE_OF_TV_SEC;

    /// In-memory size (matches `repr(C, packed)` layout, varies by target).
    const SIZE: usize = Self::SIZE_OF_TV_SEC + Self::SIZE_OF_TV_NSEC;

    /// Wire format always uses i64 (8 bytes) for tv_nsec, ensuring IPC compatibility
    /// between x86 guests (c_long=i32) and x86_64 hosts (c_long=i64).
    const WIRE_SIZE_OF_TV_NSEC: usize = size_of::<i64>();
    /// Wire format size for IPC serialization (always 16 bytes).
    pub const WIRE_SIZE: usize = Self::SIZE_OF_TV_SEC + Self::WIRE_SIZE_OF_TV_NSEC;

    /// Converts a time spec structure to a wire-format byte array.
    pub fn to_bytes(&self) -> [u8; Self::WIRE_SIZE] {
        let mut bytes: [u8; Self::WIRE_SIZE] = [0; Self::WIRE_SIZE];

        // Convert seconds field.
        bytes[Self::OFFSET_OF_TV_SEC..Self::OFFSET_OF_TV_SEC + Self::SIZE_OF_TV_SEC]
            .copy_from_slice(&self.tv_sec.to_ne_bytes());

        // Convert nano-seconds field (always as i64 for IPC compatibility).
        // Cast is needed on x86 (c_long=i32→i64) but is a no-op on x86_64 (c_long=i64).
        #[cfg_attr(target_arch = "x86_64", allow(clippy::unnecessary_cast))]
        let nsec_bytes = (self.tv_nsec as i64).to_ne_bytes();
        bytes[Self::OFFSET_OF_TV_NSEC..Self::OFFSET_OF_TV_NSEC + Self::WIRE_SIZE_OF_TV_NSEC]
            .copy_from_slice(&nsec_bytes);

        bytes
    }

    /// Tries to convert a time spec structure from a wire-format byte array.
    pub fn try_from_bytes(bytes: &[u8]) -> Result<Self, TimespecError> {
        // Check if the array has the correct wire size.
        if bytes.len() != Self::WIRE_SIZE {
            return Err(TimespecError::InvalidArraySize);
        }

        // Parse seconds field.
        let tv_sec: time_t = time_t::from_ne_bytes(
            bytes[Self::OFFSET_OF_TV_SEC..Self::OFFSET_OF_TV_SEC + Self::SIZE_OF_TV_SEC]
                .try_into()
                .map_err(|_| TimespecError::FailedToParseTvSec)?,
        );

        // Parse nano-seconds field (always read as i64, then truncate to c_long).
        // Cast is needed on x86 (i64→c_long=i32) but is a no-op on x86_64 (i64→c_long=i64).
        #[cfg_attr(target_arch = "x86_64", allow(clippy::unnecessary_cast))]
        let tv_nsec: c_long = i64::from_ne_bytes(
            bytes[Self::OFFSET_OF_TV_NSEC..Self::OFFSET_OF_TV_NSEC + Self::WIRE_SIZE_OF_TV_NSEC]
                .try_into()
                .map_err(|_| TimespecError::FailedToParseTvNsec)?,
        ) as c_long;

        Ok(Self { tv_sec, tv_nsec })
    }
}
