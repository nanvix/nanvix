// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::{
    debug,
    event,
    io,
    ipc,
    kcall::{
        handler::poll_ikc_messages,
        KcallResult,
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
    let pid: ProcessIdentifier = unsafe { ProcessManager::get() }.get_pid();
    let tid: ThreadIdentifier = unsafe { ProcessManager::get() }.get_tid();

    let result: KcallResult = match KcallNumber::from(number) {
        // Handle `getpid()` locally.
        KcallNumber::GetPid => KcallResult::Success(<i64>::from(pid).into()),
        // Handle `getppid()` locally.
        KcallNumber::GetPpid => {
            let ppid: ProcessIdentifier = unsafe { ProcessManager::get() }.get_ppid();
            KcallResult::Success(<i64>::from(ppid).into())
        },
        // Handle `gettid()` locally.
        KcallNumber::GetTid => KcallResult::Success(<i64>::from(tid).into()),
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
        // Handle `capctl()` locally.
        KcallNumber::CapCtl => pm::capctl(pid, arg0, arg1),
        // Handle `terminate()` locally.
        KcallNumber::Terminate => pm::terminate(pid, arg0),
        // Handle `evctrl()` locally.
        KcallNumber::EventCtrl => event::evctrl(pid, arg0, arg1),
        // Handle `set_thread_data_area()` locally.
        KcallNumber::SetThreadDataArea => pm::set_thread_data_area(pid, tid, arg0),
        // Handle `get_thread_data_area()` locally.
        KcallNumber::GetThreadDataArea => pm::get_thread_data_area(pid, tid),
        // Handle `mmio_free()` locally.
        KcallNumber::FreeMmio => io::mmio_free(pid, arg0, arg1),
        // Handle `mmio_alloc()` locally.
        KcallNumber::AllocMmio => io::mmio_alloc(pid, arg0, arg1),
        // Handle `mmio_info()` locally.
        KcallNumber::MmioInfo => io::mmio_info(pid, arg0, arg1, arg2),
        // Handle `pmio_free()` locally.
        KcallNumber::FreePmio => io::pmio_free(pid, arg0),
        // Handle `pmio_alloc()` locally.
        KcallNumber::AllocPmio => io::pmio_alloc(pid, arg0, arg1),
        // Handle `pmio_read()` locally.
        KcallNumber::ReadPmio => io::pmio_read(pid, arg0, arg1),
        // Handle `pmio_write()` locally.
        KcallNumber::WritePmio => io::pmio_write(pid, arg0, arg1, arg2),
        // Handle `gettime()` locally.
        KcallNumber::GetTime => pm::gettime(pid, arg0),
        // Handle `debug()` locally.
        KcallNumber::Debug => debug::debug(pid, arg0, arg1),
        // Handle `mmap()` locally.
        KcallNumber::MemoryMap => pm::mmap(pid, arg0, arg1, arg2, arg3),
        // Handle `munmap()` locally.
        KcallNumber::MemoryUnmap => pm::munmap(pid, arg0, arg1),
        // Handle `mctrl()` locally.
        KcallNumber::MemoryCtrl => pm::mctrl(pid, arg0, arg1, arg2),
        // Handle `mcopy()` locally.
        KcallNumber::MemoryCopy => pm::mcopy(pid, arg0, arg1, arg2, arg3),
        // Handle `create_thread()` locally.
        KcallNumber::CreateThread => pm::create_thread(pid, arg0),
        // Handle `detach_thread()` locally.
        KcallNumber::DetachThread => pm::detach_thread(pid, arg0),
        // Handle `duplicate()` locally.
        KcallNumber::Duplicate => pm::duplicate(pid, arg0),
        // Handle `execv()` locally. On success the calling process's image is replaced and this
        // never returns; only a failure surfaces here as an error result.
        KcallNumber::Execv => pm::execv(pid, arg0),
        // Handle `send()` locally.
        KcallNumber::Send => ipc::send(pid, tid, arg0),
        // SAFETY: The calling thread is not the kernel and no resources are held.
        KcallNumber::Recv => match unsafe { ipc::recv(tid, pid, arg0 as usize) } {
            Ok(()) => KcallResult::ok(),
            Err(sleep_error) => handle_sleep_error(sleep_error),
        },
        // SAFETY: The calling thread is not the kernel and no resources are held.
        KcallNumber::Push => match ipc::push(pid, tid, arg0, arg1, arg2 as usize, arg3) {
            Ok(()) => KcallResult::ok(),
            Err(sleep_error) => handle_sleep_error(sleep_error),
        },
        // SAFETY: The calling thread is not the kernel and no resources are held.
        KcallNumber::Pull => match ipc::pull(pid, tid, arg0, arg1, arg2 as usize, arg3) {
            Ok(bytes_transferred) => match u32::try_from(bytes_transferred) {
                Ok(bytes_u32) => KcallResult::Success(bytes_u32.into()),
                Err(_) => KcallResult::Error(ErrorCode::InvalidArgument.into()),
            },
            Err(sleep_error) => handle_sleep_error(sleep_error),
        },
        // SAFETY: The calling thread does not hold a reference to the process manager.
        KcallNumber::Resume => unsafe { event::resume(arg0 as usize) },
        // SAFETY: The calling thread is not the kernel, no resources are held, and the calling process does not hold a reference to the process manager.
        KcallNumber::MutexLock => {
            match unsafe { pm::lock_mutex(pid, tid, arg0 as usize, arg1 as usize, arg2 as usize) } {
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
        KcallNumber::CondSignal => {
            match unsafe { pm::signal_cond(pid, tid, arg0 as usize, arg1 != 0) } {
                Ok(awakened) => KcallResult::Success(awakened.into()),
                Err(e) => KcallResult::Error(e.code.into()),
            }
        },
        // SAFETY: The calling thread does not hold any resources.
        KcallNumber::SchedulerYield => match unsafe { ProcessManager::giveup() } {
            Ok(()) => KcallResult::ok(),
            Err(e) => KcallResult::Error(e.code.into()),
        },
        // SAFETY: The calling thread does not hold any resources.
        KcallNumber::Sleep => match unsafe { pm::sleep(arg0 as usize, arg1 as usize) } {
            Ok(()) => KcallResult::ok(),
            Err(sleep_error) => handle_sleep_error(sleep_error),
        },

        // Handle `snapshot()` locally (microvm platform only).
        // The snapshot capability must have been enabled via the `snapshot` kernel option
        // and is consumed on the first successful request; subsequent attempts are refused.
        #[cfg(feature = "microvm")]
        KcallNumber::Snapshot => {
            if crate::try_consume_snapshot() {
                crate::hal::platform::snapshot();
                KcallResult::ok()
            } else {
                error!("snapshot refused: not enabled or already consumed");
                KcallResult::Error(ErrorCode::OperationNotPermitted.into())
            }
        },
        // Snapshot is not supported on non-microvm platforms.
        #[cfg(not(feature = "microvm"))]
        KcallNumber::Snapshot => KcallResult::Error(ErrorCode::OperationNotSupported.into()),

        // Signal subsystem kernel calls. `sigaction()` manages per-process dispositions,
        // `sigprocmask()` manages the calling thread's blocked mask, and `kill()` posts a signal
        // to a target process; the remaining calls are still inert stubs implemented by later
        // phases of the signals effort.
        KcallNumber::Sigaction => pm::sigaction(pid, arg0, arg1, arg2),
        KcallNumber::Sigprocmask => pm::sigprocmask(pid, tid, arg0, arg1, arg2),
        KcallNumber::Kill => pm::kill(pid, arg0, arg1),
        KcallNumber::Sigreturn => pm::sigreturn(),
        KcallNumber::Sigpending => pm::sigpending(),
        KcallNumber::Sigsuspend => pm::sigsuspend(),

        // Unknown kernel call.
        _ => {
            error!("invalid kernel call (number={})", number);
            KcallResult::Error(ErrorCode::InvalidSysCall.into())
        },
    };

    // Poll for inter-kernel communication messages after handling the kernel call. Polling after
    // (rather than before) ensures that any outbound messages produced by the current kernel call
    // are already enqueued, so the poll can immediately dispatch replies and follow-up messages
    // without waiting for the next scheduling opportunity.
    poll_ikc_messages();

    result.into()
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
                panic!("failed to exit() (error={:?})", error);
            },
            InterruptReason::TimedOut => {
                error!("failed to sleep: operation timed out");
                KcallResult::Error(ErrorCode::OperationTimedOut.into())
            },
        },
    }
}
