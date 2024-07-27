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
    number::KcallNumber,
};

//==================================================================================================
// Exports
//==================================================================================================

pub use ::sys::pm::*;

//==================================================================================================
/// Get Process Identifier
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
    let result: i32 = unsafe { crate::arch::kcall0(KcallNumber::GetPid.into()) };

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
    let result: i32 = unsafe { crate::arch::kcall0(KcallNumber::GetTid.into()) };

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
    let result: i32 = unsafe { crate::arch::kcall0(KcallNumber::GetUid.into()) };
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
    let result: i32 = unsafe { crate::arch::kcall0(KcallNumber::GetEuid.into()) };
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
    let result: i32 = unsafe { crate::arch::kcall0(KcallNumber::GetGid.into()) };
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
    let result: i32 = unsafe { crate::arch::kcall0(KcallNumber::GetEgid.into()) };
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
    let result: i32 = unsafe { crate::arch::kcall1(KcallNumber::Exit.into(), status as u32) };
    Err(Error::new(ErrorCode::try_from(result)?, "failed to terminate process"))
}

//==================================================================================================
// Capability Control
//==================================================================================================

pub fn capctl(capability: Capability, value: bool) -> Result<(), Error> {
    let result: i32 =
        unsafe { crate::arch::kcall2(KcallNumber::CapCtl.into(), capability as u32, value as u32) };

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
    let result: i32 =
        unsafe { crate::arch::kcall1(KcallNumber::Terminate.into(), usize::from(pid) as u32) };

    if result == 0 {
        Ok(())
    } else {
        Err(Error::new(ErrorCode::try_from(result)?, "failed to terminate()"))
    }
}
