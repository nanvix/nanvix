// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//! User VM identifiers shared by standalone host components.

//==================================================================================================
// Configuration
//==================================================================================================

#![forbid(clippy::unwrap_used)]
#![forbid(clippy::expect_used)]

//==================================================================================================
// Imports
//==================================================================================================

use ::serde::{
    Deserialize,
    Serialize,
};
use ::std::fmt;

//==================================================================================================
// Structures
//==================================================================================================

/// Unique identifier for a user VM.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Ord, PartialOrd, Hash, Serialize, Deserialize)]
pub struct UserVmIdentifier {
    /// Underlying numeric identifier.
    value: u32,
}

//==================================================================================================
// Implementations
//==================================================================================================

impl UserVmIdentifier {
    /// Creates a user VM identifier from a raw value.
    pub const fn new(value: u32) -> Self {
        Self { value }
    }
}

impl From<UserVmIdentifier> for u32 {
    fn from(identifier: UserVmIdentifier) -> Self {
        identifier.value
    }
}

impl fmt::Display for UserVmIdentifier {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{}", self.value)
    }
}
