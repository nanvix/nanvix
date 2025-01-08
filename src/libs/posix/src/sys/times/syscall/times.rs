// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::{
    sys::{
        times::{
            message::{
                TimesRequest,
                TimesResponse,
            },
            tms,
        },
        types::clock_t,
    },
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
/// Gets the current process times.
///
/// # Parameters
///
/// - `buffer`: Buffer to store the times.
///
/// # Returns
///
/// Upon successful completion, the `times()` system call returns the elapsed time since an
/// arbitrary point in the past. Otherwise, an error code is returned.
///
pub fn times(buffer: Option<&mut tms>) -> Result<clock_t, Error> {
    ::nvx::log!("times(): {:?}", buffer);

    let pid: ProcessIdentifier = ::nvx::pm::getpid()?;

    // Build request and send it.
    let request: Message = TimesRequest::build(pid)?;
    ::nvx::ipc::send(&request)?;

    // Receive response.
    let response: Message = ::nvx::ipc::recv()?;

    // Check wether system call succeeded or not.
    if response.status != 0 {
        let error_code: ErrorCode = ErrorCode::try_from(response.status)?;
        return Err(Error::new(error_code, "times() failed"));
    } else {
        // System call succeeded, parse response.
        let message: LinuxDaemonMessage = LinuxDaemonMessage::try_from_bytes(response.payload)?;

        match message.header {
            LinuxDaemonMessageHeader::TimesResponse => {
                // Parse response.
                let response: TimesResponse = TimesResponse::from_bytes(message.payload);

                // Copy data to buffer.
                let elapsed: clock_t = response.elapsed;
                if let Some(buffer) = buffer {
                    buffer.tms_utime = response.buffer.tms_utime;
                    buffer.tms_stime = response.buffer.tms_stime;
                    buffer.tms_cutime = response.buffer.tms_cutime;
                    buffer.tms_cstime = response.buffer.tms_cstime;
                }

                Ok(elapsed)
            },
            _ => return Err(Error::new(ErrorCode::InvalidMessage, "unexpected message header")),
        }
    }
}
