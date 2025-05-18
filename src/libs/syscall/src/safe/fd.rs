// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//===================================================================================================

use crate::{
    fcntl::{
        self,
    },
    ffi::c_int,
    sys::{
        stat::{
            self,
        },
        types::mode_t,
    },
    unistd,
};
use ::nvx::sys::error::Error;

//===================================================================================================
// Raw File Descriptor
//==================================================================================================

///
/// # Description
///
/// A type alias for a raw file descriptor.
///
pub type RawFileDescriptor = c_int;

//==================================================================================================
// File Descriptor
//==================================================================================================

///
/// # Description
///
/// A structure that represents a file descriptor.
///
#[derive(Debug)]
pub struct FileDescriptor(RawFileDescriptor);

impl FileDescriptor {
    /// Opens a file descriptor.
    pub fn open(pathname: &str, flags: c_int, mode: mode_t) -> Result<Self, Error> {
        Ok(Self(fcntl::syscall::open(pathname, flags, mode)?))
    }

    /// Retrieve status information about the file descriptor.
    pub fn stat(&self, stat: &mut stat::stat) -> Result<(), Error> {
        stat::fstat(self.0, stat)
    }

    /// Reads data from the file descriptor.
    pub fn read(&self, buf: &mut [u8]) -> Result<usize, Error> {
        match unistd::syscall::read(self.0, buf) {
            Ok(n) => Ok(n as usize),
            Err(error) => Err(error),
        }
    }

    pub fn get_raw_fd(&self) -> RawFileDescriptor {
        self.0
    }
}

impl Drop for FileDescriptor {
    fn drop(&mut self) {
        // Attempt to close underlying file descriptor.
        #[cfg(feature = "syscall")]
        if let Err(error) = unistd::syscall::close(self.0) {
            ::syslog::warn!("drop() failed to close file (error={:?})", error);
        }
    }
}
