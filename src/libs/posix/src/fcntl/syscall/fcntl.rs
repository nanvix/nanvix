// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::{
    fcntl::message::FileControlRequest,
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

pub fn fcntl(fd: i32, cmd: i32, arg: u32) -> Result<(), Error> {
    ::nvx::error!("fcntl(): fd={:?}, cmd={:?}, arg={:?}", fd, cmd, arg);

    let pid: ProcessIdentifier = ::nvx::pm::getpid()?;

    // Build request and send it.
    let request: Message = FileControlRequest::build(pid, fd, cmd, arg);
    ::nvx::ipc::send(&request)?;

    // Receive response.
    let response: Message = ::nvx::ipc::recv()?;

    // Check whether system call succeeded or not.
    if response.status == -1 {
        ::nvx::error!(
            "fcntl(): failed (fd={:?}, cmd={:?}, arg={:?}, status={:?})",
            fd,
            cmd,
            arg,
            { response.status }
        );

        // System call failed, parse error code and return it.
        match ErrorCode::try_from(response.status) {
            // Error code was successfully parsed.
            Ok(error_code) => {
                // Return error code.
                Err(Error::new(error_code, "fcntl() failed"))
            },
            // Error code was not successfully parsed.
            Err(error) => {
                ::nvx::error!(
                    "fcntl(): failed to parse error code (fd={:?}, cmd={:?}, arg={:?}, error={:?})",
                    fd,
                    cmd,
                    arg,
                    error
                );
                // Return error code.
              Err(Error::new(ErrorCode::TryAgain, "fcntl() failed"))
            },
        }
    } else {
        // System call succeeded, parse response.
        let message: LinuxDaemonMessage = LinuxDaemonMessage::try_from_bytes(response.payload)?;
        // Response was successfully parsed.
        match message.header {
            // Response was successfully parsed.
            LinuxDaemonMessageHeader::FileControlResponse => Ok(()),
            // Response was not successfully parsed.
            header => {
                ::nvx::error!(
                    "fcntl(): invalid response (fd={:?}, cmd={:?}, arg={:?}, header={:?})",
                    fd,
                    cmd,
                    arg,
                    header
                );
                Err(Error::new(ErrorCode::TryAgain, "fcntl() failed"))
            },
        }
    }
}
