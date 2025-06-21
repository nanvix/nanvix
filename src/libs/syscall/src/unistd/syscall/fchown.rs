// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::{
    safe::RawFileDescriptor,
    unistd::message::FileChownRequest,
    LinuxDaemonMessage,
    LinuxDaemonMessageHeader,
};
use ::sys::{
    error::{
        Error,
        ErrorCode,
    },
    ipc::Message,
    pm::ProcessIdentifier,
};
use ::sysapi::sys_types::{
    gid_t,
    uid_t,
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
/// Upon successful completion, `fchown()` returns empty. Otherwise, it returns an error.
///
pub fn fchown(fd: RawFileDescriptor, owner: uid_t, group: gid_t) -> Result<(), Error> {
    ::syslog::trace!("fchown(): fd={:?}, owner={:?}, group={:?}", fd, owner, group);

    let pid: ProcessIdentifier = crate::unistd::getpid()?;

    // Build request and send it
    let request: Message = FileChownRequest::build(pid, fd, owner, group);
    ::sys::kcall::ipc::send(&request)?;

    // Receive response.
    let response: Message = ::sys::kcall::ipc::recv()?;

    // Check whether system call succeeded or not.
    if response.status != 0 {
        ::syslog::error!(
            "fchown(): failed (fd={:?}, owner={:?}, group={:?}, status={:?})",
            fd,
            owner,
            group,
            { response.status }
        );

        match ErrorCode::try_from(response.status) {
            // System call failed, return error.
            Ok(error_code) => Err(Error::new(error_code, "system call failed")),
            // Invalid error code.
            Err(_) => Err(Error::new(ErrorCode::InvalidMessage, "invalid error code received")),
        }
    } else {
        // System call succeeded, parse response.
        let message = LinuxDaemonMessage::try_from_bytes(response.payload)?;
        match message.header {
            // Response was successfully parsed.
            LinuxDaemonMessageHeader::FileChownResponse => Ok(()),
            // Invalid response.
            header => {
                ::syslog::error!(
                    "fchown(): invalid response (fd={:?}, owner={:?}, group={:?}, header={:?})",
                    fd,
                    owner,
                    group,
                    header
                );
                Err(Error::new(ErrorCode::InvalidMessage, "invalid response"))
            },
        }
    }
}
