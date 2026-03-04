// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::{
    sys::times::message::{
        TimesRequest,
        TimesResponse,
    },
    LinuxDaemonMessage,
    LinuxDaemonMessageHeader,
};
use ::sys::{
    error::{
        Error,
        ErrorCode,
    },
    ipc::Message,
    pm::ThreadIdentifier,
};
use sysapi::{
    sys_times::tms,
    sys_types::clock_t,
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
/// Upon successful completion, `times()` returns the elapsed time since an arbitrary point in the
/// past. Otherwise, an error code is returned.
///
#[allow(unreachable_code)]
pub fn times(buffer: &mut Option<&mut tms>) -> Result<clock_t, Error> {
    ::syslog::trace!("times(): {:?}", buffer);

    // In standalone mode, return zeroed times with elapsed=0 (no linuxd).
    #[cfg(feature = "standalone")]
    {
        if let Some(buf) = buffer {
            buf.tms_utime = 0;
            buf.tms_stime = 0;
            buf.tms_cutime = 0;
            buf.tms_cstime = 0;
        }
        return Ok(0);
    }

    let tid: ThreadIdentifier = ::sys::kcall::pm::gettid()?;

    // Build request and send it.
    let request: Message = TimesRequest::build(tid)?;
    ::sys::kcall::ipc::send(&request)?;

    // Receive response.
    let response: Message = ::sys::kcall::ipc::recv()?;

    // Check whether system call succeeded or not.
    if response.status != 0 {
        ::syslog::error!("times(): failed (buffer={:?}, status={:?})", buffer, { response.status });
        // System call failed, parse error code and return it.
        match ErrorCode::try_from(response.status) {
            Ok(error_code) => Err(Error::new(error_code, "times() failed")),
            Err(error) => {
                ::syslog::error!("times(): failed to parse error code (error={:?})", error);
                Err(Error::new(ErrorCode::TryAgain, "times() failed"))
            },
        }
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
            header => {
                ::syslog::error!("times(): failed (buffer={:?}, header={:?})", buffer, header);
                Err(Error::new(ErrorCode::InvalidMessage, "times() failed"))
            },
        }
    }
}
