// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::{
    error::{
        Error,
        ErrorCode,
    },
    kcall0,
    kcall1,
    kcall2,
    kcall3,
    kcall4,
    mm::VirtualAddress,
    number::KcallNumber,
    pm::{
        Capability,
        ConditionAddress,
        MutexAddress,
        ProcessIdentifier,
        ThreadCreateArgs,
        ThreadIdentifier,
    },
    time::SystemTime,
};
use ::core::{
    hint::unlikely,
    time::Duration,
};

//==================================================================================================
// Get Process Identifier
//==================================================================================================

///
/// # Description
///
/// Gets the process identifier of the calling process.
///
/// # Returns
///
/// Upon successful completion, the process identifier of the calling process is returned. Upon
/// failure, an error is returned instead.
///
pub fn getpid() -> Result<ProcessIdentifier, Error> {
    let result: i64 = kcall0!(KcallNumber::GetPid.into());

    // NOTE: errors are unlikely because getpid() always succeeds for a valid calling process.
    if unlikely(result < 0) {
        Err(Error::new(ErrorCode::try_from(result)?, "failed to getpid()"))
    } else {
        ProcessIdentifier::try_from(result)
    }
}

//==================================================================================================
// Get Thread Identifier
//==================================================================================================

///
/// # Description
///
/// Gets the thread identifier of the calling thread.
///
/// # Returns
///
/// Upon successful completion, the thread identifier of the calling thread is returned. Upon
/// failure, an error is returned instead.
///
pub fn gettid() -> Result<ThreadIdentifier, Error> {
    let result: i64 = kcall0!(KcallNumber::GetTid.into());

    // NOTE: errors are unlikely because gettid() always succeeds for a valid calling thread.
    if unlikely(result < 0) {
        Err(Error::new(ErrorCode::try_from(result)?, "failed to gettid()"))
    } else {
        ThreadIdentifier::try_from(result)
    }
}

//==================================================================================================
// Exit
//==================================================================================================

///
/// # Description
///
/// Exits the calling process.
///
/// # Parameters
///
/// - `status`: Exit status.
///
/// # Returns
///
/// Upon successful completion, this function does not return. Upon failure, an error is returned
/// instead.
///
pub fn exit(status: i32) -> Result<!, Error> {
    let result: i64 = kcall1!(KcallNumber::Exit.into(), status as u32);
    Err(Error::new(ErrorCode::try_from(result)?, "failed to terminate process"))
}

//==================================================================================================
// Capability Control
//==================================================================================================

///
/// # Description
///
/// Controls a capability for the calling process.
///
/// # Parameters
///
/// - `capability`: Capability to control.
/// - `value`: Whether to enable or disable the capability.
///
/// # Returns
///
/// Upon successful completion, empty is returned. Upon failure, an error is returned instead.
///
pub fn capctl(capability: Capability, value: bool) -> Result<(), Error> {
    let result: i64 = kcall2!(KcallNumber::CapCtl.into(), capability as u32, value as u32);

    // NOTE: errors are unlikely because capability control typically succeeds.
    if unlikely(result != 0) {
        Err(Error::new(ErrorCode::try_from(result)?, "failed to capctl()"))
    } else {
        Ok(())
    }
}

//==================================================================================================
// Terminate
//==================================================================================================

///
/// # Description
///
/// Terminates a target process.
///
/// # Parameters
///
/// - `pid`: Process identifier of the process to terminate.
///
/// # Returns
///
/// Upon successful completion, empty is returned. Upon failure, an error is returned instead.
///
pub fn terminate(pid: ProcessIdentifier) -> Result<(), Error> {
    let result: i64 = kcall1!(KcallNumber::Terminate.into(), u32::try_from(pid)?);

    // NOTE: errors are unlikely because terminate typically succeeds for a valid process.
    if unlikely(result != 0) {
        Err(Error::new(ErrorCode::try_from(result)?, "failed to terminate()"))
    } else {
        Ok(())
    }
}

