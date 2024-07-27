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
/// A type that represents a group identifier.
///
#[derive(Clone, Copy, PartialEq, Eq)]
pub struct GroupIdentifier(usize);

//==================================================================================================
// Implementations
//==================================================================================================

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
