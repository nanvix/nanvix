// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

use sysapi::ffi::c_int;

//==================================================================================================
// Public Functions
//==================================================================================================

/// Extracts the normalized fraction and exponent from `x`.
///
/// # Description
///
/// Decomposes `x` into a fraction `f` in `[0.5, 1)` and an integer exponent `e`
/// such that `x = f * 2^e`.
///
/// # Parameters
///
/// - `x`: Input value.
/// - `exp`: Pointer to store the exponent.
///
/// # Returns
///
/// The normalized fraction. The exponent is stored in `*exp`.
///
/// # Safety
///
/// The caller must ensure `exp` points to a valid, writable `c_int` location.
#[cfg_attr(not(feature = "std"), unsafe(no_mangle))]
pub unsafe extern "C" fn frexp(x: f64, exp: *mut c_int) -> f64 {
    // Handle special cases.
    if x == 0.0 {
        if !exp.is_null() {
            unsafe { *exp = 0 };
        }
        return x;
    }
    if x.is_nan() || x.to_bits() & 0x7FF0_0000_0000_0000 == 0x7FF0_0000_0000_0000 {
        if !exp.is_null() {
            unsafe { *exp = 0 };
        }
        return x;
    }

    let mut bits: u64 = x.to_bits();
    let mut exp_raw: u64 = (bits >> 52) & 0x7FF;

    // Handle subnormals.
    let mut adj: i64 = 0;
    if exp_raw == 0 {
        let scaled: f64 = x * 4_503_599_627_370_496.0; // 2^52
        bits = scaled.to_bits();
        exp_raw = (bits >> 52) & 0x7FF;
        adj = -52;
    }

    #[allow(clippy::cast_possible_wrap)]
    let e: i64 = (exp_raw as i64) - 1022 + adj;

    if !exp.is_null() {
        #[allow(clippy::cast_possible_truncation)]
        let e_int: c_int = e as c_int;
        unsafe { *exp = e_int };
    }

    // Construct fraction with exponent -1 (biased: 1022), in [0.5, 1).
    let frac_bits: u64 = (bits & 0x800F_FFFF_FFFF_FFFF) | 0x3FE0_0000_0000_0000;
    f64::from_bits(frac_bits)
}

//==================================================================================================
// Unit Tests
//==================================================================================================

#[cfg(all(test, feature = "std"))]
mod tests {
    use super::*;

    #[test]
    fn test_basic() {
        let mut e: c_int = 0;
        let f: f64 = unsafe { frexp(8.0, &mut e) };
        assert!((f - 0.5).abs() < 1e-10);
        assert_eq!(e, 4);
    }

    #[test]
    fn test_fraction() {
        let mut e: c_int = 0;
        let f: f64 = unsafe { frexp(0.75, &mut e) };
        assert!((f - 0.75).abs() < 1e-10);
        assert_eq!(e, 0);
    }

    #[test]
    fn test_one() {
        let mut e: c_int = 0;
        let f: f64 = unsafe { frexp(1.0, &mut e) };
        assert!((f - 0.5).abs() < 1e-10);
        assert_eq!(e, 1);
    }

    #[test]
    fn test_zero() {
        let mut e: c_int = 0;
        let f: f64 = unsafe { frexp(0.0, &mut e) };
        assert_eq!(f, 0.0);
        assert_eq!(e, 0);
    }

    #[test]
    fn test_negative() {
        let mut e: c_int = 0;
        let f: f64 = unsafe { frexp(-4.0, &mut e) };
        assert!((f - (-0.5)).abs() < 1e-10);
        assert_eq!(e, 3);
    }
}
