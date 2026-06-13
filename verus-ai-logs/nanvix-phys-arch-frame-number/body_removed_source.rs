// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use vstd::prelude::*;
#[cfg(verus_keep_ghost)]
include!("number.spec.rs");
#[cfg(verus_keep_ghost)]
include!("number.proof.rs");

use crate::mem;

//==================================================================================================
// Structures
//==================================================================================================

///
/// # Description
///
/// A type that represents a frame number.
/// A frame number is in the range from `0` to [`Self::MAX`] (inclusive).
///
#[derive(Debug, Clone, Copy)]
pub struct FrameNumber(usize);

//==================================================================================================
// Implementations
//==================================================================================================

impl FrameNumber {
    /// The maximum frame number.
    pub const MAX: usize = mem::MAX_ADDRESS / mem::FRAME_SIZE - 1;

    pub const NULL: Self = Self(0);

    ///
    /// # Description
    ///
    /// Constructs a [`FrameNumber`].
    ///
    /// # Parameters
    ///
    /// - `value`: The value of the frame number.
    ///
    /// # Returns
    ///
    /// - `Some(`[`FrameNumber`]`)`: Upon success.
    /// - `None`: If the value is greater than [`Self::MAX`].
    ///
    pub fn from_raw_value(value: usize) -> Option<Self> { ... }

    ///
    /// # Description
    ///
    /// Converts a [`FrameNumber`] into a raw value.
    ///
    /// # Returns
    ///
    /// The raw value of the target [`FrameNumber`].
    ///
    pub fn into_raw_value(self) -> usize { ... }
}

//==================================================================================================
// Unit Tests
//==================================================================================================

/// Tests if [`FrameNumber::from_raw_value()`] successfully constructs frame zero.
#[test]
fn test_frame_number_from_raw_value_zero() { ... }

/// Tests if [`FrameNumber::from_raw_value()`] successfully constructs the maximum frame number.
#[test]
fn test_frame_number_from_raw_value_max() { ... }
