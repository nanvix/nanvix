// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Modules
//==================================================================================================

use crate::{
    time::{
        clockid_t,
        message::{
            GetClockTimeRequest,
            GetClockTimeResponse,
        },
        timespec,
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
/// Gets clock time.
///
/// # Parameters
///
/// - `clock_id`: The identifier of the clock to be used.
/// - `tp`: The structure where the time is stored.
///
/// # Returns
///
/// Upon successful completion, empty is returned. Upon failure, an error is returned instead.
///
pub fn clock_gettime(clock_id: clockid_t, tp: Option<&mut timespec>) -> Result<(), Error> {
    ::nvx::log!("clock_gettime():clock_id={:?}, tp={:?}", clock_id, tp);

    let pid: ProcessIdentifier = ::nvx::pm::getpid()?;

    // Build request and send it.
    let request: Message = GetClockTimeRequest::build(pid, clock_id);
    ::nvx::ipc::send(&request)?;

    // Receive response.
    let response: Message = ::nvx::ipc::recv()?;

    // Check whether system call succeeded or not.
    if response.status != 0 {
        let error_code: ErrorCode = ErrorCode::try_from(response.status)?;
        Err(Error::new(error_code, "clock_gettime() failed"))
    } else {
        // System call succeeded, parse response.
        let message: LinuxDaemonMessage = LinuxDaemonMessage::try_from_bytes(response.payload)?;
        // Response was successfully parsed.
        match message.header {
            LinuxDaemonMessageHeader::GetClockTimeResponse => {
                let response: GetClockTimeResponse =
                    GetClockTimeResponse::from_bytes(message.payload);

                // Copy time if requested.
                if let Some(tp) = tp {
                    tp.tv_sec = response.tp.tv_sec;
                    tp.tv_nsec = response.tp.tv_nsec;
                }
                Ok(())
            },
            // Unexpected response message.
            _ => Err(Error::new(ErrorCode::InvalidMessage, "unexpected response message")),
        }
    }
}
