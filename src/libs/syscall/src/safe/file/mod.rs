// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Modules
//==================================================================================================

mod advice;
mod offset;
mod oflags;
mod regular;
mod stdio;
mod whence;

//==================================================================================================
// Imports
//===================================================================================================

use crate::{
    safe::fs::RawFileDescriptor,
    unistd,
};
use ::sys::error::Error;

//==================================================================================================
// Exports
//==================================================================================================

pub use advice::RegularFileAdvice;
pub use offset::RegularFileOffset;
pub use oflags::RegularFileOpenFlags;
pub use regular::RegularFile;
pub use stdio::{
    StandardError,
    StandardInput,
    StandardOutput,
};
pub use whence::RegularFileSeekWhence;

//==================================================================================================
// Standalone Functions
//==================================================================================================

///
/// # Description
///
/// Seeks to a specific position in a regular file.
///
/// # Parameters
///
/// - `fd`: Raw file descriptor to the regular file which to seek.
/// - `whence`: The reference point for the offset.
/// - `offset`: The offset to seek to.
///
/// # Returns
///
/// Upon successful completion, the new offset is returned. Otherwise, an error is returned
/// instead.
///
pub fn lseek(
    fd: RawFileDescriptor,
    whence: RegularFileSeekWhence,
    offset: RegularFileOffset,
) -> Result<RegularFileOffset, Error> {
    match unistd::syscall::lseek(fd, offset.into(), whence.into()) {
        Ok(new_offset) => Ok(RegularFileOffset::from(new_offset)),
        Err(error) => Err(error),
    }
}

///
/// # Description
///
/// Reads data from a regular file.
///
/// # Parameters
///
/// - `fd`: Raw file descriptor to the regular file from which to read.
/// - `buf`: The buffer to store the data.
///
/// # Returns
///
/// Upon successful completion, the number of bytes read is returned. Otherwise, an error is
/// returned instead.
///
pub fn read(fd: RawFileDescriptor, buf: &mut [u8]) -> Result<usize, Error> {
    match unistd::syscall::read(fd, buf) {
        Ok(n) => Ok(n as usize),
        Err(error) => Err(error),
    }
}

///
/// # Description
///
/// Synchronizes a regular file with the underlying storage.
///
/// # Parameters
///
/// - `fd`: Raw file descriptor to the regular file to synchronize.
///
/// # Returns
///
/// Upon successful completion, empty is returned. Otherwise, an error is returned instead.
///
pub fn fsync(fd: RawFileDescriptor) -> Result<(), Error> {
    unistd::syscall::fsync(fd)
}

///
/// # Description
///
/// Writes data to a regular file.
///
/// # Parameters
///
/// - `fd`: Raw file descriptor to the regular file to which to write.
/// - `buf`: The buffer containing the data to write.
///
/// # Returns
///
/// Upon successful completion, the number of bytes written is returned. Otherwise, an error is
/// returned instead.
///
pub fn write(fd: RawFileDescriptor, buf: &[u8]) -> Result<usize, Error> {
    match unistd::syscall::write(fd, buf) {
        Ok(n) => Ok(n as usize),
        Err(error) => Err(error),
    }
}
