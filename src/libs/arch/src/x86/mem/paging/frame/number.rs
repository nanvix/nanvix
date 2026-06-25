// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::mem;
use ::sys::mm::{
    Address,
    PhysicalAddress,
    VirtualAddress,
};

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

impl From<FrameNumber> for PhysicalAddress {
    fn from(frame_number: FrameNumber) -> Self {
        let raw_addr: usize = frame_number.into_raw_value() * mem::FRAME_SIZE;
        // SAFETY: frame-number conversions intentionally bypass the RAM-only physical-address
        // validator. Page-table entries may refer to MMIO frames, such as the LAPIC page near the
        // top of the u32 address space.
        unsafe { PhysicalAddress::from_mmio_address(VirtualAddress::from_raw_value(raw_addr)) }
    }
}

impl From<PhysicalAddress> for FrameNumber {
    fn from(phys_addr: PhysicalAddress) -> Self {
        let raw_addr: usize = phys_addr.into_raw_value();
        let frame_number: usize = raw_addr >> mem::FRAME_SHIFT;
        // The unwrap below never panics: `FrameNumber::MAX` is the number of the frame that
        // contains `MAX_ADDRESS`, so `raw_addr >> FRAME_SHIFT <= FrameNumber::MAX` holds for
        // every address in the space.
        FrameNumber::from_raw_value(frame_number).unwrap()
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
