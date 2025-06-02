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
        file::{
            self,
            offset::RegularFileOffset,
            whence::RegularFileSeekWhence,
        },
        fs::{
            FileSystemAttributes,
            RawFileDescriptor,
        },
        RegularFileAdvice,
    },
};
use ::sys::error::Error;

//==================================================================================================
// Regular File
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
    /// Creates a new `RegularFile` from a raw file descriptor.
    ///
    /// # Parameters
    ///
    /// - `fd`: The raw file descriptor.
    ///
    /// # Returns
    ///
    /// A new `RegularFile`.
    ///
    pub(crate) const fn new(fd: RawFileDescriptor) -> Self {
        Self(fd)
    }

    ///
    /// # Description
    ///
    /// Allocates bytes in a regular file.
    ///
    /// # Parameters
    ///
    /// - `offset`: Offset in bytes.
    /// - `len`: Length in bytes.
    ///
    /// # Returns
    ///
    /// Upon successful completion, empty is returned. Otherwise, an error is returned instead.
    ///
    pub fn allocate(
        &mut self,
        offset: RegularFileOffset,
        len: RegularFileOffset,
    ) -> Result<(), Error> {
        fcntl::syscall::posix_fallocate(self.0, offset.into(), len.into())
    }

    ///
    /// # Description
    ///
    /// Retrieves the attributes of a regular file.
    ///
    /// # Returns
    ///
    /// Upon successful completion, the file attributes are returned. Otherwise, an error is
    /// returned instead.
    ///
    pub fn attributes(&self) -> Result<FileSystemAttributes, Error> {
        file::fstat(self.0)
    }

    ///
    /// # Description
    ///
    /// Provides advice about the use of a regular file.
    ///
    /// # Parameters
    ///
    /// - `advice`: The advice to provide.
    /// - `offset`: The offset in the file where the advice applies.
    /// - `length`: The length of the region in the file where the advice applies.
    ///
    /// # Returns
    ///
    /// Upon successful completion, empty is returned. Otherwise, an error is returned instead.
    ///
    pub fn advise(
        &self,
        offset: RegularFileOffset,
        length: RegularFileOffset,
        advice: RegularFileAdvice,
    ) -> Result<(), Error> {
        fcntl::syscall::posix_fadvise(self.0, offset.into(), length.into(), advice.into())
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
        file::read(self.0, buf)
    }

    ///
    /// # Description
    ///
    /// Seeks to a specific position in a regular file.
    ///
    /// # Parameters
    ///
    /// - `whence`: The reference point for the offset.
    /// - `offset`: The offset to seek to.
    ///
    /// # Returns
    ///
    /// Upon successful completion, the new offset is returned. Otherwise, an error is returned
    /// instead.
    ///
    pub fn seek(
        &mut self,
        whence: RegularFileSeekWhence,
        offset: RegularFileOffset,
    ) -> Result<RegularFileOffset, Error> {
        file::lseek(self.0, whence, offset)
    }

    ///
    /// # Description
    ///
    /// Synchronizes a regular file with the underlying storage.
    ///
    /// # Returns
    ///
    /// Upon successful completion, empty is returned. Otherwise, an error is returned instead.
    ///
    pub fn synchronize(&self) -> Result<(), Error> {
        file::fsync(self.0)
    }

    ///
    /// # Description
    ///
    /// Writes data to a regular file.
    ///
    /// # Parameters
    ///
    /// - `buf`: The buffer containing the data to write.
    ///
    /// # Returns
    ///
    /// Upon successful completion, the number of bytes written is returned. Otherwise, an error is
    /// returned instead.
    ///
    pub fn write(&mut self, buf: &[u8]) -> Result<usize, Error> {
        file::write(self.0, buf)
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
        if let Err(error) = file::close(self.0) {
            ::syslog::warn!("drop() failed to close file (error={:?})", error);
        }
    }
}
