// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Public Functions
//==================================================================================================

/// Computes the hypotenuse: `sqrt(x^2 + y^2)` without undue overflow.
///
/// # Parameters
///
/// - `x`: First value.
/// - `y`: Second value.
///
/// # Returns
///
/// The Euclidean distance `sqrt(x^2 + y^2)`.
#[cfg_attr(not(feature = "std"), unsafe(no_mangle))]
pub extern "C" fn hypot(x: f64, y: f64) -> f64 {
    // Handle special cases.
    let ax: f64 = f64::from_bits(x.to_bits() & 0x7FFF_FFFF_FFFF_FFFF);
    let ay: f64 = f64::from_bits(y.to_bits() & 0x7FFF_FFFF_FFFF_FFFF);

    // If either is inf, result is inf (even if the other is NaN).
    if ax.to_bits() == 0x7FF0_0000_0000_0000 || ay.to_bits() == 0x7FF0_0000_0000_0000 {
        return f64::from_bits(0x7FF0_0000_0000_0000);
    }
    if x.is_nan() || y.is_nan() {
        return f64::from_bits(0x7FF8_0000_0000_0000);
    }

    // Use the larger magnitude to avoid overflow.
    let (big, small) = if ax >= ay { (ax, ay) } else { (ay, ax) };

    if big == 0.0 {
        return 0.0;
    }

    let ratio: f64 = small / big;
    big * crate::sqrt::sqrt(1.0 + ratio * ratio)
}

//==================================================================================================
// Unit Tests
//==================================================================================================

#[cfg(all(test, feature = "std"))]
mod tests {
    use super::*;

    #[test]
    fn test_345() {
        assert!((hypot(3.0, 4.0) - 5.0).abs() < 1e-10);
    }

    #[test]
    fn test_zero() {
        assert!((hypot(0.0, 5.0) - 5.0).abs() < 1e-10);
        assert!((hypot(3.0, 0.0) - 3.0).abs() < 1e-10);
    }

    #[test]
    fn test_inf() {
        assert_eq!(hypot(f64::INFINITY, 1.0), f64::INFINITY);
        assert_eq!(hypot(1.0, f64::INFINITY), f64::INFINITY);
    }

    #[test]
    fn test_nan() {
        assert!(hypot(f64::NAN, 1.0).is_nan());
    }
}