//==================================================================================================
// Create Thread
//==================================================================================================

::core::arch::global_asm!(
    r#"
    .global _do_start_thread
    .extern _do_exit_thread
    .type _do_start_thread, @function

    _do_start_thread:
        #
        # Entry point for newly created threads.
        #
        # The kernel sets up a trap frame so that IRET "returns" to this function.
        # The kernel passes the thread function pointer in EDX and its argument
        # in ECX.
        #
        # This stub calls func(arg) and then _do_exit_thread(status) directly,
        # enforcing 16-byte stack alignment before each CALL instruction. This
        # avoids routing through a Rust intermediate function whose compiler-
        # generated prologue may not preserve 16-byte alignment (the Nanvix Rust
        # target disables SSE, so LLVM omits alignment-preserving prologues).
        # The callee func may be compiled by GCC with SSE enabled and may
        # therefore require 16-byte-aligned stack frames (e.g., movaps).
        #

        # Save func and arg into callee-saved registers.
        # This is the thread root frame so there is no caller state to preserve.
        mov esi, edx        # ESI = func
        mov edi, ecx        # EDI = arg

        # Set up frame pointer and force 16-byte alignment.
        and esp, -16
        mov ebp, esp

        #
        # Call func(arg).
        #
        # Stack alignment arithmetic (i386 SysV ABI):
        #   and esp,-16 -> ESP = 0 (mod 16)   (force-aligned)
        #   sub esp, 12 -> ESP = 4 (mod 16)   (alignment padding)
        #   push edi    -> ESP = 0 (mod 16)   (push arg)
        #   call esi    -> ESP = 12 (mod 16)  (return address pushed by CALL)
        #
        sub esp, 12
        push edi
        call esi

        #
        # Call _do_exit_thread(status).
        #
        # func() returned status in EAX.  Re-align the stack for the next call.
        #
        # Stack alignment arithmetic:
        #   and esp,-16 -> ESP = 0 (mod 16)   (force-aligned)
        #   sub esp, 12 -> ESP = 4 (mod 16)   (alignment padding)
        #   push eax    -> ESP = 0 (mod 16)   (push status)
        #   call        -> ESP = 12 (mod 16)  (return address pushed by CALL)
        #
        and esp, -16
        sub esp, 12
        push eax
        call _do_exit_thread

    # Safety net: _do_exit_thread() calls exit_thread() and never returns.
    # If it somehow does, spin forever rather than falling through.
    1: jmp 1b
    "#
);

///
/// # Description
///
/// Exit handler for newly created threads. Called by the `_do_start_thread` assembly stub when the
/// thread function returns. Invokes [`exit_thread()`] with the thread's return status.
///
/// # Parameters
///
/// - `status`: Exit status returned by the thread function.
///
#[unsafe(no_mangle)]
pub extern "C" fn _do_exit_thread(status: usize) -> ! {
    let _ = exit_thread(status);
    unreachable!("failed to exit thread");
}

///
/// # Description
///
/// Creates a new thread in the calling process.
///
/// # Parameters
///
/// - `args`: Mutable reference to thread creation arguments, including the entry point and
///   stack configuration. The `user_fn` field is overwritten with the internal thread entry stub.
///
/// # Returns
///
/// Upon successful completion, the thread identifier of the new thread is returned. Upon failure,
/// an error is returned instead.
///
pub fn create_thread(args: &mut ThreadCreateArgs) -> Result<ThreadIdentifier, Error> {
    unsafe extern "C" {
        fn _do_start_thread() -> !;
    }

    // Check if the user function is set.
    if args.user_fn != ThreadCreateArgs::NULL_USER_FN {
        // If the user function is already set, it means that this function has been called
        // recursively, which is not allowed.
        return Err(Error::new(ErrorCode::InvalidArgument, "user function is set"));
    }

    args.user_fn = VirtualAddress::from_raw_value(_do_start_thread as *const () as usize);

    let result: i64 =
        kcall1!(KcallNumber::CreateThread.into(), args as *const ThreadCreateArgs as usize as u32);

    // NOTE: errors may happen on resource exhaustion, but are still unlikely in general.
    if unlikely(result < 0) {
        Err(Error::new(ErrorCode::try_from(result)?, "failed to create_thread()"))
    } else {
        ThreadIdentifier::try_from(result)
    }
}

