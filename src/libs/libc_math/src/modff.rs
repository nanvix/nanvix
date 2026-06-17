// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Public Functions
//==================================================================================================

/// Splits `x` into integer and fractional parts (single-precision).
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
/// The caller must ensure `iptr` points to a valid, writable `f32` location.
#[cfg_attr(not(feature = "std"), unsafe(no_mangle))]
pub unsafe extern "C" fn modff(x: f32, iptr: *mut f32) -> f32 {
    let t: f32 = crate::truncf::truncf(x);
    if !iptr.is_null() {
        unsafe { *iptr = t };
    }
    if x.is_nan() {
        return x;
    }
    if x.to_bits() & 0x7F80_0000 == 0x7F80_0000 {
        return f32::from_bits(x.to_bits() & 0x8000_0000);
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
        let mut i: f32 = 0.0;
        let f: f32 = unsafe { modff(3.75, &mut i) };
        assert!((i - 3.0).abs() < 1e-5);
        assert!((f - 0.75).abs() < 1e-5);
    }

    #[test]
    fn test_negative() {
        let mut i: f32 = 0.0;
        let f: f32 = unsafe { modff(-3.75, &mut i) };
        assert!((i - (-3.0)).abs() < 1e-5);
        assert!((f - (-0.75)).abs() < 1e-5);
    }

    #[test]
    fn test_zero() {
        let mut i: f32 = 0.0;
        let f: f32 = unsafe { modff(0.0, &mut i) };
        assert_eq!(i, 0.0);
        assert_eq!(f, 0.0);
    }
}
