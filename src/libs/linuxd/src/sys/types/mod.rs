// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Configuration
//==================================================================================================

#![allow(non_camel_case_types)]

//==================================================================================================

/// Used for file block counts.
pub type blkcnt_t = i64;

/// Used for block sizes.
pub type blksize_t = i64;

/// Used for clock ID type in the clock and timer functions.
pub type clockid_t = i32;

/// Used for device IDs.
pub type dev_t = u64;

/// Used for group IDs.
pub type gid_t = u32;

/// Used for file serial numbers.
pub type ino_t = u64;

/// Used for link counts.
pub type nlink_t = u64;

/// Used for file sizes.
pub type off_t = i64;

/// Used for file attributes.
pub type mode_t = u32;

/// Used for time in seconds.
pub type time_t = i64;

/// Used for user IDs.
pub type uid_t = u32;
