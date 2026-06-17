// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

/// Computes the next representable value after `x` in the direction of `y`.
///
/// # Parameters
///
/// - `x`: Starting value.
/// - `y`: Direction value.
///
/// # Returns
///
/// The next representable double after `x` toward `y`.
#[cfg_attr(not(feature = "std"), unsafe(no_mangle))]
pub extern "C" fn nextafter(x: f64, y: f64) -> f64 {
    if x.is_nan() || y.is_nan() {
        return f64::NAN;
    }
    if x == y {
        return y;
    }
    if x == 0.0 {
        // Smallest subnormal in the direction of y.
        let tiny: f64 = f64::from_bits(1);
        return if y > 0.0 { tiny } else { -tiny };
    }

    let bits: u64 = x.to_bits();
    // Moving away from zero increases the magnitude bits; toward zero decreases.
    let away_from_zero: bool = (y > x) == (x > 0.0);
    let next: u64 = if away_from_zero { bits + 1 } else { bits - 1 };
    f64::from_bits(next)
}

//==================================================================================================
// Unit Tests
//==================================================================================================

#[cfg(all(test, feature = "std"))]
mod tests {
    use super::*;

    #[test]
    fn test_toward_larger() {
        assert_eq!(nextafter(1.0, 2.0), 1.0 + f64::EPSILON);
    }

    #[test]
    fn test_toward_smaller() {
        assert!(nextafter(1.0, 0.0) < 1.0);
        assert_eq!(nextafter(1.0, 0.0), 1.0 - f64::EPSILON / 2.0);
    }

    #[test]
    fn test_equal() {
        assert_eq!(nextafter(3.5, 3.5), 3.5);
    }

    #[test]
    fn test_from_zero() {
        assert_eq!(nextafter(0.0, 1.0), f64::from_bits(1));
        assert_eq!(nextafter(0.0, -1.0), -f64::from_bits(1));
    }

    #[test]
    fn test_nan() {
        assert!(nextafter(f64::NAN, 1.0).is_nan());
        assert!(nextafter(1.0, f64::NAN).is_nan());
    }
}
