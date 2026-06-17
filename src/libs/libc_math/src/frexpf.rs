// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

use sysapi::ffi::c_int;

//==================================================================================================
// Public Functions
//==================================================================================================

/// Extracts the normalized fraction and exponent from `x` (single-precision).
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
pub unsafe extern "C" fn frexpf(x: f32, exp: *mut c_int) -> f32 {
    if x == 0.0 {
        if !exp.is_null() {
            unsafe { *exp = 0 };
        }
        return x;
    }
    if x.is_nan() || x.to_bits() & 0x7F80_0000 == 0x7F80_0000 {
        if !exp.is_null() {
            unsafe { *exp = 0 };
        }
        return x;
    }

    let mut bits: u32 = x.to_bits();
    let mut exp_raw: u32 = (bits >> 23) & 0xFF;

    let mut adj: i64 = 0;
    if exp_raw == 0 {
        let scaled: f32 = x * 8_388_608.0; // 2^23
        bits = scaled.to_bits();
        exp_raw = (bits >> 23) & 0xFF;
        adj = -23;
    }

    let e: i64 = i64::from(exp_raw) - 126 + adj;

    if !exp.is_null() {
        #[allow(clippy::cast_possible_truncation)]
        let e_int: c_int = e as c_int;
        unsafe { *exp = e_int };
    }

    let frac_bits: u32 = (bits & 0x807F_FFFF) | 0x3F00_0000;
    f32::from_bits(frac_bits)
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
        let f: f32 = unsafe { frexpf(8.0, &mut e) };
        assert!((f - 0.5).abs() < 1e-5);
        assert_eq!(e, 4);
    }

    #[test]
    fn test_zero() {
        let mut e: c_int = 0;
        let f: f32 = unsafe { frexpf(0.0, &mut e) };
        assert_eq!(f, 0.0);
        assert_eq!(e, 0);
    }

    #[test]
    fn test_one() {
        let mut e: c_int = 0;
        let f: f32 = unsafe { frexpf(1.0, &mut e) };
        assert!((f - 0.5).abs() < 1e-5);
        assert_eq!(e, 1);
    }
}
