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
/// A type that represents a thread identifier.
///
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct ThreadIdentifier(usize);

//==================================================================================================
// Implementations
//==================================================================================================

impl From<usize> for ThreadIdentifier {
    fn from(id: usize) -> ThreadIdentifier {
        ThreadIdentifier(id)
    }
}

impl From<u32> for ThreadIdentifier {
    fn from(id: u32) -> ThreadIdentifier {
        ThreadIdentifier(id as usize)
    }
}

impl From<ThreadIdentifier> for usize {
    fn from(tid: ThreadIdentifier) -> usize {
        tid.0
    }
}

impl From<ThreadIdentifier> for u32 {
    fn from(tid: ThreadIdentifier) -> u32 {
        tid.0 as u32
    }
}

impl TryFrom<ThreadIdentifier> for i32 {
    type Error = Error;

    fn try_from(tid: ThreadIdentifier) -> Result<Self, Self::Error> {
        if tid.0 > i32::MAX as usize {
            Err(Error::new(ErrorCode::InvalidArgument, "invalid thread identifier"))
        } else {
            Ok(tid.0 as i32)
        }
    }
}

impl TryFrom<ThreadIdentifier> for i64 {
    type Error = Error;

    fn try_from(tid: ThreadIdentifier) -> Result<Self, Self::Error> {
        if tid.0 > i64::MAX as usize {
            Err(Error::new(ErrorCode::InvalidArgument, "invalid thread identifier"))
        } else {
            Ok(tid.0 as i64)
        }
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

impl TryFrom<i64> for ThreadIdentifier {
    type Error = Error;

    fn try_from(raw_tid: i64) -> Result<Self, Self::Error> {
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
