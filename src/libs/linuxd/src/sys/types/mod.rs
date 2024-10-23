// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Configuration
//==================================================================================================

#![allow(non_camel_case_types)]

//==================================================================================================
// Imports
//==================================================================================================

use ::core::ffi;

//==================================================================================================

/// Used for file block counts.
pub type blkcnt_t = ffi::c_long;

/// Used for block sizes.
pub type blksize_t = ffi::c_long;

/// Used for clock ID type in the clock and timer functions.
pub type clockid_t = i32;

/// Used for device IDs.
pub type dev_t = u64;

/// Used for group IDs.
pub type gid_t = u32;

/// Used for file serial numbers.
pub type ino_t = ffi::c_ulong;

/// Used for link counts.
pub type nlink_t = u64;

/// Used for file sizes.
pub type off_t = ffi::c_long;

/// Used for file attributes.
pub type mode_t = ffi::c_uint;

/// Used for time in seconds.
pub type time_t = i64;

/// Used for user IDs.
pub type uid_t = u32;
