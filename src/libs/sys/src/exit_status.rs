// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Lint Configuration
//===================================================================================================

#![deny(clippy::as_conversions)]
#![forbid(clippy::unwrap_used)]
#![forbid(clippy::expect_used)]
#![forbid(clippy::panic)]
#![forbid(clippy::unimplemented)]
#![forbid(clippy::todo)]
#![forbid(clippy::unreachable)]

//==================================================================================================
// Imports
//==================================================================================================

use ::error::ErrorCode;

//==================================================================================================
// Structure
//==================================================================================================

///
/// # Description
///
/// A structure that represents the exit status of a process.
///
#[derive(Debug, PartialEq, Eq, Clone, Copy)]
#[repr(C)]
pub struct ExitStatus(u32);

//==================================================================================================
// Implementations
//==================================================================================================

impl ExitStatus {
    ///
    /// # Description
    ///
    /// Creates a new [`ExitStatus`] with a success exit status code.
    ///
    /// # Returns
    ///
    /// A new [`ExitStatus`] with a success exit status code.
    ///
    pub fn ok() -> Self {
        Self(0)
    }

    ///
    /// # Description
    ///
    /// Returns the memory representation of the target [`ExitStatus`] as a byte array in native
    /// byte order.
    ///
    /// # Returns
    ///
    /// A byte array in native byte order representing the target [`ExitStatus`].
    ///
    pub fn to_ne_bytes(self) -> [u8; 4] {
        self.0.to_ne_bytes()
    }

    ///
    /// # Description
    ///
    /// Creates a [`ExitStatus`] from the given byte array in native byte order.
    ///
    /// # Parameters
    ///
    /// - `bytes`: The byte array in native byte order.
    ///
    /// # Returns
    ///
    /// A new [`ExitStatus`] created from the given byte array.
    ///
    pub fn from_ne_bytes(bytes: &[u8; 4]) -> Self {
        Self(u32::from_ne_bytes(*bytes))
    }
}

#[cfg(target_pointer_width = "32")]
#[allow(clippy::as_conversions)]
impl From<ExitStatus> for usize {
    fn from(code: ExitStatus) -> Self {
        code.0 as usize
    }
}

#[cfg(target_pointer_width = "32")]
#[allow(clippy::as_conversions)]
impl From<ExitStatus> for u32 {
    fn from(code: ExitStatus) -> Self {
        code.0
    }
}

impl From<ErrorCode> for ExitStatus {
    fn from(error: ErrorCode) -> Self {
        Self(error.into())
    }
}

#[cfg(target_pointer_width = "32")]
#[allow(clippy::as_conversions)]
impl From<usize> for ExitStatus {
    fn from(code: usize) -> Self {
        Self(code as u32)
    }
}

impl From<u32> for ExitStatus {
    fn from(code: u32) -> Self {
        Self(code)
    }
}
