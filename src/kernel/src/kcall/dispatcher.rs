// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Modules
//==================================================================================================

use crate::{
    event,
    ipc,
    kcall::ScoreBoard,
    pm::{
        self,
        InterruptReason,
        ProcessManager,
        SleepError,
    },
};
use ::core::hint::cold_path;
use ::sys::{
    error::{
        Error,
        ErrorCode,
    },
    number::KcallNumber,
    pm::{
        ProcessIdentifier,
        ThreadIdentifier,
    },
};

//==================================================================================================
//  Standalone Functions
//==================================================================================================

///
/// # Description
///
/// High-level kernel call dispatcher.
///
/// # Parameters
///
/// - `arg0`: First kernel call argument.
/// - `arg1`: Second kernel call argument.
/// - `arg2`: Third kernel call argument.
/// - `arg3`: Fourth kernel call argument.
/// - `arg4`: Fifth kernel call argument.
/// - `number`: Number of the kernel call.
///
#[no_mangle]
pub extern "C" fn do_kcall(number: u32, arg0: u32, arg1: u32, arg2: u32, arg3: u32) -> i32 {
    let pid: ProcessIdentifier = match unsafe { ProcessManager::get() }.get_pid() {
        Ok(pid) => pid,
        Err(e) => return e.code.into_errno(),
    };
    let tid: ThreadIdentifier = match unsafe { ProcessManager::get() }.get_tid() {
        Ok(tid) => tid,
        Err(e) => return e.code.into_errno(),
    };

    match KcallNumber::from(number) {
        // Handle `getpid()` locally.
        KcallNumber::GetPid => pid.into(),
        // Handle `gettid()` locally.
        KcallNumber::GetTid => match tid.try_into() {
            Ok(tid) => tid,
            Err(error) => {
                cold_path();
                warn!("do_kcall(): failed to convert tid to i32 (error={:?})", error);
                error.code.into_errno()
            },
        },
        KcallNumber::Exit => {
            // SAFETY: the calling process is not the kernel.
            let e: Error = unsafe { ProcessManager::exit(arg0 as i32).unwrap_err() };
            e.code.into_errno()
        },
        // SAFETY: The calling thread is not the kernel and no resources are held. Furthermore,
        // the process manager and the virtual memory manager are initialized and access to them
        // is synchronized.
        KcallNumber::JoinThread => match unsafe { pm::join_thread(pid, arg0, arg1) } {
            Ok(status) => status as i32,
            Err(sleep_error) => handle_sleep_error(sleep_error).unwrap(),
        },
        KcallNumber::ExitThread => {
            // SAFETY: the calling process is not the kernel and it does not hold a mutable
            // reference to the inner state of the process manager.
            let e: Error = unsafe { ProcessManager::exit_thread(arg0 as usize).unwrap_err() };
            e.code.into_errno()
        },
        // SAFETY: The calling thread is not the kernel and no resources are held.
        KcallNumber::Recv => match unsafe { ipc::recv(pid, arg0 as usize) } {
            Ok(()) => 0,
            Err(sleep_error) => handle_sleep_error(sleep_error).unwrap(),
        },
        // SAFETY: The calling thread does not hold a reference to the process manager.
        KcallNumber::Resume => unsafe { event::resume(arg0 as usize) },
        // SAFETY: The calling thread is not the kernel, no resources are held, and the calling process does not hold a reference to the process manager.
        KcallNumber::MutexLock => match unsafe { pm::lock_mutex(arg0 as usize) } {
            Ok(()) => 0,
            Err(sleep_error) => handle_sleep_error(sleep_error).unwrap(),
        },
        // Dispatch kernel call for remote execution.
        _ => match ScoreBoard::get_mut() {
            // SAFETY: The calling thread is not the kernel and no resources are held.
            Ok(scoreboard) => {
                match unsafe { scoreboard.dispatch(number, pid, tid, arg0, arg1, arg2, arg3) } {
                    Ok(result) => result,
                    Err(sleep_error) => handle_sleep_error(sleep_error).unwrap(),
                }
            },
            Err(e) => e.code.into_errno(),
        },
    }
}

fn handle_sleep_error(sleep_error: SleepError) -> Result<i32, !> {
    match sleep_error {
        SleepError::Generic(generic_error) => Ok(generic_error.code.into_errno()),
        SleepError::Interrupted(reason) => match reason {
            InterruptReason::Killed => {
                // SAFETY: the calling process is not the kernel.
                let error: Error = unsafe {
                    ProcessManager::exit(ErrorCode::Interrupted.into_errno()).unwrap_err()
                };
                panic!("failled to exit() (error={:?})", error);
            },
        },
    }
}
