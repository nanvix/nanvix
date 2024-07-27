// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::arch::mem;

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
    pub fn from_raw_value(value: usize) -> Option<Self> {
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
    pub fn into_raw_value(self) -> usize {
        self.0
    }
}

//==================================================================================================
// Unit Tests
//==================================================================================================

/// Tests if [`FrameNumber::from_raw_value()`] successfully constructs frame zero.
fn test_frame_number_from_raw_value_zero() -> bool {
    let raw_value: usize = 0;
    match FrameNumber::from_raw_value(raw_value) {
        Some(frame_number) => {
            assert_eq!(frame_number.into_raw_value(), raw_value);
            true
        },
        None => false,
    }
}

/// Tests if [`FrameNumber::from_raw_value()`] successfully constructs the maximum frame number.
fn test_frame_number_from_raw_value_max() -> bool {
    let raw_value: usize = FrameNumber::MAX;
    match FrameNumber::from_raw_value(raw_value) {
        Some(frame_number) => {
            assert_eq!(frame_number.into_raw_value(), raw_value);
            true
        },
        None => false,
    }
}

/// Runs unit tests for [`FrameNumber`].
pub fn test() -> bool {
    let mut passed: bool = true;

    passed &= run_test!(test_frame_number_from_raw_value_zero);
    passed &= run_test!(test_frame_number_from_raw_value_max);

    passed
}
