// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::memory::{
    ReadBytes,
    ReadBytesError,
};

//==================================================================================================
// Structures
//==================================================================================================

/// An address in memory.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
#[repr(C)]
pub struct Address(u32);

//==================================================================================================
// Implementations
//==================================================================================================

impl Address {
    /// Creates a new address.
    pub fn new(val: u32) -> Self {
        Self(val)
    }

    /// Returns the value of the address.
    pub fn value(&self) -> u32 {
        self.0
    }
}

impl ReadBytes for Address {
    fn read_le_bytes(from: &[u8]) -> Result<Self, ReadBytesError> {
        Ok(Self(u32::read_le_bytes(from)?))
    }
}
