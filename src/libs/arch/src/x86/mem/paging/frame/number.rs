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
#[verus_verify(external_derive)]
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
    /// Converts a [`FrameNumber`] into a raw value.
    ///
    /// # Returns
    ///
    /// The raw value of the target [`FrameNumber`].
    ///
    // Pure newtype-identity projection: the returned raw value equals the
    // abstract frame index (`result as int == self@`). The type invariant
    // additionally bounds it to `0 ..= spec_max_frame_number()`, which callers
    // (`pde.rs`/`pte.rs` `<< FRAME_SHIFT`, `phys.rs::from_number` `* FRAME_SIZE`,
    // `frame.rs` refcount indexing) rely on for their no-overflow proofs.
    #[verus_spec(result =>
        ensures
            result as int == self@,
            0 <= result as int <= spec_max_frame_number(),
    )]
    pub fn into_raw_value(self) -> usize {
        self.0
    }
}

#[verus_verify]
impl FrameNumber {
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
    // Validating constructor. `Some` is returned **iff** `value` is a
    // representable frame index (`value <= spec_max_frame_number()`), which is
    // the only failure signal (`None` otherwise — bidirectional). On success the
    // index is preserved exactly (`f@ == value as int`) and the result is
    // well-formed (`f.inv()`). Together with `into_raw_value` this yields the
    // round-trip identity `from_raw_value(v).unwrap().into_raw_value() == v`.
    #[verus_spec(result =>
        ensures
            (result is Some) <==> (value as int <= spec_max_frame_number()),
            result matches Some(f) ==> f@ == value as int && f.inv(),
    )]
    pub fn from_raw_value(value: usize) -> Option<Self> {
        if value > Self::MAX {
            return None;
        }

        Some(Self(value))
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
