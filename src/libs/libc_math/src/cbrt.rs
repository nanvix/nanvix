// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Public Functions
//==================================================================================================

/// Computes the cube root of `x`.
///
/// # Description
///
/// Uses Halley's method with an initial guess from IEEE 754 bit manipulation.
///
/// # Parameters
///
/// - `x`: Input value.
///
/// # Returns
///
/// The cube root of `x`.
#[cfg_attr(not(feature = "std"), unsafe(no_mangle))]
pub extern "C" fn cbrt(x: f64) -> f64 {
    if x.is_nan() || x == 0.0 {
        return x;
    }
    if x.to_bits() & 0x7FFF_FFFF_FFFF_FFFF == 0x7FF0_0000_0000_0000 {
        return x;
    }

    let abs_x: f64 = f64::from_bits(x.to_bits() & 0x7FFF_FFFF_FFFF_FFFF);
    let sign: f64 = if x < 0.0 { -1.0 } else { 1.0 };

    // Initial guess via bit manipulation: cbrt(x) ≈ 2^(e/3) * m^(1/3).
    let bits: u64 = abs_x.to_bits();
    let guess_bits: u64 = bits / 3 + 0x2AA0_0000_0000_0000;
    let mut guess: f64 = f64::from_bits(guess_bits);

    // Newton's method: g_{n+1} = (2*g + x/g^2) / 3.
    let mut i: u32 = 0;
    while i < 8 {
        guess = (2.0 * guess + abs_x / (guess * guess)) / 3.0;
        i += 1;
    }

    sign * guess
}

//==================================================================================================
// Unit Tests
//==================================================================================================

#[cfg(all(test, feature = "std"))]
mod tests {
    use super::*;

    #[test]
    fn test_perfect_cubes() {
        assert!((cbrt(8.0) - 2.0).abs() < 1e-10);
        assert!((cbrt(27.0) - 3.0).abs() < 1e-10);
    }

    #[test]
    fn test_negative() {
        assert!((cbrt(-8.0) - (-2.0)).abs() < 1e-10);
    }

    #[test]
    fn test_one() {
        assert!((cbrt(1.0) - 1.0).abs() < 1e-10);
    }

    #[test]
    fn test_zero() {
        assert_eq!(cbrt(0.0), 0.0);
    }

    #[test]
    fn test_nan() {
        assert!(cbrt(f64::NAN).is_nan());
    }
}
