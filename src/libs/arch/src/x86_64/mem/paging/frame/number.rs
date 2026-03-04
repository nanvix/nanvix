// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::mem;

//==================================================================================================
// Structures
//==================================================================================================

///
/// # Description
///
/// A type that represents a frame number.
/// On x86_64, frame numbers address up to 52-bit physical addresses,
/// so the maximum frame number is `(1 << 52) / FRAME_SIZE - 1`.
///
#[derive(Debug, Clone, Copy)]
pub struct FrameNumber(u64);

//==================================================================================================
// Implementations
//==================================================================================================

impl FrameNumber {
    /// Maximum frame number (52-bit physical address space).
    pub const MAX: u64 = (mem::MAX_PHYSICAL_ADDRESS >> mem::FRAME_SHIFT as u64) as u64;

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
    pub fn from_raw_value(value: u64) -> Option<Self> {
        if value > Self::MAX {
            return None;
        }

        Some(Self(value))
    }

    ///
    /// # Description
    ///
    /// Converts a [`FrameNumber`] into a raw value.
    ///
    /// # Returns
    ///
    /// The raw value of the target [`FrameNumber`].
    ///
    pub fn into_raw_value(self) -> u64 {
        self.0
    }
}

//==================================================================================================
// Unit Tests
//==================================================================================================

/// Tests if [`FrameNumber::from_raw_value()`] successfully constructs frame zero.
#[test]
fn test_frame_number_from_raw_value_zero() {
    let raw_value: u64 = 0;
    let frame_number: FrameNumber = FrameNumber::from_raw_value(raw_value).unwrap();
    assert_eq!(frame_number.into_raw_value(), raw_value);
}

/// Tests if [`FrameNumber::from_raw_value()`] successfully constructs the maximum frame number.
#[test]
fn test_frame_number_from_raw_value_max() {
    let raw_value: u64 = FrameNumber::MAX;
    let frame_number: FrameNumber = FrameNumber::from_raw_value(raw_value).unwrap();
    assert_eq!(frame_number.into_raw_value(), raw_value);
}
