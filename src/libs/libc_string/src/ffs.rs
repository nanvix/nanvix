// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use ::sysapi::ffi::c_int;

//==================================================================================================
// Standalone Functions
//==================================================================================================

///
/// # Description
///
/// Finds the position of the first (least significant) set bit in `i`.
///
/// # Returns
///
/// The 1-based index of the first set bit, or 0 if `i` is zero.
///
/// # References
///
/// - <https://pubs.opengroup.org/onlinepubs/9799919799/functions/ffs.html>
///
#[cfg_attr(not(feature = "std"), unsafe(no_mangle))]
pub extern "C" fn ffs(i: c_int) -> c_int {
    if i == 0 {
        return 0;
    }
    // For a non-zero `i32`, `trailing_zeros()` ∈ 0..=31, so the 1-based index
    // (`+ 1`) is 1..=32 and always converts cleanly to `c_int` (the `Err` arm is
    // unreachable). Using `try_from` avoids an unchecked `as` cast, which this
    // crate forbids (clippy::cast_possible_truncation / cast_possible_wrap).
    match c_int::try_from(i.trailing_zeros()) {
        Ok(n) => n + 1,
        Err(_) => 0,
    }
}

#[cfg(all(test, feature = "std"))]
mod test {
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
}
