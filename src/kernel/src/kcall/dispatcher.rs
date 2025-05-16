// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::{
    event,
    ipc,
    kcall::{
        KcallResult,
        ScoreBoard,
    },
    pm::{
        self,
        InterruptReason,
        ProcessManager,
        SleepError,
    },
};
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
    ExitStatus,
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
#[unsafe(no_mangle)]
pub extern "C" fn do_kcall(number: u32, arg0: u32, arg1: u32, arg2: u32, arg3: u32) -> i64 {
    let pid: ProcessIdentifier = match unsafe { ProcessManager::get() }.get_pid() {
        Ok(pid) => pid,
        Err(e) => return KcallResult::Error(e.code.into()).into(),
    };
    let tid: ThreadIdentifier = match unsafe { ProcessManager::get() }.get_tid() {
        Ok(tid) => tid,
        Err(e) => return KcallResult::Error(e.code.into()).into(),
    };

    match KcallNumber::from(number) {
        // Handle `getpid()` locally.
        KcallNumber::GetPid => KcallResult::Success(Into::<usize>::into(pid).into()),
        // Handle `gettid()` locally.
        KcallNumber::GetTid => KcallResult::Success(Into::<usize>::into(tid).into()),
        KcallNumber::Exit => {
            // SAFETY: the calling process is not the kernel.
            let e: Error = unsafe { ProcessManager::exit(ExitStatus::from(arg0)).unwrap_err() };
            KcallResult::Error(e.code.into())
        },
        // SAFETY: The calling thread is not the kernel and no resources are held. Furthermore,
        // the process manager and the virtual memory manager are initialized and access to them
        // is synchronized.
        KcallNumber::JoinThread => match unsafe { pm::join_thread(pid, arg0, arg1) } {
            Ok(status) => KcallResult::Success(Into::<u32>::into(status).into()),
            Err(sleep_error) => handle_sleep_error(sleep_error),
        },
        KcallNumber::ExitThread => {
            // SAFETY: the calling process is not the kernel and it does not hold a mutable
            // reference to the inner state of the process manager.
            let e: Error = unsafe { ProcessManager::exit_thread(arg0.into()).unwrap_err() };
            KcallResult::Error(e.code.into())
        },
        // SAFETY: The calling thread is not the kernel and no resources are held.
        KcallNumber::Recv => match unsafe { ipc::recv(pid, arg0 as usize) } {
            Ok(()) => KcallResult::ok(),
            Err(sleep_error) => handle_sleep_error(sleep_error),
        },
        // SAFETY: The calling thread does not hold a reference to the process manager.
        KcallNumber::Resume => unsafe { event::resume(arg0 as usize) },
        // SAFETY: The calling thread is not the kernel, no resources are held, and the calling process does not hold a reference to the process manager.
        KcallNumber::MutexLock => {
            match unsafe { pm::lock_mutex(arg0 as usize, arg1 as usize, arg2 as usize) } {
                Ok(()) => KcallResult::ok(),
                Err(sleep_error) => handle_sleep_error(sleep_error),
            }
        },
        // SAFETY: The calling thread does not hold a reference to the process manager.
        KcallNumber::MutexUnlock => match unsafe { pm::unlock_mutex(pid, tid, arg0 as usize) } {
            Ok(()) => KcallResult::ok(),
            Err(e) => KcallResult::Error(e.code.into()),
        },
        // SAFETY: The calling thread is not the kernel, no resources are held, and the calling process does not hold a reference to the process manager.
        KcallNumber::CondWait => {
            match unsafe {
                pm::wait_cond(pid, tid, arg0 as usize, arg1 as usize, arg2 as usize, arg3 as usize)
            } {
                Ok(()) => KcallResult::ok(),
                Err(sleep_error) => handle_sleep_error(sleep_error),
            }
        },
        // SAFETY: The calling thread is not the kernel, no resources are held, and the calling process does not hold a reference to the process manager.
        KcallNumber::CondSignal => match unsafe { pm::signal_cond(arg0 as usize, arg1 != 0) } {
            Ok(awakened) => KcallResult::Success(awakened.into()),
            Err(e) => KcallResult::Error(e.code.into()),
        },
        // SAFETY: The calling thread does not hold any resources.
        KcallNumber::SchedulerYield => match unsafe { ProcessManager::switch() } {
            Ok(()) => KcallResult::ok(),
            Err(e) => KcallResult::Error(e.code.into()),
        },

        // Dispatch kernel call for remote execution.
        _ => match ScoreBoard::get_mut() {
            // SAFETY: The calling thread is not the kernel and no resources are held.
            Ok(scoreboard) => {
                match unsafe { scoreboard.dispatch(number, pid, tid, arg0, arg1, arg2, arg3) } {
                    Ok(result) => result,
                    Err(sleep_error) => handle_sleep_error(sleep_error),
                }
            },
            Err(e) => KcallResult::Error(e.code.into()),
        },
    }
    .into()
}

fn handle_sleep_error(sleep_error: SleepError) -> KcallResult {
    match sleep_error {
        SleepError::Generic(generic_error) => {
            error!("failed to sleep: {:?}", generic_error);
            KcallResult::Error(generic_error.code.into())
        },
        SleepError::Interrupted(reason) => match reason {
            InterruptReason::Killed => {
                // SAFETY: the calling process is not the kernel.
                let error: Error =
                    unsafe { ProcessManager::exit(ErrorCode::Interrupted.into()).unwrap_err() };
                panic!("failled to exit() (error={:?})", error);
            },
            InterruptReason::TimedOut => {
                error!("failed to sleep: operation timed out");
                KcallResult::Error(ErrorCode::OperationTimedOut.into())
            },
        },
    }
}
