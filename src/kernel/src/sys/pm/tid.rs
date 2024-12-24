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
#[derive(Clone, Copy, PartialEq, Eq)]
pub struct ThreadIdentifier(usize);

//==================================================================================================
// Implementations
//==================================================================================================

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
