// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use ::sys::error::{
    Error,
    ErrorCode,
};

//==================================================================================================
// Structures
//==================================================================================================

///
/// # Description
///
/// A type that encodes the bit width of an I/O port.
///
#[derive(Debug)]
pub enum IoPortWidth {
    // 8-bit I/O port.
    Bits8,
    // 16-bit I/O port.
    Bits16,
    // 32-bit I/O port.
    Bits32,
}

//==================================================================================================
// Implementations
//==================================================================================================

impl TryFrom<usize> for IoPortWidth {
    type Error = Error;

    ///
    /// # Description
    ///
    /// Attempts to convert a [`usize`] to an I/O port width.
    ///
    /// # Parameters
    ///
    /// - `value`: [`usize`] value to convert.
    ///
    /// # Returns
    ///
    /// On success, the I/O port width is returned. On failure, an error is returned instead.
    ///
    fn try_from(value: usize) -> Result<Self, Self::Error> {
        match value {
            0 => Ok(Self::Bits8),
            1 => Ok(Self::Bits16),
            2 => Ok(Self::Bits32),
            _ => {
                let reason: &str = "invalid io port width";
                error!("try_from(): {}", reason);
                Err(Error::new(ErrorCode::InvalidArgument, reason))
            },
        }
    }
}

impl TryFrom<u32> for IoPortWidth {
    type Error = Error;

    ///
    /// # Description
    ///
    /// Attempts to convert a [`u32`] to an I/O port width.
    ///
    /// # Parameters
    ///
    /// - `value`: [`u32`] value to convert.
    ///
    /// # Returns
    ///
    /// On success, the I/O port width is returned. On failure, an error is returned instead.
    ///
    fn try_from(value: u32) -> Result<Self, Self::Error> {
        Self::try_from(value as usize)
    }
}
