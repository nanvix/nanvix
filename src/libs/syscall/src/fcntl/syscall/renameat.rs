// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::{
    fcntl::message::RenameAtRequest,
    message::MessagePartitioner,
    safe::RawFileDescriptor,
    LinuxDaemonMessage,
    LinuxDaemonMessageHeader,
};
use ::alloc::vec::Vec;
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
/// Renames a file relative to a directory file descriptor.
///
/// # Parameters
///
/// - `olddirfd`: Directory file descriptor of the old file.
/// - `oldpath`:  Pathname of the old file.
/// - `newdirfd`: Directory file descriptor of the new file.
/// - `newpath`:  Pathname of the new file.
///
/// # Returns
///
/// Upon successful completion, the `renameat()` system call returns empty. Otherwise, it returns an
/// error.
///
pub fn renameat(
    olddirfd: RawFileDescriptor,
    oldpath: &str,
    newdirfd: RawFileDescriptor,
    newpath: &str,
) -> Result<(), Error> {
    ::syslog::trace!(
        "renameat(): olddirfd={:?}, oldpath={:?}, newdirfd={:?}, newpath={:?}",
        olddirfd,
        oldpath,
        newdirfd,
        newpath
    );

    let pid: ProcessIdentifier = ::nvx::pm::getpid()?;

    // Build request and send it.
    let request: RenameAtRequest = RenameAtRequest::new(olddirfd, oldpath, newdirfd, newpath)?;
    let requests: Vec<Message> = request.into_parts(pid)?;
    for request in requests {
        ::nvx::ipc::send(&request)?;
    }

    // Receive response.
    let response: Message = ::nvx::ipc::recv()?;

    // Check whether system call succeeded or not.
    if response.status != 0 {
        ::syslog::error!(
            "renameat(): failed (olddirfd={:?}, oldpath={:?}, newdirfd={:?}, newpath={:?}, \
             error_code={:?})",
            olddirfd,
            oldpath,
            newdirfd,
            newpath,
            { response.status }
        );
        // System call failed, parse error code and return.
        match ErrorCode::try_from(response.status) {
            // Succeeded to parse error code.
            Ok(error_code) => {
                // Return error.
                Err(Error::new(error_code, "renameat() failed"))
            },
            // Failed to parse error code, return generic error.
            Err(error) => {
                ::syslog::error!(
                    "renameat(): failed to parse error code (olddirfd={:?}, oldpath={:?}, \
                     newdirfd={:?}, newpath={:?}, error={:?})",
                    olddirfd,
                    oldpath,
                    newdirfd,
                    newpath,
                    error
                );
                Err(Error::new(ErrorCode::TryAgain, "renameat(): failed"))
            },
        }
    } else {
        // System call succeeded, parse response.
        let message: LinuxDaemonMessage = LinuxDaemonMessage::try_from_bytes(response.payload)?;
        match message.header {
            LinuxDaemonMessageHeader::RenameAtResponse => Ok(()),
            header => {
                let reason: &str = "unexpected message header";
                ::syslog::error!(
                    "renameat(): {:?} (olddirfd={:?}, oldpath={:?}, newdirfd={:?}, newpath={:?}, \
                     header={:?})",
                    reason,
                    olddirfd,
                    oldpath,
                    newdirfd,
                    newpath,
                    header
                );
                Err(Error::new(ErrorCode::InvalidMessage, reason))
            },
        }
    }
}
