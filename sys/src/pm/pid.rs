// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::error::{
    Error,
    ErrorCode,
};

//==================================================================================================
// Structures
//==================================================================================================

///
/// # Description
///
/// A type that represents a process identifier.
///
#[derive(Default, Clone, Copy, PartialEq, Eq)]
pub struct ProcessIdentifier(usize);

//==================================================================================================
// Implementations
//==================================================================================================

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