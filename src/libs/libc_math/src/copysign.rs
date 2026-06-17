// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Public Functions
//==================================================================================================

/// Returns a value with the magnitude of `x` and the sign of `y`.
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
pub extern "C" fn copysign(x: f64, y: f64) -> f64 {
    let mag: u64 = x.to_bits() & 0x7FFF_FFFF_FFFF_FFFF;
    let sign: u64 = y.to_bits() & 0x8000_0000_0000_0000;
    f64::from_bits(mag | sign)
}

//==================================================================================================
// Unit Tests
//==================================================================================================

#[cfg(all(test, feature = "std"))]
mod tests {
    use super::*;

    #[test]
    fn test_positive_to_negative() {
        assert!((copysign(3.0, -1.0) - (-3.0)).abs() < 1e-10);
    }

    #[test]
    fn test_negative_to_positive() {
        assert!((copysign(-3.0, 1.0) - 3.0).abs() < 1e-10);
    }

    #[test]
    fn test_same_sign() {
        assert!((copysign(3.0, 1.0) - 3.0).abs() < 1e-10);
    }

    #[test]
    fn test_zero() {
        assert_eq!(copysign(0.0, -1.0).to_bits(), (-0.0_f64).to_bits());
    }
}
