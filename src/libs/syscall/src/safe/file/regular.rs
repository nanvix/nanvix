// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//===================================================================================================

use crate::{
    fcntl::{
        self,
    },
    safe::{
        file::RegularFileOpenFlags,
        fs::{
            FileSystemAttributes,
            FileSystemPath,
            FileSystemPermissions,
            RawFileDescriptor,
        },
    },
    sys::{
        stat::{
            self,
        },
        types::mode_t,
    },
    unistd,
};
use ::nvx::sys::error::Error;

//==================================================================================================
// File Descriptor
//==================================================================================================

///
/// # Description
///
/// A structure that represents a regular file in the file system.
///
#[derive(Debug)]
pub struct RegularFile(RawFileDescriptor);

impl RegularFile {
    ///
    /// # Description
    ///
    /// Opens a regular file in the file system.
    ///
    /// # Parameters
    ///
    /// - `pathname`: The path to the file.
    /// - `flags`: The flags to open the file.
    /// - `permissions`: File permissions when creating a new file.
    ///
    /// # Returns
    ///
    /// Upon successful completion, a `RegularFile` structure is returned. Otherwise, an error is
    /// returned instead.
    ///
    pub fn open(
        pathname: &FileSystemPath,
        flags: RegularFileOpenFlags,
        permissions: Option<FileSystemPermissions>,
    ) -> Result<Self, Error> {
        let mode: mode_t = match permissions {
            Some(permissions) => permissions.into(),
            None => 0,
        };
        Ok(Self(fcntl::syscall::open(pathname.as_str(), flags.into(), mode)?))
    }

    ///
    /// # Description
    ///
    /// Retrieves the attributes of a regular file.
    ///
    /// # Parameters
    ///
    /// - `attributes`: The structure to store the file attributes.
    ///
    /// # Returns
    ///
    /// Upon successful completion, the file attributes are stored in `attributes` and empty is
    /// returned.  Otherwise, an error is returned instead.
    ///
    pub fn attributes(&self, attributes: &mut FileSystemAttributes) -> Result<(), Error> {
        stat::fstat(self.0, attributes.as_raw_mut())
    }

    ///
    /// # Description
    ///
    /// Reads data from a regular file.
    ///
    /// # Parameters
    ///
    /// - `buf`: The buffer to store the data.
    ///
    /// # Returns
    ///
    /// Upon successful completion, the number of bytes read is returned. Otherwise, an error is
    /// returned instead.
    ///
    pub fn read(&self, buf: &mut [u8]) -> Result<usize, Error> {
        match unistd::syscall::read(self.0, buf) {
            Ok(n) => Ok(n as usize),
            Err(error) => Err(error),
        }
    }

    ///
    /// # Description
    ///
    /// Casts `self` to a raw file descriptor.
    ///
    /// # Returns
    ///
    /// A raw file descriptor.
    ///
    pub fn as_raw_fd(&self) -> RawFileDescriptor {
        self.0
    }
}

impl Drop for RegularFile {
    fn drop(&mut self) {
        // Attempt to close underlying file descriptor.
        if let Err(error) = unistd::syscall::close(self.0) {
            ::syslog::warn!("drop() failed to close file (error={:?})", error);
        }
    }
}