//==================================================================================================
// Exit Thread
//==================================================================================================

///
/// # Description
///
/// Exits the calling thread.
///
/// # Parameters
///
/// - `status`: Exit status of the thread.
///
/// # Returns
///
/// Upon successful completion, this function does not return. Upon failure, an error is returned
/// instead.
///
pub fn exit_thread(status: usize) -> Result<!, Error> {
    let result: i64 = kcall1!(KcallNumber::ExitThread.into(), status as u32);

    Err(Error::new(ErrorCode::try_from(result)?, "failed to terminate thread"))
}

//==================================================================================================
// Join Thread
//==================================================================================================

///
/// # Description
///
/// Waits for a target thread to terminate and retrieves its exit status.
///
/// # Parameters
///
/// - `tid`: Thread identifier of the thread to join.
/// - `retval`: Mutable reference to store the exit status of the joined thread.
///
/// # Returns
///
/// Upon successful completion, empty is returned. Upon failure, an error is returned instead.
///
pub fn join_thread(tid: ThreadIdentifier, retval: &mut usize) -> Result<(), Error> {
    let result: i64 =
        kcall2!(KcallNumber::JoinThread.into(), i32::from(tid) as u32, retval as *mut usize as u32);

    // NOTE: errors are unlikely because join typically succeeds for a valid thread.
    if unlikely(result != 0) {
        Err(Error::new(ErrorCode::try_from(result)?, "failed to join thread"))
    } else {
        Ok(())
    }
}

//==================================================================================================
// Lock Mutex
//==================================================================================================

///
/// # Description
///
/// Locks a mutex. If the mutex is already held, the calling thread blocks until the mutex becomes
/// available or the optional timeout expires.
///
/// # Parameters
///
/// - `mutex_addr`: Address of the mutex to lock.
/// - `timeout`: Optional timeout. If `None`, the call blocks indefinitely.
///
/// # Returns
///
/// Upon successful completion, empty is returned. Upon failure, an error is returned instead.
///
pub fn lock_mutex(mutex_addr: MutexAddress, timeout: Option<SystemTime>) -> Result<(), Error> {
    // Attempt to convert the timeout.
    let (seconds, nanoseconds): (u32, u32) = match timeout {
        Some(timeout) => {
            let seconds: u32 = timeout.seconds().try_into().map_err(|_| {
                Error::new(ErrorCode::InvalidArgument, "timeout value is too large")
            })?;
            let nanoseconds: u32 = timeout.nanoseconds();
            (seconds, nanoseconds)
        },
        None => (u32::MAX, u32::MAX),
    };

    let result: i64 = kcall3!(
        KcallNumber::MutexLock.into(),
        usize::from(mutex_addr) as u32,
        seconds,
        nanoseconds
    );

    // NOTE: errors are unlikely because lock typically succeeds for a valid mutex.
    if unlikely(result != 0) {
        Err(Error::new(ErrorCode::try_from(result)?, "failed to lock mutex"))
    } else {
        Ok(())
    }
}

//==================================================================================================
// Unlock Mutex
//==================================================================================================

///
/// # Description
///
/// Unlocks a mutex previously locked by the calling thread.
///
/// # Parameters
///
/// - `mutex_addr`: Address of the mutex to unlock.
///
/// # Returns
///
/// Upon successful completion, empty is returned. Upon failure, an error is returned instead.
///
pub fn unlock_mutex(mutex_addr: MutexAddress) -> Result<(), Error> {
    let result: i64 = kcall1!(KcallNumber::MutexUnlock.into(), usize::from(mutex_addr) as u32);

    // NOTE: errors are unlikely because unlock typically succeeds for a valid, held mutex.
    if unlikely(result != 0) {
        Err(Error::new(ErrorCode::try_from(result)?, "failed to unlock mutex"))
    } else {
        Ok(())
    }
}

