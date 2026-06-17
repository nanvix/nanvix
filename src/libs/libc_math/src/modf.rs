// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Public Functions
//==================================================================================================

/// Splits `x` into integer and fractional parts.
///
/// # Description
///
/// Stores the integer part in `*iptr` and returns the fractional part. Both parts
/// have the same sign as `x`.
///
/// # Parameters
///
/// - `x`: Input value.
/// - `iptr`: Pointer to store the integer part.
///
/// # Returns
///
/// The fractional part of `x`.
///
/// # Safety
///
/// The caller must ensure `iptr` points to a valid, writable `f64` location.
#[cfg_attr(not(feature = "std"), unsafe(no_mangle))]
pub unsafe extern "C" fn modf(x: f64, iptr: *mut f64) -> f64 {
    let t: f64 = crate::trunc::trunc(x);
    if !iptr.is_null() {
        unsafe { *iptr = t };
    }
    // Handle NaN and inf.
    if x.is_nan() {
        return x;
    }
    if x.to_bits() & 0x7FF0_0000_0000_0000 == 0x7FF0_0000_0000_0000 {
        // For infinity, fractional part is ±0.
        return f64::from_bits(x.to_bits() & 0x8000_0000_0000_0000);
    }
    x - t
}

//==================================================================================================
// Unit Tests
//==================================================================================================

#[cfg(all(test, feature = "std"))]
mod tests {
    use super::*;

    #[test]
    fn test_positive() {
        let mut i: f64 = 0.0;
        let f: f64 = unsafe { modf(3.75, &mut i) };
        assert!((i - 3.0).abs() < 1e-10);
        assert!((f - 0.75).abs() < 1e-10);
    }

    #[test]
    fn test_negative() {
        let mut i: f64 = 0.0;
        let f: f64 = unsafe { modf(-3.75, &mut i) };
        assert!((i - (-3.0)).abs() < 1e-10);
        assert!((f - (-0.75)).abs() < 1e-10);
    }

    #[test]
    fn test_integer() {
        let mut i: f64 = 0.0;
        let f: f64 = unsafe { modf(5.0, &mut i) };
        assert!((i - 5.0).abs() < 1e-10);
        assert!(f.abs() < 1e-10);
    }

    #[test]
    fn test_zero() {
        let mut i: f64 = 0.0;
        let f: f64 = unsafe { modf(0.0, &mut i) };
        assert_eq!(i, 0.0);
        assert_eq!(f, 0.0);
    }
}
