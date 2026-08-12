// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::{
    event::{
        EventManager,
        PendingDelivery,
    },
    pm::{
        self,
        ProcessManager,
        SleepError,
    },
};
use ::sys::{
    error::Error,
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
/// Receives an inter-process message by waiting on the event manager and handing the selected
/// message to `copy_to_user`. The selection is committed only if that copy succeeds, so a failed
/// copy leaves the message pending for a later receive.
///
/// # Parameters
///
/// - `tid`: Thread identifier of the calling thread.
/// - `pid`: Process identifier of the calling process.
/// - `copy_to_user`: Operation that copies the selected message out of the kernel.
///
/// # Returns
///
/// Upon successful completion, empty is returned. On failure, a sleep error is returned instead.
///
/// # Safety
///
/// The calling thread must not be the kernel thread and must not hold a reference to the process
/// manager. The `copy_to_user` operation must not commit the selection nor mutate delivery state.
///
pub(crate) unsafe fn recv_with<F>(
    tid: ThreadIdentifier,
    pid: ProcessIdentifier,
    copy_to_user: F,
) -> Result<(), SleepError>
where
    F: FnOnce(&Message) -> Result<(), Error>,
{
    let delivery: PendingDelivery = EventManager::wait(tid, pid)?;
    copy_to_user(delivery.message()).map_err(SleepError::Generic)?;
    EventManager::commit(delivery);
    Ok(())
}

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

    recv_with(tid, pid, |message| {
        pm::copy_to_user(ProcessManager::get_mut(), pid, msg as *mut Message, message)
    })
}
