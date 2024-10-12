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
use ::core::ffi;
use ::nvx::{
    ipc::Message,
    pm::ProcessIdentifier,
    sys::error::ErrorCode,
};

//==================================================================================================
// Standalone Functions
//==================================================================================================

/// Get clock time.
pub fn clock_gettime(clock_id: clockid_t, tp: *mut timespec) -> ffi::c_int {
    assert!(crate::time::__is_clock_id_supported(clock_id), "unsupported clock_id {:?}", clock_id);

    let pid: ProcessIdentifier = match ::nvx::pm::getpid() {
        Ok(pid) => pid,
        Err(e) => return e.code.into_errno(),
    };

    // Build request and send it.
    let request: Message = GetClockTimeRequest::build(pid, clock_id);
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
                LinuxDaemonMessageHeader::GetClockTimeResponse => {
                    let response: GetClockTimeResponse =
                        GetClockTimeResponse::from_bytes(message.payload);

                    // Copy time if requested.
                    if !tp.is_null() {
                        unsafe {
                            (*tp).tv_sec = response.tp.tv_sec;
                            (*tp).tv_nsec = response.tp.tv_nsec;
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
