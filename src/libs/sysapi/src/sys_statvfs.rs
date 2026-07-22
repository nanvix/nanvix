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
    ffi::c_ulong,
    sys_types::{
        fsblkcnt_t,
        fsfilcnt_t,
    },
};

//==================================================================================================
// Constants
//==================================================================================================

/// Indicates a read-only file system.
pub const ST_RDONLY: c_ulong = 1;
/// Indicates that set-user-ID and set-group-ID bits are ignored on execution.
pub const ST_NOSUID: c_ulong = 2;

//==================================================================================================
// Structures
//==================================================================================================

/// File-system information returned by `statvfs()` and `fstatvfs()`.
#[derive(Debug, Clone, Copy, Default)]
#[repr(C)]
pub struct statvfs {
    /// File-system block size.
    pub f_bsize: c_ulong,
    /// Fundamental file-system block size.
    pub f_frsize: c_ulong,
    /// Total number of blocks.
    pub f_blocks: fsblkcnt_t,
    /// Total number of free blocks.
    pub f_bfree: fsblkcnt_t,
    /// Number of free blocks available to unprivileged users.
    pub f_bavail: fsblkcnt_t,
    /// Total number of file nodes.
    pub f_files: fsfilcnt_t,
    /// Total number of free file nodes.
    pub f_ffree: fsfilcnt_t,
    /// Number of free file nodes available to unprivileged users.
    pub f_favail: fsfilcnt_t,
    /// File-system identifier.
    pub f_fsid: c_ulong,
    /// Mount flags.
    pub f_flag: c_ulong,
    /// Maximum filename length.
    pub f_namemax: c_ulong,
}
