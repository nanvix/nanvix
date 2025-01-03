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

//==================================================================================================
// Implementations
//==================================================================================================

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
