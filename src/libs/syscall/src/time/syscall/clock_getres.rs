// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Modules
//==================================================================================================

use ::config::kernel::TIMER_FREQ;
use ::sys::{
    error::{
        Error,
        ErrorCode,
    },
    time::NANOSECONDS_PER_SECOND,
};
use ::sysapi::ffi::c_long;
use sysapi::{
    sys_types::clockid_t,
    time::{
        clock_ids::{
            CLOCK_MONOTONIC,
            CLOCK_PROCESS_CPUTIME_ID,
            CLOCK_REALTIME,
            CLOCK_THREAD_CPUTIME_ID,
        },
        timespec,
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
    ::syslog::error!("clock_getres(): clock_id={:?}, res={:?}", clock_id, res);

    match clock_id {
        // Check if the clock ID is valid.
        CLOCK_REALTIME | CLOCK_MONOTONIC => {
            if let Some(res) = res {
                res.tv_sec = 0;
                res.tv_nsec = NANOSECONDS_PER_SECOND as c_long / TIMER_FREQ as c_long;
            }
            Ok(())
        },
        CLOCK_PROCESS_CPUTIME_ID | CLOCK_THREAD_CPUTIME_ID => {
            let reason: &str = "unsupported clock id";
            ::syslog::error!("clock_getres(): {} (clock_id={:?}, res={:?})", reason, clock_id, res);
            Err(Error::new(ErrorCode::OperationNotSupported, "clock_getres() failed"))
        },
        clock_id => {
            let reason: &str = "invalid clock id";
            ::syslog::error!("clock_getres(): {} (clock_id={:?}, res={:?})", reason, clock_id, res);
            Err(Error::new(ErrorCode::InvalidArgument, "clock_getres() failed"))
        },
    }
}
