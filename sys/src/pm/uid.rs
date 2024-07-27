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
/// A type that represents a user identifier.
///
#[derive(Clone, Copy, PartialEq, Eq)]
pub struct UserIdentifier(usize);

//==================================================================================================
// Implementations
//==================================================================================================

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
