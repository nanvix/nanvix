// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Modules
//==================================================================================================

use crate::{
    time::{
        clockid_t,
        message::{
            ClockGetResolutionResponse,
            ClockResolutionRequest,
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
/// Gets the resolution of the specified clock.
///
/// # Parameters
///
/// - `clock_id`: The clock ID.
/// - `res`: The structure where the resolution is stored.
///
/// # Returns
///
/// Upon successful completion, `clock_getres()` returns empty. Otherwise, it returns an error.
///
pub fn clock_getres(clock_id: clockid_t, res: &mut Option<&mut timespec>) -> Result<(), Error> {
    ::nvx::error!("clock_getres(): clock_id={:?}, res={:?}", clock_id, res);

    let pid: ProcessIdentifier = ::nvx::pm::getpid()?;

    // Build request and send it.
    let request: Message = ClockResolutionRequest::build(pid, clock_id);
    ::nvx::ipc::send(&request)?;

    // Receive response.
    let response: Message = ::nvx::ipc::recv()?;

    // Check whether system call succeeded or not.
    if response.status != 0 {
        ::nvx::error!(
            "clock_getres(): failed (clock_id={:?}, res={:?}, status={:?})",
            clock_id,
            res,
            { response.status }
        );
        // System call failed, parse error code and return it.
        match ErrorCode::try_from(response.status) {
            // Error code was successfully parsed.
            Ok(error_code) => {
                // Return error code.
                Err(Error::new(error_code, "clock_getres() failed"))
            },
            // Error code was not successfully parsed.
            Err(error) => {
                ::nvx::error!(
                    "clock_getres(): failed to parse error code (clock_id={:?}, res={:?}, \
                     error={:?})",
                    clock_id,
                    res,
                    error
                );
                Err(Error::new(ErrorCode::TryAgain, "clock_getres() failed"))
            },
        }
    } else {
        // System call succeeded, parse response.
        let message: LinuxDaemonMessage = LinuxDaemonMessage::try_from_bytes(response.payload)?;
        // Response was successfully parsed.
        match message.header {
            LinuxDaemonMessageHeader::GetClockResolutionResponse => {
                let response: ClockGetResolutionResponse =
                    ClockGetResolutionResponse::from_bytes(message.payload);

                // Copy resolution if requested.
                if let Some(res) = res {
                    res.tv_sec = response.res.tv_sec;
                    res.tv_nsec = response.res.tv_nsec;
                }

                Ok(())
            },
            // Unexpected response message.
            header => {
                ::nvx::error!(
                    "clock_getres(): invalid response (clock_id={:?}, res={:?}, header={:?})",
                    clock_id,
                    res,
                    header
                );
                Err(Error::new(ErrorCode::TryAgain, "clock_getres() failed"))
            },
        }
    }
}
