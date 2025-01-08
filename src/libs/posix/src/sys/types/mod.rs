// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Configuration
//==================================================================================================

#![allow(non_camel_case_types)]

//==================================================================================================
// Imports
//==================================================================================================

use crate::ffi::{
    c_int,
    c_uint,
};

//==================================================================================================
// Types
//==================================================================================================

/// Used for file block counts.
/// TODO: fix with of this type to conform to Linux and POSIX.
pub type blkcnt_t = i64;

/// Used for block sizes.
/// TODO: fix with of this type to conform to Linux and POSIX.
pub type blksize_t = i64;

/// Used for system times in clock ticks or `CLOCKS_PER_SEC`.
/// TODO: fix with of this type to conform to Linux and POSIX.
pub type clock_t = i64;

/// Used for clock ID type in the clock and timer functions.
pub type clockid_t = c_int;

/// Used for device IDs.
/// TODO: fix with of this type to conform to Linux and POSIX.
pub type dev_t = u64;

/// Used for group IDs.
pub type gid_t = c_uint;

/// Used for file serial numbers.
/// TODO: fix with of this type to conform to Linux and POSIX.
pub type ino_t = u64;

/// Used for file attributes.
pub type mode_t = c_uint;

/// Used for link counts.
/// TODO: fix with of this type to conform to Linux and POSIX.
pub type nlink_t = u64;

/// Used for file sizes.
/// TODO: fix with of this type to conform to Linux and POSIX.
pub type off_t = i64;

/// Used for process IDs and process group IDs.
pub type pid_t = c_int;

/// Used for object sizes.
pub type size_t = c_uint;

/// Used for a count of bytes or an error indication.
pub type ssize_t = c_int;

/// Used for time in seconds.
/// TODO: fix with of this type to conform to Linux and POSIX.
pub type time_t = i64;

/// Used for user IDs.
pub type uid_t = c_uint;
