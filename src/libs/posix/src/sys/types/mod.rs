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
    c_longlong,
    c_uint,
    c_ulonglong,
};

//==================================================================================================
// Types
//==================================================================================================

/// Used for file block counts.
pub type blkcnt_t = c_longlong;

/// Used for block sizes.
pub type blksize_t = c_longlong;

/// Used for system times in clock ticks or `CLOCKS_PER_SEC`.
pub type clock_t = c_longlong;

/// Used for clock ID type in the clock and timer functions.
pub type clockid_t = c_int;

/// Used for device IDs.
pub type dev_t = c_ulonglong;

/// Used for group IDs.
pub type gid_t = c_uint;

/// Used for file serial numbers.
pub type ino_t = c_ulonglong;

/// Used for file attributes.
pub type mode_t = c_uint;

/// Used for link counts.
pub type nlink_t = c_ulonglong;

/// Used for file sizes.
pub type off_t = c_longlong;

/// Used for process IDs and process group IDs.
pub type pid_t = c_int;

/// Used for object sizes.
pub type size_t = c_uint;

/// Used for a count of bytes or an error indication.
pub type ssize_t = c_int;

/// Used for time in seconds.
pub type time_t = c_longlong;

/// Used for user IDs.
pub type uid_t = c_uint;
