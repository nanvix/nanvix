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
    number::KcallNumber,
    pm::{
        Capability,
        GroupIdentifier,
        ProcessIdentifier,
        ThreadIdentifier,
        UserIdentifier,
    },
};

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
    let result: i32 = kcall0!(KcallNumber::GetPid.into());

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
    let result: i32 = kcall0!(KcallNumber::GetTid.into());

    ThreadIdentifier::try_from(result)
}

//==================================================================================================
// Get User Identifier
//==================================================================================================

///
/// # Description
///
/// Gets the user identifier of the calling process.
///
/// # Return Values
///
/// Upon successful completion, the user identifier of the calling process is returned. Upon
/// failure, an error is returned instead.
///
pub fn getuid() -> Result<UserIdentifier, Error> {
    let result: i32 = kcall0!(KcallNumber::GetUid.into());
    UserIdentifier::try_from(result)
}

//==================================================================================================
// Get Effective User Identifier
//==================================================================================================

///
/// # Description
///
/// Gets the effective user identifier of the calling process.
///
/// # Return Values
///
/// Upon successful completion, the effective user identifier of the calling process is returned.
/// Upon failure, an error is returned instead.
///
pub fn geteuid() -> Result<UserIdentifier, Error> {
    let result: i32 = kcall0!(KcallNumber::GetEuid.into());
    UserIdentifier::try_from(result)
}

//==================================================================================================
// Get Group Identifier
//==================================================================================================

///
/// # Description
///
/// Gets the group identifier of the calling process.
///
/// # Return Values
///
/// Upon successful completion, the group identifier of the calling process is returned. Upon
/// failure, an error is returned instead.
///
pub fn getgid() -> Result<GroupIdentifier, Error> {
    let result: i32 = kcall0!(KcallNumber::GetGid.into());
    GroupIdentifier::try_from(result)
}

//==================================================================================================
// Get Effective Group Identifier
//==================================================================================================

///
/// # Description
///
/// Gets the effective group identifier of the calling process.
///
/// # Return Values
///
/// Upon successful completion, the effective group identifier of the calling process is returned.
/// Upon failure, an error is returned instead.
///
pub fn getegid() -> Result<GroupIdentifier, Error> {
    let result: i32 = kcall0!(KcallNumber::GetEgid.into());
    GroupIdentifier::try_from(result)
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
    let result: i32 = kcall1!(KcallNumber::Exit.into(), status as u32);
    Err(Error::new(ErrorCode::try_from(result)?, "failed to terminate process"))
}

//==================================================================================================
// Capability Control
//==================================================================================================

pub fn capctl(capability: Capability, value: bool) -> Result<(), Error> {
    let result: i32 = kcall2!(KcallNumber::CapCtl.into(), capability as u32, value as u32);

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
    let result: i32 = kcall1!(KcallNumber::Terminate.into(), usize::from(pid) as u32);

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

#[no_mangle]
pub extern "C" fn _start_thread(func: extern "C" fn(usize) -> usize, arg: usize) -> ! {
    let status = func(arg);
    let _ = exit_thread(status);
    unreachable!("failed to exit thread");
}

pub fn create_thread(user_fn: fn(usize) -> usize, arg: usize) -> Result<ThreadIdentifier, Error> {
    extern "C" {
        fn _do_start_thread() -> !;
    }

    let result: i32 = kcall3!(
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
    let result: i32 = kcall1!(KcallNumber::ExitThread.into(), status as u32);

    Err(Error::new(ErrorCode::try_from(result)?, "failed to terminate thread"))
}

//==================================================================================================
// Join Thread
//==================================================================================================

pub fn join_thread(tid: ThreadIdentifier, retval: &mut usize) -> Result<i32, Error> {
    let result: i32 = kcall2!(
        KcallNumber::JoinThread.into(),
        usize::from(tid) as u32,
        retval as *mut usize as u32
    );

    if result < 0 {
        Err(Error::new(ErrorCode::try_from(result)?, "failed to join thread"))
    } else {
        Ok(result)
    }
}
