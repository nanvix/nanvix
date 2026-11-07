// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use ::sys::{
    error::{
        Error,
        ErrorCode,
    },
    pm::ThreadIdentifier,
};
use ::sysapi::{
    ffi::c_int,
    pthread::PTHREAD_NULL,
    sched::{
        sched_param,
        sched_policy::SCHED_OTHER,
    },
    sys_types::pthread_t,
};

//==================================================================================================
// Constants
//==================================================================================================

/// Scheduling policy of every thread.
///
/// The kernel schedules all threads under a single preemptive time-sharing policy, which is the
/// policy that `SCHED_OTHER` denotes.
const THREAD_SCHED_POLICY: c_int = SCHED_OTHER;

/// Scheduling priority of every thread.
///
/// The kernel does not support thread priorities, so all threads share the same priority.
const THREAD_SCHED_PRIORITY: c_int = 0;

//==================================================================================================
// Standalone Functions
//==================================================================================================

///
/// # Description
///
/// Gets the scheduling policy and scheduling parameters of a thread.
///
/// # Parameters
///
/// - `thread`: Thread whose scheduling policy and parameters will be retrieved.
///
/// # Return Value
///
/// On success, this function returns the scheduling policy and the scheduling parameters of the
/// thread. Otherwise, it returns an error indicating the reason for the failure.
///
/// # Errors
///
/// The following errors can be returned by this function:
///
/// - [`ErrorCode::NoSuchProcess`] if `thread` cannot identify a thread.
///
/// # Notes
///
/// - Threads are created without applying the scheduling attributes of their thread attributes
///   objects, and their scheduling policy and parameters cannot be changed afterwards. Therefore,
///   every thread runs under `SCHED_OTHER` with a scheduling priority of zero.
///
pub fn pthread_getschedparam(thread: pthread_t) -> Result<(c_int, sched_param), Error> {
    ::syslog::trace!("pthread_getschedparam(): thread={thread:?}");

    // Check if `thread` cannot identify a thread. The null thread identifier is never assigned to a
    // user thread, and neither are identifiers that do not fit in a kernel thread identifier.
    if thread == PTHREAD_NULL || ThreadIdentifier::try_from(thread).is_err() {
        let reason: &'static str = "thread identifier does not identify a thread";
        ::syslog::warn!("pthread_getschedparam(): {reason} (thread={thread:?})");
        return Err(Error::new(ErrorCode::NoSuchProcess, reason));
    }

    let param: sched_param = sched_param {
        sched_priority: THREAD_SCHED_PRIORITY,
    };

    Ok((THREAD_SCHED_POLICY, param))
}
