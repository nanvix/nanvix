// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

use super::*;

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
