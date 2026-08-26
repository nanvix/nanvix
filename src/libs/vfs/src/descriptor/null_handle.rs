// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//! Null-device descriptor handle.

//==================================================================================================
// Imports
//==================================================================================================

use super::AccessMode;
use ::fat32::Fat32Error;
use ::sysapi::{
    ffi::{
        c_int,
        c_short,
    },
    poll::poll_flags::{
        POLLIN,
        POLLOUT,
        POLLRDNORM,
        POLLWRNORM,
    },
    sys_types::off_t,
    unistd::file_seek,
};

//==================================================================================================
// Structures
//==================================================================================================

/// An open description of `/dev/null`.
pub struct NullHandle {
    /// Access permitted by this open description.
    access_mode: AccessMode,
}

//==================================================================================================
// Implementations
//==================================================================================================

impl NullHandle {
    /// Creates a null-device handle with the requested access mode.
    pub fn new(access_mode: AccessMode) -> Self {
        Self { access_mode }
    }

    /// Reads EOF from the null device.
    pub fn read(&self) -> Result<usize, Fat32Error> {
        if !self.access_mode.readable() {
            return Err(Fat32Error::InvalidFd);
        }
        Ok(0)
    }

    /// Discards a buffer and returns its full length.
    pub fn write(&self, buf: &[u8]) -> Result<usize, Fat32Error> {
        if !self.access_mode.writable() {
            return Err(Fat32Error::InvalidFd);
        }
        Ok(buf.len())
    }

    /// Seeks the null device, which remains at offset zero.
    pub fn seek(&self, whence: c_int) -> Result<off_t, Fat32Error> {
        match whence {
            file_seek::SEEK_SET | file_seek::SEEK_CUR | file_seek::SEEK_END => Ok(0),
            _ => Err(Fat32Error::InvalidArgument),
        }
    }

    /// Returns events that can complete immediately.
    pub fn poll(&self, events: c_short) -> c_short {
        const READ_EVENTS: c_short = POLLIN | POLLRDNORM;
        const WRITE_EVENTS: c_short = POLLOUT | POLLWRNORM;

        events & (READ_EVENTS | WRITE_EVENTS)
    }
}