//==================================================================================================
// Signal Condition Variable
//==================================================================================================

///
/// # Description
///
/// Signals a condition variable, waking one or all waiting threads.
///
/// # Parameters
///
/// - `cond_addr`: Address of the condition variable to signal.
/// - `broadcast`: If `true`, wakes all waiting threads. If `false`, wakes at most one.
///
/// # Returns
///
/// Upon successful completion, the number of threads awakened is returned. Upon failure, an error
/// is returned instead.
///
pub fn signal_cond(cond_addr: ConditionAddress, broadcast: bool) -> Result<usize, Error> {
    let result: i64 = kcall4!(
        KcallNumber::CondSignal.into(),
        usize::from(cond_addr) as u32,
        broadcast as u32,
        u32::MAX,
        u32::MAX
    );

    // NOTE: errors are unlikely because signal typically succeeds for a valid condition variable.
    if unlikely(result < 0) {
        Err(Error::new(ErrorCode::try_from(result)?, "failed to signal condition variable"))
    } else {
        Ok(result as usize)
    }
}

//==================================================================================================
// Wait Condition Variable
//==================================================================================================

///
/// # Description
///
/// Waits on a condition variable. The calling thread atomically releases the associated mutex and
/// blocks until the condition variable is signaled or the optional timeout expires.
///
/// # Parameters
///
/// - `cond_addr`: Address of the condition variable to wait on.
/// - `mutex_addr`: Address of the mutex associated with the condition variable.
/// - `timeout`: Optional timeout. If `None`, the call blocks indefinitely.
///
/// # Returns
///
/// Upon successful completion, empty is returned. Upon failure, an error is returned instead.
///
pub fn wait_cond(
    cond_addr: ConditionAddress,
    mutex_addr: MutexAddress,
    timeout: Option<SystemTime>,
) -> Result<(), Error> {
    // Attempt to convert the timeout.
    let (seconds, nanoseconds): (u32, u32) = match timeout {
        Some(timeout) => {
            let seconds: u32 = timeout.seconds().try_into().map_err(|_| {
                Error::new(ErrorCode::InvalidArgument, "timeout value is too large")
            })?;
            let nanoseconds: u32 = timeout.nanoseconds();
            (seconds, nanoseconds)
        },
        None => (u32::MAX, u32::MAX),
    };

    let result: i64 = kcall4!(
        KcallNumber::CondWait.into(),
        usize::from(cond_addr) as u32,
        usize::from(mutex_addr) as u32,
        seconds,
        nanoseconds
    );

    // NOTE: errors are unlikely because wait typically succeeds for valid condition/mutex.
    if unlikely(result != 0) {
        Err(Error::new(ErrorCode::try_from(result)?, "failed to wait condition variable"))
    } else {
        Ok(())
    }
}

//==================================================================================================
// Get Time
//==================================================================================================

///
/// # Description
///
/// Gets the current system time.
///
/// # Parameters
///
/// - `buffer`: A mutable reference to a buffer where the system time will be stored.
///
/// # Returns
///
/// Upon successful completion, `gettime()` returns empty. Upon failure, it returns an `Error` to
/// indicate the error.
///
pub fn gettime(buffer: &mut SystemTime) -> Result<(), Error> {
    let result: i64 =
        kcall1!(KcallNumber::GetTime.into(), buffer as *mut SystemTime as usize as u32);

    // NOTE: errors are unlikely because gettime typically succeeds for a valid buffer.
    if unlikely(result != 0) {
        Err(Error::new(ErrorCode::try_from(result)?, "failed to get time"))
    } else {
        Ok(())
    }
}

