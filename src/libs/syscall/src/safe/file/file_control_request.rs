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
use ::core::ffi::VaListImpl;
use ::sys::error::{
    Error,
    ErrorCode,
};
use ::sysapi::{
    fcntl::file_control_request::{
        F_DUPFD,
        F_DUPFD_CLOEXEC,
        F_DUPFD_CLOFORK,
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

#[derive(Debug)]
pub enum SocketOwner {
    /// Invalid owner.
    Invalid,
    /// Process owner.
    Process(c_int),
    /// Group owner.
    Group(c_int),
}

impl From<c_int> for SocketOwner {
    fn from(owner: c_int) -> Self {
        if owner < 0 {
            if owner == -1 {
                SocketOwner::Invalid
            } else {
                SocketOwner::Group(owner)
            }
        } else {
            SocketOwner::Process(owner)
        }
    }
}

impl From<&SocketOwner> for c_int {
    fn from(owner: &SocketOwner) -> Self {
        match owner {
            SocketOwner::Invalid => -1,
            SocketOwner::Process(pid) => *pid,
            SocketOwner::Group(gid) => *gid,
        }
    }
}

///
/// # Description
///
/// This enumeration defines the various file control requests that can be made using the `fcntl`
/// system call.
///
#[derive(Debug)]
pub enum FileControlRequest {
    /// Duplicate file descriptor.
    Duplicate(RawFileDescriptor),
    /// Duplicate file descriptor and set the close-on-exec flag.
    DuplicateWithCloseOnExec(RawFileDescriptor),
    /// Duplicate file descriptor and set the close-on-fork flag.
    DuplicateWithCloseOnFork(RawFileDescriptor),
    /// Get file descriptor flags.
    GetFileDescriptorFlags,
    /// Set file descriptor flags.
    SetFileDescriptorFlags(FileDescriptorFlags),
    /// Get file status flags and file access modes.
    GetFileStatusFlags,
    /// Set file status flags.
    SetFileStatusFlags(FileStatusFlags),
    /// Get owner (process or group) of the file.
    GetSocketOwner,
    /// Set owner (process or group) of the file.
    SetSocketOwner(SocketOwner),
}

impl From<&FileControlRequest> for (c_int, Option<c_int>) {
    fn from(flag: &FileControlRequest) -> Self {
        match flag {
            FileControlRequest::Duplicate(fd) => (F_DUPFD, Some(*fd)),
            FileControlRequest::DuplicateWithCloseOnExec(fd) => (F_DUPFD_CLOEXEC, Some(*fd)),
            FileControlRequest::DuplicateWithCloseOnFork(fd) => (F_DUPFD_CLOFORK, Some(*fd)),
            FileControlRequest::GetFileDescriptorFlags => (F_GETFD, None),
            FileControlRequest::SetFileDescriptorFlags(flags) => (F_SETFD, Some(flags.into())),
            FileControlRequest::GetFileStatusFlags => (F_GETFL, None),
            FileControlRequest::SetFileStatusFlags(flags) => (F_SETFL, Some(flags.into())),
            FileControlRequest::GetSocketOwner => (F_GETOWN, None),
            FileControlRequest::SetSocketOwner(owner) => (F_SETOWN, Some(owner.into())),
        }
    }
}

impl TryFrom<(c_int, Option<c_int>)> for FileControlRequest {
    type Error = Error;

    fn try_from(value: (c_int, Option<c_int>)) -> Result<Self, Self::Error> {
        match value {
            (F_DUPFD, Some(fd)) => Ok(FileControlRequest::Duplicate(fd)),
            (F_DUPFD_CLOEXEC, Some(fd)) => Ok(FileControlRequest::DuplicateWithCloseOnExec(fd)),
            (F_DUPFD_CLOFORK, Some(fd)) => Ok(FileControlRequest::DuplicateWithCloseOnFork(fd)),
            (F_GETFD, None) => Ok(FileControlRequest::GetFileDescriptorFlags),
            (F_SETFD, Some(flags)) => {
                let fd_flags: FileDescriptorFlags = FileDescriptorFlags::try_from(flags)?;
                Ok(FileControlRequest::SetFileDescriptorFlags(fd_flags))
            },
            (F_GETFL, None) => Ok(FileControlRequest::GetFileStatusFlags),
            (F_SETFL, Some(flags)) => {
                let fd_flags: FileStatusFlags = FileStatusFlags::try_from(flags)?;
                Ok(FileControlRequest::SetFileStatusFlags(fd_flags))
            },
            (F_GETOWN, None) => Ok(FileControlRequest::GetSocketOwner),
            (F_SETOWN, Some(owner)) => {
                Ok(FileControlRequest::SetSocketOwner(SocketOwner::from(owner)))
            },
            (_cmd, _arg) => {
                Err(Error::new(ErrorCode::InvalidArgument, "unsupported file control request"))
            },
        }
    }
}

impl<'a> TryFrom<(c_int, VaListImpl<'a>)> for FileControlRequest {
    type Error = Error;

    fn try_from(value: (c_int, VaListImpl<'a>)) -> Result<Self, Self::Error> {
        let (cmd, mut arg) = value;
        match cmd {
            F_DUPFD => {
                let fd: c_int = unsafe { arg.arg() };
                ::syslog::debug!("F_DUPFD: fd={fd:?}");
                Ok(FileControlRequest::Duplicate(fd))
            },
            F_DUPFD_CLOEXEC => {
                let fd: c_int = unsafe { arg.arg() };
                ::syslog::debug!("F_DUPFD_CLOEXEC: fd={fd:?}");
                Ok(FileControlRequest::DuplicateWithCloseOnExec(fd))
            },
            F_DUPFD_CLOFORK => {
                let fd: c_int = unsafe { arg.arg() };
                ::syslog::debug!("F_DUPFD_CLOFORK: fd={fd:?}");
                Ok(FileControlRequest::DuplicateWithCloseOnFork(fd))
            },
            F_GETFD => Ok(FileControlRequest::GetFileDescriptorFlags),
            F_SETFD => {
                let flags: c_int = unsafe { arg.arg() };
                ::syslog::debug!("F_SETFD: flags={flags:?}");
                let fd_flags: FileDescriptorFlags = FileDescriptorFlags::try_from(flags)?;
                Ok(FileControlRequest::SetFileDescriptorFlags(fd_flags))
            },
            F_GETFL => Ok(FileControlRequest::GetFileStatusFlags),
            F_SETFL => {
                let flags: c_int = unsafe { arg.arg() };
                ::syslog::debug!("F_SETFL: flags={flags:?}");
                let status_flags: FileStatusFlags = FileStatusFlags::try_from(flags)?;
                Ok(FileControlRequest::SetFileStatusFlags(status_flags))
            },
            F_GETOWN => Ok(FileControlRequest::GetSocketOwner),
            F_SETOWN => {
                let owner: c_int = unsafe { arg.arg() };
                ::syslog::debug!("F_SETOWN: owner={owner:?}");
                Ok(FileControlRequest::SetSocketOwner(SocketOwner::from(owner)))
            },
            _arg => Err(Error::new(ErrorCode::InvalidArgument, "unsupported file control request")),
        }
    }
}
