// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Modules
//==================================================================================================

use ::sys::{
    error::{
        Error,
        ErrorCode,
    },
    time::SystemTime,
};
use ::sysapi::{
    ffi::c_long,
    sys_types::{
        clockid_t,
        time_t,
    },
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
    ::syslog::trace!("clock_gettime(): clock_id={:?}, tp={:?}", clock_id, tp);

    match clock_id {
        CLOCK_MONOTONIC | CLOCK_REALTIME => {
            // Get system time and store it in the provided timespec structure.
            let mut now: SystemTime = SystemTime::default();
            ::sys::kcall::pm::gettime(&mut now)?;
            ::syslog::debug!("clock_gettime(): now={:?}", now);
            if let Some(tp) = tp {
                tp.tv_sec = now.seconds() as time_t;
                tp.tv_nsec = now.nanoseconds() as c_long;
            }

            Ok(())
        },
        CLOCK_PROCESS_CPUTIME_ID | CLOCK_THREAD_CPUTIME_ID => {
            let reason: &str = "unsupported clock id";
            ::syslog::error!("clock_gettime(): {} (clock_id={:?}, tp={:x?})", reason, clock_id, tp);
            Err(Error::new(ErrorCode::OperationNotSupported, reason))
        },

        clock_id => {
            let reason: &str = "invalid clock id";
            ::syslog::error!("clock_gettime(): {} (clock_id={:?}, tp={:x?})", reason, clock_id, tp);
            Err(Error::new(ErrorCode::InvalidArgument, reason))
        },
    }
}
