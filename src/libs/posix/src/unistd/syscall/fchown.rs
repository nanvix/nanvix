// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::{
    ffi::c_int,
    sys::types::{
        gid_t,
        uid_t,
    },
    unistd::message::FileChownRequest,
    LinuxDaemonMessage,
    LinuxDaemonMessageHeader,
};
use ::nvx::{
    ipc::Message,
    pm::ProcessIdentifier,
    sys::error::{
        Error,
        ErrorCode,
    },
};

//==================================================================================================
// Standalone Functions
//==================================================================================================

///
/// # Description
///
/// Changes the owner and group of a file.
///
/// # Parameters
///
/// - `fd`: File descriptor.
/// - `owner`: Owner of the file.
/// - `group`: Group of the file.
///
/// # Returns
///
/// Upon successful completion, the `fchown()` system call returns empty. Otherwise, it returns an
/// error.
///
pub fn fchown(fd: c_int, owner: uid_t, group: gid_t) -> Result<(), Error> {
    ::nvx::log!("fchown(): fd={:?}, owner={:?}, group={:?}", fd, owner, group);

    let pid: ProcessIdentifier = ::nvx::pm::getpid()?;

    // Build request and send it
    let request: Message = FileChownRequest::build(pid, fd, owner, group);
    ::nvx::ipc::send(&request)?;

    // Receive response.
    let response: Message = ::nvx::ipc::recv()?;

    // Check whether system call succeeded or not.
    if response.status != 0 {
        // System call failed, parse error code and return it.
        let error_code: ErrorCode = ErrorCode::try_from(response.status)?;
        Err(Error::new(error_code, "fchown() failed"))
    } else {
        // System call succeeded, parse response.
        let message = LinuxDaemonMessage::try_from_bytes(response.payload)?;
        match message.header {
            // Response was successfully parsed.
            LinuxDaemonMessageHeader::FileChownResponse => Ok(()),
            // Invalid response.
            _ => Err(Error::new(ErrorCode::InvalidMessage, "unexpected message header")),
        }
    }
}
