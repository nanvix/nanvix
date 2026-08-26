// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//! Descriptor access mode.

//==================================================================================================
// Enumerations
//==================================================================================================

/// Access permitted by an open file description.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AccessMode {
    /// Reads are permitted.
    ReadOnly,
    /// Writes are permitted.
    WriteOnly,
    /// Reads and writes are permitted.
    ReadWrite,
}

//==================================================================================================
// Implementations
//==================================================================================================

impl AccessMode {
    /// Returns whether reads are permitted.
    pub fn readable(self) -> bool {
        matches!(self, Self::ReadOnly | Self::ReadWrite)
    }

    /// Returns whether writes are permitted.
    pub fn writable(self) -> bool {
        matches!(self, Self::WriteOnly | Self::ReadWrite)
    }
}
