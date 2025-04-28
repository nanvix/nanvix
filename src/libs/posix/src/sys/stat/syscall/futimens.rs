// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::{
    sys::stat::message::UpdateFileAccessTimeRequest,
    time::timespec,
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

pub fn futimens(fd: i32, times: [timespec; 2]) -> Result<(), Error> {
    ::nvx::error!("futimens(): fd={:?}, times={:?}", fd, times);

    let pid: ProcessIdentifier = ::nvx::pm::getpid()?;

    // Build request and send it.
    let request: Message = UpdateFileAccessTimeRequest::build(pid, fd, times);
    ::nvx::ipc::send(&request)?;

    // Receive response.
    let response: Message = ::nvx::ipc::recv()?;

    // Check whether system call succeeded or not.
    if response.status != 0 {
        ::nvx::error!("futimens(): failed (fd={:?}, times={:?}, status={:?})", fd, times, {
            response.status
        });

        // System call failed, parse error code and return it.
        match ErrorCode::try_from(response.status) {
            // Error code was successfully parsed.
            Ok(error_code) => {
                // Return error code.
                Err(Error::new(error_code, "futimens() failed"))
            },
            // Error code was not successfully parsed.
            Err(error) => {
                ::nvx::error!(
                    "futimens(): failed to parse error code (fd={:?}, times={:?}, error={:?})",
                    fd,
                    times,
                    error
                );
                Err(Error::new(ErrorCode::TryAgain, "futimens() failed"))
            },
        }
    } else {
        // System call succeeded, parse response.
        let message: LinuxDaemonMessage = LinuxDaemonMessage::try_from_bytes(response.payload)?;
        // Response was successfully parsed.
        match message.header {
            // Response was successfully parsed.
            LinuxDaemonMessageHeader::UpdateFileAccessTimeResponse => Ok(()),
            // Response was not successfully parsed.
            header => {
                ::nvx::error!(
                    "futimens(): invalid response (fd={:?}, times={:?}, header={:?})",
                    fd,
                    times,
                    header
                );
                Err(Error::new(ErrorCode::TryAgain, "futimens() failed"))
            },
        }
    }
}
