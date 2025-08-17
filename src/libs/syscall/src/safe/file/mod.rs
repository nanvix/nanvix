// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Modules
//==================================================================================================

mod advice;
mod file_control_request;
mod offset;
mod oflags;
mod regular;
mod stdio;
mod whence;

//==================================================================================================
// Imports
//===================================================================================================

use crate::{
    fcntl,
    safe::{
        fs::RawFileDescriptor,
        time::Time,
        FileSystemAttributes,
        FileSystemPermissions,
    },
    sys::{
        self,
    },
    unistd,
};
use ::core::ffi::c_int;
use ::sys::error::Error;

//==================================================================================================
// Exports
//==================================================================================================

pub use advice::RegularFileAdvice;
pub use file_control_request::FileControlRequest;
pub use offset::RegularFileOffset;
pub use oflags::RegularFileOpenFlags;
pub use regular::RegularFile;
pub use stdio::{
    StandardError,
    StandardInput,
    StandardOutput,
};
use sysapi::{
    sys_stat,
    sys_types::mode_t,
    time::timespec,
};
pub use whence::RegularFileSeekWhence;

//==================================================================================================
// Standalone Functions
//==================================================================================================

///
/// # Description
///
/// Closes a file descriptor.
///
/// # Parameters
///
/// - `fd`: Raw file descriptor to the file to close.
///
/// # Returns
///
/// Upon successful completion, empty is returned. Otherwise, an error is returned instead.
///
pub fn close(fd: RawFileDescriptor) -> Result<(), Error> {
    unistd::syscall::close(fd)
}

///
/// # Description
///
/// Changes the access permissions of a file descriptor.
///
/// # Parameters
///
/// - `fd`: Raw file descriptor to the file for which to change access permissions.
/// - `mode`: The new access permissions for the file.
///
/// # Returns
///
/// Upon successful completion, empty is returned. Otherwise, an error is returned instead.
///
pub fn fchmod(fd: RawFileDescriptor, permissions: FileSystemPermissions) -> Result<(), Error> {
    let mode: mode_t = permissions.into();
    sys::stat::fchmod(fd, mode)
}

///
/// # Description
///
/// Performs a control operation on a file descriptor.
///
/// # Parameters
///
/// - `fd`: Raw file descriptor to the file on which to perform the control operation.
/// - `cmd`: The control operation to perform.
/// - `arg`: An argument for the control operation, if required.
///
/// # Returns
///
/// Upon successful completion, empty is returned. Otherwise, an error is returned instead.
///
pub fn fcntl(fd: RawFileDescriptor, cmd: &FileControlRequest) -> Result<c_int, Error> {
    let (cmd, arg): (c_int, Option<c_int>) = cmd.into();
    fcntl::syscall::fcntl(fd, cmd, arg)
}

///
/// # Description
///
/// Synchronizes the data of a file descriptor to disk.
///
/// # Parameters
///
/// - `fd`: Raw file descriptor to the file to synchronize.
///
/// # Returns
///
/// Upon successful completion, empty is returned. Otherwise, an error is returned instead.
///
pub fn fdatasync(fd: RawFileDescriptor) -> Result<(), Error> {
    unistd::syscall::fdatasync(fd)
}

///
/// # Description
///
/// Retrieves file status information
///
/// # Parameters
///
/// - `fd`: Raw file descriptor to the file for which to retrieve status information.
///
/// # Returns
///
/// Upon successful completion, the file status information is returned. Otherwise, an error is
/// returned instead.
///
pub fn fstat(fd: RawFileDescriptor) -> Result<FileSystemAttributes, Error> {
    let mut st: sys_stat::stat = sys_stat::stat::default();
    sys::stat::fstat(fd, &mut st)?;
    Ok(FileSystemAttributes::from(st))
}

///
/// # Description
///
/// Sets access and modification times of a file descriptor.
///
/// # Parameters
///
/// - `fd`: Raw file descriptor to the file for which to set access and modification times.
/// - `times`: Access and modification times to set.
///
/// # Returns
///
/// Upon successful completion, empty is returned. Otherwise, an error is returned instead.
///
pub fn futimens(fd: RawFileDescriptor, times: &[Time; 2]) -> Result<(), Error> {
    let times: [timespec; 2] = [times[0].into(), times[1].into()];
    sys::stat::futimens(fd, &times)
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
