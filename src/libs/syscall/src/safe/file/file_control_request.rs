// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::safe::{
    FileDescriptorFlags,
    FileStatusFlags,
    RawFileDescriptor,
};
use ::sysapi::{
    fcntl::file_control_request::{
        F_DUPFD,
        F_DUPFD_CLOEXEC,
        F_GETFD,
        F_GETFL,
        F_GETOWN,
        F_SETFD,
        F_SETFL,
        F_SETOWN,
    },
    ffi::c_int,
};

//==================================================================================================
// File Control Request
//==================================================================================================

///
/// # Description
///
/// This enumeration defines the various file control requests that can be made using the `fcntl`
/// system call.
///
pub enum FileControlRequest {
    /// Duplicate file descriptor.
    Duplicate(RawFileDescriptor),
    /// Get file descriptor flags.
    GetFileDescriptorFlags,
    /// Set file descriptor flags.
    SetFileDescriptorFlags(FileDescriptorFlags),
    /// Get file status flags and file access modes.
    GetFileStatusFlags,
    /// Set file status flags.
    SetFileStatusFlags(FileStatusFlags),
    /// Get owner (process or group) of the file.
    GetOwner,
    /// Set owner (process or group) of the file.
    SetOwner(c_int),
    /// Duplicate file descriptor and set the close-on-exec flag.
    DuplicateWithCloseOnExec(RawFileDescriptor),
}

impl From<FileControlRequest> for (c_int, c_int) {
    fn from(flag: FileControlRequest) -> Self {
        match flag {
            FileControlRequest::Duplicate(fd) => (F_DUPFD, fd),
            FileControlRequest::GetFileDescriptorFlags => (F_GETFD, 0),
            FileControlRequest::SetFileDescriptorFlags(flags) => (F_SETFD, flags.into()),
            FileControlRequest::GetFileStatusFlags => (F_GETFL, 0),
            FileControlRequest::SetFileStatusFlags(flags) => (F_SETFL, flags.into()),
            FileControlRequest::GetOwner => (F_GETOWN, 0),
            FileControlRequest::SetOwner(owner) => (F_SETOWN, owner),
            FileControlRequest::DuplicateWithCloseOnExec(fd) => (F_DUPFD_CLOEXEC, fd),
        }
    }
}
