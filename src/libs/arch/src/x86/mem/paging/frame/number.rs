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
#[verus_verify]
#[derive(Debug, Clone, Copy)]
pub struct FrameNumber(usize);

//==================================================================================================
// Implementations
//==================================================================================================

#[verus_verify]
impl FrameNumber {
    /// The maximum frame number.
    ///
    /// This is the number of the highest frame whose entire extent fits within the addressable
    /// space `[0, MAX_ADDRESS]`. A frame `f` spans the byte range
    /// `[f * FRAME_SIZE, (f + 1) * FRAME_SIZE - 1]`, so the last fully-addressable frame is
    /// `(MAX_ADDRESS + 1) / FRAME_SIZE - 1`. That expression is rewritten below to avoid
    /// overflowing `usize` when `MAX_ADDRESS == usize::MAX`:
    ///
    /// - If `MAX_ADDRESS` is the last byte of its frame (`MAX_ADDRESS % FRAME_SIZE ==
    ///   FRAME_SIZE - 1`), that top frame is wholly addressable and is therefore the maximum.
    /// - Otherwise the top frame is only partially addressable, so the preceding frame is the
    ///   maximum.
    pub const MAX: usize = if mem::MAX_ADDRESS % mem::FRAME_SIZE == mem::FRAME_SIZE - 1 {
        mem::MAX_ADDRESS / mem::FRAME_SIZE
    } else {
        mem::MAX_ADDRESS / mem::FRAME_SIZE - 1
    };

    #[verus_spec(ensures Self::NULL@ == 0)]
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
    #[verus_spec(result =>
        ensures
            value as int <= Self::spec_max() ==> (result is Some
                && (result->Some_0)@ == value as int),
            value as int > Self::spec_max() ==> result is None,
    )]
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
    #[verus_spec(result =>
        ensures
            result as int == self@,
            0 <= self@ <= Self::spec_max(),
    )]
    pub fn into_raw_value(self) -> usize {
        self.0
    }
}

//==================================================================================================
// Unit Tests
//==================================================================================================

/// Tests if [`FrameNumber::from_raw_value()`] successfully constructs frame zero.
#[test]
fn test_frame_number_from_raw_value_zero() {
    let raw_value: usize = 0;
    let frame_number: FrameNumber = FrameNumber::from_raw_value(raw_value).unwrap();
    assert_eq!(frame_number.into_raw_value(), raw_value);
}

/// Tests if [`FrameNumber::from_raw_value()`] successfully constructs the maximum frame number.
#[test]
fn test_frame_number_from_raw_value_max() {
    let raw_value: usize = FrameNumber::MAX;
    let frame_number: FrameNumber = FrameNumber::from_raw_value(raw_value).unwrap();
    assert_eq!(frame_number.into_raw_value(), raw_value);
}

/// Regression test: the frame that contains the top-of-space address must be representable as a
/// [`FrameNumber`]. With an off-by-one [`FrameNumber::MAX`], converting such an address panicked.
#[test]
fn test_frame_number_top_of_space_is_representable() {
    // Frame number of the highest addressable byte.
    let top_frame: usize = mem::MAX_ADDRESS / mem::FRAME_SIZE;
    assert_eq!(FrameNumber::MAX, top_frame);
    assert!(FrameNumber::from_raw_value(top_frame).is_some());
}
