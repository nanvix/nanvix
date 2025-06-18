// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::{
    message::MessagePartitioner,
    safe::RawFileDescriptor,
    unistd::message::LinkAtRequest,
    LinuxDaemonMessage,
    LinuxDaemonMessageHeader,
};
use ::alloc::{
    string::ToString,
    vec::Vec,
};
use ::sys::{
    error::{
        Error,
        ErrorCode,
    },
    ipc::Message,
    pm::ProcessIdentifier,
};
use ::sysapi::ffi::c_int;

//==================================================================================================
// Standalone Functions
//==================================================================================================

///
/// # Description
///
/// Creates a new hard link to an existing file relative to a directory file descriptor.
///
/// # Parameters
///
/// - `olddirfd`: Directory file descriptor of the existing file.
/// - `oldpath`: Path to the existing file.
/// - `newdirfd`: Directory file descriptor of the new file.
/// - `newpath`: Path to the new file.
/// - `flags`: Flags to control the behavior of the system call.
///
/// # Returns
///
/// Upon successful completion, `linkat()` returns empty. Otherwise, it returns an error.
///
pub fn linkat(
    olddirfd: RawFileDescriptor,
    oldpath: &str,
    newdirfd: RawFileDescriptor,
    newpath: &str,
    flags: c_int,
) -> Result<(), Error> {
    ::syslog::trace!(
        "linkat(): olddirfd={}, oldpath={}, newdirfd={}, newpath={}, flags={}",
        olddirfd,
        oldpath,
        newdirfd,
        newpath,
        flags
    );

    let pid: ProcessIdentifier = crate::unistd::getpid()?;

    let request: LinkAtRequest =
        LinkAtRequest::new(olddirfd, oldpath.to_string(), newdirfd, newpath.to_string(), flags)?;

    let requests: Vec<Message> = request.into_parts(pid)?;

    // Send request.
    for request in requests {
        ::sys::kcall::ipc::send(&request)?;
    }

    // Receive response.
    let response: Message = ::sys::kcall::ipc::recv()?;

    // Check whether system call succeeded or not.
    if response.status != 0 {
        ::syslog::error!(
            "linkat(): failed (olddirfd={}, oldpath={}, newdirfd={}, newpath={}, flags={}, \
             error={})",
            olddirfd,
            oldpath,
            newdirfd,
            newpath,
            flags,
            { response.status },
        );
        // System call failed, parse error code and return.
        match ErrorCode::try_from(response.status) {
            // Error code was successfully parsed.
            Ok(error_code) => {
                // Return error.
                Err(Error::new(error_code, "linkat() failed"))
            },
            // Error code was not successfully parsed.
            Err(error) => {
                ::syslog::error!(
                    "linkat(): failed to parse error code (olddirfd={}, oldpath={}, newdirfd={}, \
                     newpath={}, flags={}, error={:?})",
                    olddirfd,
                    oldpath,
                    newdirfd,
                    newpath,
                    flags,
                    error
                );
                Err(Error::new(ErrorCode::TryAgain, "linkat(): failed"))
            },
        }
    } else {
        // System call succeeded, parse response.
        let message: LinuxDaemonMessage = LinuxDaemonMessage::try_from_bytes(response.payload)?;
        // Response was successfully parsed.
        match message.header {
            // Response was successfully parsed.
            LinuxDaemonMessageHeader::LinkAtResponse => Ok(()),
            // Response was not successfully parsed.
            header => {
                let reason: &str = "unexpected message header";
                ::syslog::error!(
                    "linkat(): {:?} (olddirfd={}, oldpath={}, newdirfd={}, newpath={}, flags={}, \
                     header={:?})",
                    reason,
                    olddirfd,
                    oldpath,
                    newdirfd,
                    newpath,
                    flags,
                    header
                );
                Err(Error::new(ErrorCode::InvalidMessage, reason))
            },
        }
    }
}
