// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::{
    event::EventManager,
    pm::{
        self,
        ProcessManager,
        SleepError,
    },
};
use ::sys::{
    ipc::Message,
    pm::{
        ProcessIdentifier,
        ThreadIdentifier,
    },
};

//==================================================================================================
// Standalone Functions
//==================================================================================================

///
/// # Description
///
/// Receives an inter-process message by waiting on the event manager and copying the result to
/// user space.
///
/// # Parameters
///
/// - `tid`: Thread identifier of the calling thread.
/// - `pid`: Process identifier of the calling process.
/// - `msg`: User-space address where the received message will be stored.
///
/// # Returns
///
/// Upon successful completion, empty is returned. On failure, a sleep error is returned instead.
///
/// # Safety
///
/// The calling thread must not be the kernel thread and must not hold a reference to the process
/// manager. The `msg` pointer must be a valid user-space address within the calling process.
///
pub unsafe fn recv(
    tid: ThreadIdentifier,
    pid: ProcessIdentifier,
    msg: usize,
) -> Result<(), SleepError> {
    trace!("pid={:?}", pid);

    match EventManager::wait(tid, pid) {
        Ok(message) => {
            pm::copy_to_user(ProcessManager::get_mut(), pid, msg as *mut Message, &message)
                .map_err(SleepError::Generic)
        },
        Err(sleep_error) => Err(sleep_error),
    }
}
