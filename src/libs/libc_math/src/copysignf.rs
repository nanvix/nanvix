// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Public Functions
//==================================================================================================

/// Returns a value with the magnitude of `x` and the sign of `y` (single-precision).
///
/// # Parameters
///
/// - `x`: Value providing magnitude.
/// - `y`: Value providing sign.
///
/// # Returns
///
/// A value with `|x|` and the sign of `y`.
#[cfg_attr(not(feature = "std"), unsafe(no_mangle))]
pub extern "C" fn copysignf(x: f32, y: f32) -> f32 {
    let mag: u32 = x.to_bits() & 0x7FFF_FFFF;
    let sign: u32 = y.to_bits() & 0x8000_0000;
    f32::from_bits(mag | sign)
}

//==================================================================================================
// Unit Tests
//==================================================================================================

#[cfg(all(test, feature = "std"))]
mod tests {
    use super::*;

    #[test]
    fn test_positive_to_negative() {
        assert!((copysignf(3.0, -1.0) - (-3.0)).abs() < 1e-5);
    }

    #[test]
    fn test_negative_to_positive() {
        assert!((copysignf(-3.0, 1.0) - 3.0).abs() < 1e-5);
    }
}
