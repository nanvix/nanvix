// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use ::sys::error::Error;
use ::sysapi::{
    ffi::c_int,
    sys_types::mode_t,
};
#[cfg(not(feature = "standalone"))]
use {
    crate::{
        message::MessagePartitioner,
        sys::stat::message::FileChmodAtRequest,
        LinuxDaemonMessage,
        LinuxDaemonMessageHeader,
    },
    ::alloc::vec::Vec,
    ::sys::{
        error::ErrorCode,
        ipc::Message,
        pm::ThreadIdentifier,
    },
};

//==================================================================================================
// Standalone Functions
//==================================================================================================

///
/// # Description
///
/// Changes the mode of a file relative to a directory file descriptor.
///
/// # Parameters
///
/// - `dirfd`: Directory file descriptor.
/// - `path`:  Pathname of the file.
/// - `mode`:  Mode.
/// - `flag`:  Flag.
///
/// # Returns
///
/// Upon successful completion, the `fchmodat()` system call returns empty. Otherwise, it returns an
/// error.
///
#[allow(unreachable_code)]
pub fn fchmodat(_dirfd: c_int, _path: &str, _mode: mode_t, _flag: c_int) -> Result<(), Error> {
    // In standalone mode, this operation is not supported.
    // TODO: https://github.com/nanvix/nanvix/issues/1607
    #[cfg(feature = "standalone")]
    {
        Ok(())
    }

    // Forward to linuxd via IPC.
    #[cfg(not(feature = "standalone"))]
    fchmodat_linuxd(_dirfd, _path, _mode, _flag)
}

/// Forwards a `fchmodat` request to linuxd via IPC.
#[cfg(not(feature = "standalone"))]
fn fchmodat_linuxd(dirfd: c_int, path: &str, mode: mode_t, flag: c_int) -> Result<(), Error> {
    let tid: ThreadIdentifier = ::sys::kcall::pm::gettid()?;

    let request: FileChmodAtRequest = FileChmodAtRequest::new(dirfd, mode, flag, path)?;

    let requests: Vec<Message> = request.into_parts(tid)?;

    for request in &requests {
        ::sys::kcall::ipc::send(request)?;
    }

    let response: Message = ::sys::kcall::ipc::recv()?;

    // Check whether system call succeeded or not.
    if response.status != 0 {
        let error_code: ErrorCode = ErrorCode::try_from(response.status)?;
        Err(Error::new(error_code, "fchmodat() failed"))
    } else {
        // System call succeeded, parse response.
        let message: LinuxDaemonMessage = LinuxDaemonMessage::try_from_bytes(response.payload)?;

        match message.header {
            LinuxDaemonMessageHeader::FileChmodAtResponse => Ok(()),
            _ => Err(Error::new(ErrorCode::InvalidMessage, "unexpected message header")),
        }
    }
}
