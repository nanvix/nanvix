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
    number::KcallNumber,
    pm::{
        Capability,
        ConditionAddress,
        MutexAddress,
        ProcessIdentifier,
        ThreadIdentifier,
    },
};
use ::time::SystemTime;

//==================================================================================================
// Get Process Identifier
//==================================================================================================

///
/// # Description
///
/// Gets the process identifier of the calling process.
///
/// # Return Values
///
/// Upon successful completion, the process identifier of the calling process is returned. Upon
/// failure, an error is returned instead.
///
pub fn getpid() -> Result<ProcessIdentifier, Error> {
    let result: i64 = kcall0!(KcallNumber::GetPid.into());

    ProcessIdentifier::try_from(result)
}

//==================================================================================================
// Get Thread Identifier
//==================================================================================================

///
/// # Description
///
/// Gets the thread identifier of the calling thread.
///
/// # Return Values
///
/// Upon successful completion, the thread identifier of the calling thread is returned. Upon
/// failure, an error is returned instead.
///
pub fn gettid() -> Result<ThreadIdentifier, Error> {
    let result: i64 = kcall0!(KcallNumber::GetTid.into());

    ThreadIdentifier::try_from(result)
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
/// # Return Values
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

pub fn capctl(capability: Capability, value: bool) -> Result<(), Error> {
    let result: i64 = kcall2!(KcallNumber::CapCtl.into(), capability as u32, value as u32);

    if result == 0 {
        Ok(())
    } else {
        Err(Error::new(ErrorCode::try_from(result)?, "failed to capctl()"))
    }
}

//==================================================================================================
// Terminate
//==================================================================================================

pub fn terminate(pid: ProcessIdentifier) -> Result<(), Error> {
    let result: i64 = kcall1!(KcallNumber::Terminate.into(), usize::from(pid) as u32);

    if result == 0 {
        Ok(())
    } else {
        Err(Error::new(ErrorCode::try_from(result)?, "failed to terminate()"))
    }
}

//==================================================================================================
// Create Thread
//==================================================================================================

::core::arch::global_asm!(concat!(
    ".global _do_start_thread\n",
    ".extern _start_thread\n",
    ".type _do_start_thread, @function\n",
    "_do_start_thread:\n",
    // Start call stack frame
    "    mov ebp, esp\n",
    "    push ecx\n",
    "    push edx\n",
    "    call _start_thread\n",
    "1: jmp 1b"
));

#[unsafe(no_mangle)]
pub extern "C" fn _start_thread(func: extern "C" fn(usize) -> usize, arg: usize) -> ! {
    let status = func(arg);
    let _ = exit_thread(status);
    unreachable!("failed to exit thread");
}

pub fn create_thread(
    user_fn: extern "C" fn(usize) -> usize,
    arg: usize,
) -> Result<ThreadIdentifier, Error> {
    unsafe extern "C" {
        fn _do_start_thread() -> !;
    }

    let result: i64 = kcall3!(
        KcallNumber::CreateThread.into(),
        _do_start_thread as usize as u32,
        user_fn as usize as u32,
        arg as u32
    );

    ThreadIdentifier::try_from(result)
}

//==================================================================================================
// Exit Thread
//==================================================================================================

pub fn exit_thread(status: usize) -> Result<!, Error> {
    let result: i64 = kcall1!(KcallNumber::ExitThread.into(), status as u32);

    Err(Error::new(ErrorCode::try_from(result)?, "failed to terminate thread"))
}

//==================================================================================================
// Join Thread
//==================================================================================================

pub fn join_thread(tid: ThreadIdentifier, retval: &mut usize) -> Result<i64, Error> {
    let result: i64 = kcall2!(
        KcallNumber::JoinThread.into(),
        usize::from(tid) as u32,
        retval as *mut usize as u32
    );

    if result != 0 {
        Err(Error::new(ErrorCode::try_from(result)?, "failed to join thread"))
    } else {
        Ok(result)
    }
}

//==================================================================================================
// Lock Mutex
//==================================================================================================

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

    if result == 0 {
        Ok(())
    } else {
        Err(Error::new(ErrorCode::try_from(result)?, "failed to lock mutex"))
    }
}

//==================================================================================================
// Unlock Mutex
//==================================================================================================

pub fn unlock_mutex(mutex_addr: MutexAddress) -> Result<(), Error> {
    let result: i64 = kcall1!(KcallNumber::MutexUnlock.into(), usize::from(mutex_addr) as u32);

    if result == 0 {
        Ok(())
    } else {
        Err(Error::new(ErrorCode::try_from(result)?, "failed to unlock mutex"))
    }
}

//==================================================================================================
// Signal Condition Variable
//==================================================================================================

pub fn signal_cond(cond_addr: ConditionAddress, broadcast: bool) -> Result<usize, Error> {
    let result: i64 = kcall4!(
        KcallNumber::CondSignal.into(),
        usize::from(cond_addr) as u32,
        broadcast as u32,
        u32::MAX,
        u32::MAX
    );

    if result >= 0 {
        Ok(result as usize)
    } else {
        Err(Error::new(ErrorCode::try_from(result)?, "failed to signal condition variable"))
    }
}

//==================================================================================================
// Wait Condition Variable
//==================================================================================================

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

    if result == 0 {
        Ok(())
    } else {
        Err(Error::new(ErrorCode::try_from(result)?, "failed to wait condition variable"))
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

    if result == 0 {
        Ok(())
    } else {
        Err(Error::new(ErrorCode::try_from(result)?, "failed to get time"))
    }
}
