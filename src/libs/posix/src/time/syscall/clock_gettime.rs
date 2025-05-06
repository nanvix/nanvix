// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Modules
//==================================================================================================

use crate::{
    ffi::c_long,
    time::{
        clockid_t,
        time_t,
        timespec,
        CLOCK_MONOTONIC,
        CLOCK_PROCESS_CPUTIME_ID,
        CLOCK_REALTIME,
        CLOCK_THREAD_CPUTIME_ID,
    },
};
use ::nvx::sys::error::{
    Error,
    ErrorCode,
};
use ::time::SystemTime;

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
pub fn clock_gettime(clock_id: clockid_t, tp: &mut Option<&mut timespec>) -> Result<(), Error> {
    ::nvx::trace!("clock_gettime(): clock_id={:?}, tp={:?}", clock_id, tp);

    match clock_id {
        CLOCK_MONOTONIC | CLOCK_REALTIME => {
            // Get system time and store it in the provided timespec structure.
            let mut now: SystemTime = SystemTime::default();
            ::nvx::pm::gettime(&mut now)?;
            nvx::debug!("clock_gettime(): now={:?}", now);
            if let Some(tp) = tp {
                tp.tv_sec = now.seconds() as time_t;
                tp.tv_nsec = now.nanoseconds() as c_long;
            }

            Ok(())
        },
        CLOCK_PROCESS_CPUTIME_ID | CLOCK_THREAD_CPUTIME_ID => {
            let reason: &str = "unsupported clock id";
            ::nvx::error!("clock_gettime(): {} (clock_id={:?}, tp={:x?})", reason, clock_id, tp);
            Err(Error::new(ErrorCode::OperationNotSupported, reason))
        },

        clock_id => {
            let reason: &str = "invalid clock id";
            ::nvx::error!("clock_gettime(): {} (clock_id={:?}, tp={:x?})", reason, clock_id, tp);
            Err(Error::new(ErrorCode::InvalidArgument, reason))
        },
    }
}
