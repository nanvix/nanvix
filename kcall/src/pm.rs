// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::{
    Error,
    ErrorCode,
    KcallNumber,
};

//==================================================================================================
// Process Identifier
//==================================================================================================

///
/// # Description
///
/// A type that represents a process identifier.
///
#[derive(Default, Clone, Copy, PartialEq, Eq)]
pub struct ProcessIdentifier(usize);

impl ProcessIdentifier {
    pub const KERNEL: ProcessIdentifier = ProcessIdentifier(0);
}

impl core::fmt::Debug for ProcessIdentifier {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "{:?}", self.0)
    }
}

impl From<usize> for ProcessIdentifier {
    fn from(id: usize) -> ProcessIdentifier {
        ProcessIdentifier(id)
    }
}

impl From<ProcessIdentifier> for usize {
    fn from(pid: ProcessIdentifier) -> usize {
        pid.0
    }
}

impl From<ProcessIdentifier> for i32 {
    fn from(pid: ProcessIdentifier) -> i32 {
        pid.0 as i32
    }
}

impl From<ProcessIdentifier> for u32 {
    fn from(pid: ProcessIdentifier) -> u32 {
        pid.0 as u32
    }
}

impl TryFrom<i32> for ProcessIdentifier {
    type Error = Error;

    fn try_from(raw_pid: i32) -> Result<Self, Self::Error> {
        if raw_pid < 0 {
            Err(Error::new(ErrorCode::InvalidArgument, "invalid process identifier"))
        } else {
            Ok(ProcessIdentifier(raw_pid as usize))
        }
    }
}

//==================================================================================================
// Thread Identifier
//==================================================================================================

///
/// # Description
///
/// A type that represents a thread identifier.
///
#[derive(Clone, Copy, PartialEq, Eq)]
pub struct ThreadIdentifier(usize);

impl From<usize> for ThreadIdentifier {
    fn from(id: usize) -> ThreadIdentifier {
        ThreadIdentifier(id)
    }
}

impl From<ThreadIdentifier> for usize {
    fn from(tid: ThreadIdentifier) -> usize {
        tid.0
    }
}

impl From<ThreadIdentifier> for i32 {
    fn from(tid: ThreadIdentifier) -> i32 {
        tid.0 as i32
    }
}

impl TryFrom<i32> for ThreadIdentifier {
    type Error = Error;

    fn try_from(raw_tid: i32) -> Result<Self, Self::Error> {
        if raw_tid < 0 {
            Err(Error::new(ErrorCode::InvalidArgument, "invalid thread identifier"))
        } else {
            Ok(ThreadIdentifier(raw_tid as usize))
        }
    }
}

impl core::fmt::Debug for ThreadIdentifier {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "{:?}", self.0)
    }
}

//==================================================================================================
// User Identifier
//==================================================================================================

///
/// # Description
///
/// A type that represents a user identifier.
///
#[derive(Clone, Copy, PartialEq, Eq)]
pub struct UserIdentifier(usize);

impl UserIdentifier {
    pub const ROOT: UserIdentifier = UserIdentifier(0);
}

impl From<usize> for UserIdentifier {
    fn from(id: usize) -> UserIdentifier {
        UserIdentifier(id)
    }
}

impl From<u32> for UserIdentifier {
    fn from(id: u32) -> UserIdentifier {
        UserIdentifier(id as usize)
    }
}

impl From<UserIdentifier> for usize {
    fn from(uid: UserIdentifier) -> usize {
        uid.0
    }
}

impl From<UserIdentifier> for i32 {
    fn from(uid: UserIdentifier) -> i32 {
        uid.0 as i32
    }
}

impl TryFrom<i32> for UserIdentifier {
    type Error = Error;

    fn try_from(raw_uid: i32) -> Result<Self, Self::Error> {
        if raw_uid < 0 {
            Err(Error::new(ErrorCode::InvalidArgument, "invalid user identifier"))
        } else {
            Ok(UserIdentifier(raw_uid as usize))
        }
    }
}