//==================================================================================================
// Sleep
//==================================================================================================

///
/// # Description
///
/// Puts the calling thread to sleep.
///
/// # Parameters
///
/// - `timeout`: Sleep duration.
///
/// # Returns
///
/// Upon successful completion, empty is returned. Otherwise, an error is returned instead.
///
pub fn sleep(timeout: Duration) -> Result<(), Error> {
    let seconds: u32 = timeout
        .as_secs()
        .try_into()
        .map_err(|_| Error::new(ErrorCode::InvalidArgument, "timeout value is too large"))?;
    let nanoseconds: u32 = timeout.subsec_nanos();

    let result: i64 = kcall2!(KcallNumber::Sleep.into(), seconds, nanoseconds);

    // NOTE: errors are unlikely because sleep typically succeeds for a valid timeout.
    if unlikely(result != 0) {
        Err(Error::new(ErrorCode::try_from(result)?, "failed to sleep"))
    } else {
        Ok(())
    }
}

//==================================================================================================
// Get Thread Data Area
//==================================================================================================

///
/// # Description
///
/// Gets the base address for the user-space thread data area of the calling thread.
///
/// # Returns
///
/// On successful completion, this function returns the base address for the user-space thread data
/// area of the calling thread. On failure, this function returns an error code that indicates the
/// reason of failure.
///
/// # Errors
///
/// This function fails with the following error codes:
///
/// - [`ErrorCode::ValueOutOfRange`]: The thread-local pointer cannot be represented correctly.
/// - [`ErrorCode::NoSuchEntry`]: The specified process or thread does not exist.
/// - [`ErrorCode::ResourceBusy`]: The process manager is busy and cannot handle the request.
///
pub fn get_thread_data_area() -> Result<*mut u8, Error> {
    let result: i64 = kcall0!(KcallNumber::GetThreadDataArea.into());

    // NOTE: errors are unlikely because get_thread_data_area typically succeeds.
    if unlikely(result < 0) {
        Err(Error::new(ErrorCode::try_from(result)?, "failed to get thread data area"))
    } else {
        Ok(result as *mut u8)
    }
}

//==================================================================================================
// Set Thread Data Area
//==================================================================================================

///
/// # Description
///
/// Sets the base address for the user-space thread data area of the calling thread.
///
/// # Parameters
///
/// - `user_tda`: Base address for the user-space thread data area. If null, clears the thread data area.
///
/// # Returns
///
/// On successful completion, this function returns empty. On failure, this function returns an
/// error code that indicates the reason of failure.
///
/// # Errors
///
/// This function fails with the following error codes:
///
/// - [`ErrorCode::InvalidArgument`]: The provided thread-local storage pointer is invalid.
/// - [`ErrorCode::NoSuchEntry`]: The specified process or thread does not exist.
/// - [`ErrorCode::ResourceBusy`]: The process manager is busy and cannot handle the request.
///
///
pub fn set_thread_data_area(user_tda: *mut u8) -> Result<(), Error> {
    let result: i64 = kcall1!(KcallNumber::SetThreadDataArea.into(), user_tda as usize as u32);

    // NOTE: errors are unlikely because set_thread_data_area typically succeeds.
    if unlikely(result != 0) {
        Err(Error::new(ErrorCode::try_from(result)?, "failed to set data area"))
    } else {
        Ok(())
    }
}

//==================================================================================================
// Snapshot
//==================================================================================================

///
/// # Description
///
/// Creates a snapshot of the virtual machine. The snapshot captures all guest memory and
/// processor state into files managed by the VMM.
///
/// # Returns
///
/// Upon successful completion, empty is returned. Upon failure, an error is returned instead.
///
pub fn snapshot() -> Result<(), Error> {
    let result: i64 = kcall0!(KcallNumber::Snapshot.into());

    if unlikely(result != 0) {
        Err(Error::new(ErrorCode::try_from(result)?, "failed to snapshot()"))
    } else {
        Ok(())
    }
}
