// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use ::kcall::{
    Error,
    ErrorCode,
};

//==================================================================================================
// Structures
//==================================================================================================

///
/// # Description
///
/// A type that encodes the type of an I/O port.
///
#[derive(Debug, PartialEq, Eq)]
pub enum IoPortType {
    ReadOnly,
    WriteOnly,
    ReadWrite,
}

//==================================================================================================
// Implementations
//==================================================================================================

impl TryFrom<usize> for IoPortType {
    type Error = Error;

    ///
    /// # Description
    ///
    /// Attempts to convert a [`usize`] to an I/O port type.
    ///
    /// # Parameters
    ///
    /// - `value`: [`usize`] value to convert.
    ///
    /// # Returns
    ///
    /// On success, the I/O port type is returned. On failure, an error is returned instead.
    ///
    fn try_from(value: usize) -> Result<Self, Self::Error> {
        match value {
            0 => Ok(Self::ReadOnly),
            1 => Ok(Self::WriteOnly),
            2 => Ok(Self::ReadWrite),
            _ => {
                let reason: &str = "invalid io port type";
                error!("try_from(): {}", reason);
                Err(Error::new(ErrorCode::InvalidArgument, reason))
            },
        }
    }
}

impl TryFrom<u32> for IoPortType {
    type Error = Error;

    ///
    /// # Description
    ///
    /// Attempts to convert a [`u32`] to an I/O port type.
    ///
    /// # Parameters
    ///
    /// - `value`: [`u32`] value to convert.
    ///
    /// # Returns
    ///
    /// On success, the I/O port type is returned. On failure, an error is returned instead.
    ///
    fn try_from(value: u32) -> Result<Self, Self::Error> {
        Self::try_from(value as usize)
    }
}
