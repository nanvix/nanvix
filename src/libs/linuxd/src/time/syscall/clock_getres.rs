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
use ::core::ffi;
use ::nvx::{
    ipc::Message,
    pm::ProcessIdentifier,
    sys::error::ErrorCode,
};

//==================================================================================================
// Standalone Functions
//==================================================================================================

/// Get clock resolution.
pub fn clock_getres(clock_id: clockid_t, res: *mut timespec) -> ffi::c_int {
    assert!(crate::time::__is_clock_id_supported(clock_id), "unsupported clock_id {:?}", clock_id);

    let pid: ProcessIdentifier = match ::nvx::pm::getpid() {
        Ok(pid) => pid,
        Err(e) => return e.code.into_errno(),
    };

    // Build request and send it.
    let request: Message = ClockResolutionRequest::build(pid, clock_id);
    if let Err(e) = ::nvx::ipc::send(&request) {
        return e.code.into_errno();
    }

    // Receive response.
    let response: Message = match ::nvx::ipc::recv() {
        Ok(response) => response,
        Err(e) => return e.code.into_errno(),
    };

    // Check whether system call succeeded or not.
    if response.status != 0 {
        // System call failed, parse error code and return it.
        match ErrorCode::try_from(response.status) {
            Ok(e) => e.into_errno(),
            Err(_) => ErrorCode::InvalidMessage.into_errno(),
        }
    } else {
        // System call succeeded, parse response.
        match LinuxDaemonMessage::try_from_bytes(response.payload) {
            // Response was successfully parsed.
            Ok(message) => match message.header {
                LinuxDaemonMessageHeader::GetClockResolutionResponse => {
                    let response: ClockGetResolutionResponse =
                        ClockGetResolutionResponse::from_bytes(message.payload);

                    // Copy resolution if requested.
                    if !res.is_null() {
                        unsafe {
                            (*res).tv_sec = response.res.tv_sec;
                            (*res).tv_nsec = response.res.tv_nsec;
                        }
                    }

                    0
                },
                // Unexpected response message.
                _ => ErrorCode::InvalidMessage.into_errno(),
            },
            // Failed to parse response.
            Err(e) => e.code.into_errno(),
        }
    }
}
