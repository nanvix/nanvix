// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

/// Computes the hyperbolic tangent of `x`.
///
/// # Description
///
/// Evaluates `tanh` from `expm1` so precision is preserved near zero, and
/// preserves the sign of a zero argument (`tanh(-0.0) == -0.0`). This is the musl
/// algorithm and is accurate to within one unit in the last place.
///
/// # Parameters
///
/// - `x`: Input value.
///
/// # Returns
///
/// The value `tanh(x)`, saturating to `+/-1` for large magnitudes.
#[cfg_attr(not(feature = "std"), unsafe(no_mangle))]
#[allow(clippy::cast_possible_truncation)]
pub extern "C" fn tanh(x: f64) -> f64 {
    let sign: bool = (x.to_bits() >> 63) != 0;
    let ax: f64 = f64::from_bits(x.to_bits() & 0x7fff_ffff_ffff_ffff); // |x|
    let w: u32 = (ax.to_bits() >> 32) as u32;

    let t: f64;
    if w > 0x3fe1_93ea {
        // |x| > log(3)/2 ~= 0.5493, or NaN.
        if w > 0x4034_0000 {
            // |x| > 20, or NaN: tanh saturates (and propagates NaN).
            t = 1.0 - 0.0 / ax;
        } else {
            let e: f64 = crate::expm1::expm1(2.0 * ax);
            t = 1.0 - 2.0 / (e + 2.0);
        }
    } else if w > 0x3fd0_58ae {
        // |x| > log(5/3)/2 ~= 0.2554
        let e: f64 = crate::expm1::expm1(2.0 * ax);
        t = e / (e + 2.0);
    } else if w >= 0x0010_0000 {
        // 2^-1022 <= |x| <= 0.2554: use the -2x form to preserve precision.
        let e: f64 = crate::expm1::expm1(-2.0 * ax);
        t = -e / (e + 2.0);
    } else {
        // |x| is zero or subnormal: tanh(x) rounds to x.
        t = ax;
    }
    if sign {
        -t
    } else {
        t
    }
}

//==================================================================================================
// Unit Tests
//==================================================================================================

#[cfg(all(test, feature = "std"))]
mod tests {
    use super::*;

    #[test]
    fn test_zero() {
        assert!((tanh(0.0)).abs() < 1e-10);
    }

    #[test]
    fn test_one() {
        assert!((tanh(1.0) - 0.761_594_155_955_764_9).abs() < 1e-10);
    }

    #[test]
    fn test_odd_symmetry() {
        assert!((tanh(-1.0) + 0.761_594_155_955_764_9).abs() < 1e-10);
    }

    #[test]
    fn test_saturation() {
        assert_eq!(tanh(30.0), 1.0);
        assert_eq!(tanh(-30.0), -1.0);
    }

    #[test]
    fn test_nan() {
        assert!(tanh(f64::NAN).is_nan());
    }

    #[test]
    fn test_signed_zero() {
        // tanh must preserve the sign of zero.
        assert_eq!(tanh(0.0).to_bits(), 0.0_f64.to_bits());
        assert_eq!(tanh(-0.0).to_bits(), (-0.0_f64).to_bits());
    }

    #[test]
    fn test_matches_std_over_range() {
        let mut x: f64 = -25.0;
        while x <= 25.0 {
            let got: f64 = tanh(x);
            let want: f64 = x.tanh();
            assert!(
                (got - want).abs() <= want.abs() * 1e-14 + 1e-16,
                "tanh({x}) = {got}, want {want}"
            );
            x += 0.011;
        }
    }
}
