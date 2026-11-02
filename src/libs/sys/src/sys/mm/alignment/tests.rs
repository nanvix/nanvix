// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

use super::*;

#[test]
fn test_align_up_basic() {
    assert_eq!(align_up(1, Alignment::Align4), Some(4));
    assert_eq!(align_up(4, Alignment::Align4), Some(4));
    assert_eq!(align_up(5, Alignment::Align4), Some(8));
}

#[test]
fn test_align_up_zero() {
    assert_eq!(align_up(0, Alignment::Align4), Some(0));
    assert_eq!(align_up(0, Alignment::Align4096), Some(0));
}

#[test]
fn test_align_up_already_aligned() {
    assert_eq!(align_up(4096, Alignment::Align4096), Some(4096));
    assert_eq!(align_up(8192, Alignment::Align4096), Some(8192));
}

#[test]
fn test_align_up_overflow_returns_none() {
    assert_eq!(align_up(usize::MAX, Alignment::Align4096), None);
    assert_eq!(align_up(usize::MAX - 1, Alignment::Align4096), None);
    assert_eq!(align_up(usize::MAX - 4094, Alignment::Align4096), None);
}

#[test]
fn test_align_up_near_max_no_overflow() {
    // usize::MAX - 2 cannot be aligned to 4 without overflow:
    // (usize::MAX - 2) + 3 = usize::MAX + 1, which overflows.
    assert_eq!(align_up(usize::MAX - 2, Alignment::Align4), None);
    // usize::MAX - 3 = ...11111100, already aligned to 4, so no addition needed beyond 0.
    assert_eq!(align_up(usize::MAX - 3, Alignment::Align4), Some(usize::MAX - 3));
}

#[test]
fn test_align_down_basic() {
    assert_eq!(align_down(5, Alignment::Align4), 4);
    assert_eq!(align_down(4, Alignment::Align4), 4);
    assert_eq!(align_down(3, Alignment::Align4), 0);
}

#[test]
fn test_is_aligned_basic() {
    assert!(is_aligned(4, Alignment::Align4));
    assert!(!is_aligned(5, Alignment::Align4));
    assert!(is_aligned(0, Alignment::Align4096));
}
