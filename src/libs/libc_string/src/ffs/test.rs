// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

use super::ffs;
use ::sysapi::ffi::c_int;

#[test]
fn test_ffs_zero() {
    assert_eq!(ffs(0), 0, "ffs(0) should be 0");
}

#[test]
fn test_ffs_low_bit() {
    assert_eq!(ffs(1), 1, "ffs(1) should be 1");
}

#[test]
fn test_ffs_second_bit() {
    assert_eq!(ffs(2), 2, "ffs(2) should be 2");
}

#[test]
fn test_ffs_mixed_bits() {
    // 0b1100 has its least-significant set bit at position 3 (1-based).
    assert_eq!(ffs(12), 3, "ffs(12) should be 3");
}

#[test]
fn test_ffs_high_bit() {
    // 0x80 has its least-significant set bit at position 8 (1-based).
    assert_eq!(ffs(0x80), 8, "ffs(0x80) should be 8");
}

#[test]
fn test_ffs_sign_bit() {
    // i32::MIN has only bit 31 set, so the 1-based index is 32.
    assert_eq!(ffs(c_int::MIN), 32, "ffs(i32::MIN) should be 32");
}
