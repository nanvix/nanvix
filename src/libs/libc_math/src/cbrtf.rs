// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Public Functions
//==================================================================================================

/// Computes the cube root of `x` (single-precision).
///
/// # Parameters
///
/// - `x`: Input value.
///
/// # Returns
///
/// The cube root of `x`.
#[cfg_attr(not(feature = "std"), unsafe(no_mangle))]
pub extern "C" fn cbrtf(x: f32) -> f32 {
    if x.is_nan() || x == 0.0 {
        return x;
    }
    if x.to_bits() & 0x7FFF_FFFF == 0x7F80_0000 {
        return x;
    }

    let abs_x: f32 = f32::from_bits(x.to_bits() & 0x7FFF_FFFF);
    let sign: f32 = if x < 0.0 { -1.0 } else { 1.0 };

    let bits: u32 = abs_x.to_bits();
    let guess_bits: u32 = bits / 3 + 0x2A50_0000;
    let mut guess: f32 = f32::from_bits(guess_bits);

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
    fn test_perfect_cube() {
        assert!((cbrtf(8.0) - 2.0).abs() < 1e-5);
    }

    #[test]
    fn test_negative() {
        assert!((cbrtf(-8.0) - (-2.0)).abs() < 1e-5);
    }

    #[test]
    fn test_zero() {
        assert_eq!(cbrtf(0.0), 0.0);
    }
}