impl core::fmt::Debug for UserIdentifier {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "{:?}", self.0)
    }
}

//==================================================================================================
// Capability
//==================================================================================================

///
/// # Description
///
/// A type that represents a capability.
///
#[derive(Debug, Clone, Copy)]
pub enum Capability {
    /// Exception control.
    ExceptionControl,
    /// Interrupt control.
    InterruptControl,
    /// I/O management.
    IoManagement,
    /// Memory management.
    MemoryManagement,
    /// Process management.
    ProcessManagement,
}

impl TryFrom<u32> for Capability {
    type Error = Error;

    fn try_from(value: u32) -> Result<Self, Self::Error> {
        match value {
            0 => Ok(Capability::ExceptionControl),
            1 => Ok(Capability::InterruptControl),
            2 => Ok(Capability::IoManagement),
            3 => Ok(Capability::MemoryManagement),
            4 => Ok(Capability::ProcessManagement),
            _ => Err(Error::new(ErrorCode::InvalidArgument, "invalid capability")),
        }
    }
}

//==================================================================================================
// Group Identifier
//==================================================================================================

///
/// # Description
///
/// A type that represents a group identifier.
///
#[derive(Clone, Copy, PartialEq, Eq)]
pub struct GroupIdentifier(usize);

impl GroupIdentifier {
    pub const ROOT: GroupIdentifier = GroupIdentifier(0);
}

impl From<usize> for GroupIdentifier {
    fn from(id: usize) -> GroupIdentifier {
        GroupIdentifier(id)
    }
}

impl From<u32> for GroupIdentifier {
    fn from(id: u32) -> GroupIdentifier {
        GroupIdentifier(id as usize)
    }
}

impl From<GroupIdentifier> for usize {
    fn from(gid: GroupIdentifier) -> usize {
        gid.0
    }
}

impl From<GroupIdentifier> for i32 {
    fn from(gid: GroupIdentifier) -> i32 {
        gid.0 as i32
    }
}

impl TryFrom<i32> for GroupIdentifier {
    type Error = Error;

    fn try_from(raw_gid: i32) -> Result<Self, Self::Error> {
        if raw_gid < 0 {
            Err(Error::new(ErrorCode::InvalidArgument, "invalid group identifier"))
        } else {
            Ok(GroupIdentifier(raw_gid as usize))
        }
    }
}

impl core::fmt::Debug for GroupIdentifier {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "{:?}", self.0)
    }
}

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
    let result: i32 = unsafe { crate::kcall0(KcallNumber::GetPid.into()) };

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
    let result: i32 = unsafe { crate::kcall0(KcallNumber::GetTid.into()) };

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
    let result: i32 = unsafe { crate::kcall0(KcallNumber::GetUid.into()) };
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
    let result: i32 = unsafe { crate::kcall0(KcallNumber::GetEuid.into()) };
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
    let result: i32 = unsafe { crate::kcall0(KcallNumber::GetGid.into()) };
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
    let result: i32 = unsafe { crate::kcall0(KcallNumber::GetEgid.into()) };
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
    let result: i32 = unsafe { crate::kcall1(KcallNumber::Exit.into(), status as u32) };
    Err(Error::new(ErrorCode::try_from(result)?, "failed to terminate process"))
}

//==================================================================================================
// Capability Control
//==================================================================================================

pub fn capctl(capability: Capability, value: bool) -> Result<(), Error> {
    let result: i32 =
        unsafe { crate::kcall2(KcallNumber::CapCtl.into(), capability as u32, value as u32) };

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
        unsafe { crate::kcall1(KcallNumber::Terminate.into(), usize::from(pid) as u32) };

    if result == 0 {
        Ok(())
    } else {
        Err(Error::new(ErrorCode::try_from(result)?, "failed to terminate()"))
    }
}
