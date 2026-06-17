// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Public Functions
//==================================================================================================

/// Computes the hypotenuse: `sqrt(x^2 + y^2)` without undue overflow (single-precision).
///
/// # Parameters
///
/// - `x`: First value.
/// - `y`: Second value.
///
/// # Returns
///
/// The Euclidean distance.
#[cfg_attr(not(feature = "std"), unsafe(no_mangle))]
pub extern "C" fn hypotf(x: f32, y: f32) -> f32 {
    let ax: f32 = f32::from_bits(x.to_bits() & 0x7FFF_FFFF);
    let ay: f32 = f32::from_bits(y.to_bits() & 0x7FFF_FFFF);

    if ax.to_bits() == 0x7F80_0000 || ay.to_bits() == 0x7F80_0000 {
        return f32::from_bits(0x7F80_0000);
    }
    if x.is_nan() || y.is_nan() {
        return f32::from_bits(0x7FC0_0000);
    }

    let (big, small) = if ax >= ay { (ax, ay) } else { (ay, ax) };
    if big == 0.0 {
        return 0.0;
    }

    let ratio: f32 = small / big;
    big * crate::sqrtf::sqrtf(1.0 + ratio * ratio)
}

//==================================================================================================
// Unit Tests
//==================================================================================================

#[cfg(all(test, feature = "std"))]
mod tests {
    use super::*;

    #[test]
    fn test_345() {
        assert!((hypotf(3.0, 4.0) - 5.0).abs() < 1e-4);
    }

    #[test]
    fn test_inf() {
        assert_eq!(hypotf(f32::INFINITY, 1.0), f32::INFINITY);
    }
}
