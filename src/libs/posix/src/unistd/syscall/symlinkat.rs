// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::{
    message::MessagePartitioner,
    unistd::message::SymbolicLinkAtRequest,
    LinuxDaemonMessage,
    LinuxDaemonMessageHeader,
};
use ::alloc::{
    string::ToString,
    vec::Vec,
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
/// Creates a symbolic link relative to a directory file descriptor.
///
/// # Parameters
///
/// - `target`: Path to the file to be linked.
/// - `dirfd`: Directory file descriptor.
/// - `linkpath`: Path to the new file.
///
/// # Returns
///
/// Upon successful completion, `symlinkat()` returns empty. Otherwise, it returns an error.
///
pub fn symlinkat(target: &str, dirfd: i32, linkpath: &str) -> Result<(), Error> {
    ::nvx::trace!("symlinkat(): target={:?}, dirfd={:?}, linkpath={:?}", target, dirfd, linkpath);

    let pid: ProcessIdentifier = ::nvx::pm::getpid()?;

    let request: SymbolicLinkAtRequest =
        SymbolicLinkAtRequest::new(target.to_string(), dirfd, linkpath.to_string())?;

    let requests: Vec<Message> = request.into_parts(pid)?;

    // Send request.
    for request in requests {
        ::nvx::ipc::send(&request)?;
    }

    // Receive response.
    let response: Message = ::nvx::ipc::recv()?;

    // Check whether system call succeeded or not.
    if response.status != 0 {
        ::nvx::error!(
            "symlinkat(): failed (target={:?}, dirfd={:?}, linkpath={:?}, error_code={:?})",
            target,
            dirfd,
            linkpath,
            { response.status },
        );
        // System call failed, parse error code and return.
        match ErrorCode::try_from(response.status) {
            // Succeeded to parse error code.
            Ok(error_code) => {
                // Return error.
                Err(Error::new(error_code, "symlinkat() failed"))
            },
            // Failed to parse error code, return generic error.
            Err(error) => {
                ::nvx::error!(
                    "symlinkat(): failed to parse error code (target={:?}, dirfd={:?}, \
                     linkpath={:?}, error={:?})",
                    target,
                    dirfd,
                    linkpath,
                    error
                );
                Err(Error::new(ErrorCode::TryAgain, "symlinkat(): failed"))
            },
        }
    } else {
        // System call succeeded, parse response.
        let message: LinuxDaemonMessage = LinuxDaemonMessage::try_from_bytes(response.payload)?;
        match message.header {
            // Response was successfully parsed.
            LinuxDaemonMessageHeader::SymbolicLinkAtResponse => Ok(()),
            // Response was not successfully parsed.
            header => {
                ::nvx::error!(
                    "symlinkat(): failed to parse response (target={:?}, dirfd={:?}, \
                     linkpath={:?}, header={:?})",
                    target,
                    dirfd,
                    linkpath,
                    header
                );
                Err(Error::new(ErrorCode::TryAgain, "symlinkat(): failed"))
            },
        }
    }
}
