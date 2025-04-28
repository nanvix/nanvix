// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::{
    message::MessagePartitioner,
    unistd::message::ChangeDirectoryRequest,
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
/// Changes the current working directory.
///
/// # Parameters
///
/// - `path`: Pathname of the new working directory.
///
/// # Returns
///
/// Upon successful completion, the `chdir()` system call returns empty. Otherwise, it returns an
/// error.
///
pub fn chdir(path: &str) -> Result<(), Error> {
    ::nvx::trace!("chdir(): path={:?}", path);

    let pid: ProcessIdentifier = ::nvx::pm::getpid()?;

    // Build request and send it.
    let request: ChangeDirectoryRequest = ChangeDirectoryRequest::new(path)?;
    let requests: Vec<Message> = request.into_parts(pid)?;
    for request in requests {
        ::nvx::ipc::send(&request)?;
    }

    // Receive response.
    let response: Message = ::nvx::ipc::recv()?;

    // Check whether system call succeeded or not.
    if response.status != 0 {
        ::nvx::error!("chdir(): failed (path={:?}, error_code={:?})", path, { response.status });
        // System call failed, parse error code and return.
        match ErrorCode::try_from(response.status) {
            // Succeeded to parse error code.
            Ok(error_code) => {
                // Return error.
                Err(Error::new(error_code, "chdir() failed"))
            },
            // Failed to parse error code, return generic error.
            Err(error) => {
                ::nvx::error!(
                    "chdir(): failed to parse error code (path={:?}, error={:?})",
                    path,
                    error
                );
                Err(Error::new(ErrorCode::TryAgain, "chdir(): failed"))
            },
        }
    } else {
        // System call succeeded, parse response.
        let message: LinuxDaemonMessage = LinuxDaemonMessage::try_from_bytes(response.payload)?;
        match message.header {
            LinuxDaemonMessageHeader::ChangeDirectoryResponse => Ok(()),
            header => {
                let reason: &str = "unexpected message header";
                ::nvx::error!("chdir(): {:?} (path={:?}, header={:?})", reason, path, header);
                Err(Error::new(ErrorCode::InvalidMessage, reason))
            },
        }
    }
}
