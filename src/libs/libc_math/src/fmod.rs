// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Public Functions
//==================================================================================================

/// Computes the floating-point remainder of `x / y`.
///
/// # Description
///
/// The result `r` has the same sign as `x` and magnitude less than that of `y`,
/// and satisfies `x == n*y + r` for some integer `n`. The computation is exact:
/// it is performed on the integer significands so no rounding error is
/// introduced.
///
/// Special values follow IEEE-754 / C99: `fmod(±0, y) = ±0` for `y != 0`,
/// `fmod(x, ±inf) = x` for finite `x`, and `fmod(x, 0)`, `fmod(±inf, y)` and any
/// NaN operand yield NaN.
///
/// # Parameters
///
/// - `x`: Dividend.
/// - `y`: Divisor.
///
/// # Returns
///
/// The floating-point remainder of `x / y`.
#[cfg_attr(not(feature = "std"), unsafe(no_mangle))]
#[allow(
    clippy::cast_possible_truncation,
    clippy::cast_possible_wrap,
    clippy::eq_op
)]
pub extern "C" fn fmod(x: f64, y: f64) -> f64 {
    let mut uxi: u64 = x.to_bits();
    let mut uyi: u64 = y.to_bits();
    let mut ex: i32 = ((uxi >> 52) & 0x7ff) as i32;
    let mut ey: i32 = ((uyi >> 52) & 0x7ff) as i32;
    let sx: u64 = uxi >> 63;

    // y == 0, or x is non-finite (inf/NaN), or y is NaN: the result is NaN.
    // `(x*y)/(x*y)` yields a quiet NaN and raises the invalid flag, matching libm.
    if (uyi << 1) == 0 || y.is_nan() || ex == 0x7ff {
        return (x * y) / (x * y);
    }
    // |x| <= |y|: either |x| < |y| (result is x, covering fmod(x, inf) = x and
    // fmod(±0, y) = ±0) or |x| == |y| (result is a zero with the sign of x).
    if (uxi << 1) <= (uyi << 1) {
        if (uxi << 1) == (uyi << 1) {
            return 0.0 * x;
        }
        return x;
    }

    // Normalize x into a 53-bit significand with the leading bit at position 52,
    // tracking the (possibly subnormal) exponent in `ex`.
    if ex == 0 {
        let mut i: u64 = uxi << 12;
        while (i >> 63) == 0 {
            ex -= 1;
            i <<= 1;
        }
        uxi <<= (1 - ex) as u32;
    } else {
        uxi &= u64::MAX >> 12;
        uxi |= 1 << 52;
    }
    // Normalize y likewise.
    if ey == 0 {
        let mut i: u64 = uyi << 12;
        while (i >> 63) == 0 {
            ey -= 1;
            i <<= 1;
        }
        uyi <<= (1 - ey) as u32;
    } else {
        uyi &= u64::MAX >> 12;
        uyi |= 1 << 52;
    }

    // Compute the remainder of the significands via shift-and-subtract long division.
    while ex > ey {
        let i: u64 = uxi.wrapping_sub(uyi);
        if (i >> 63) == 0 {
            if i == 0 {
                return 0.0 * x;
            }
            uxi = i;
        }
        uxi <<= 1;
        ex -= 1;
    }
    let i: u64 = uxi.wrapping_sub(uyi);
    if (i >> 63) == 0 {
        if i == 0 {
            return 0.0 * x;
        }
        uxi = i;
    }
    while (uxi >> 52) == 0 {
        uxi <<= 1;
        ex -= 1;
    }

    // Reassemble the floating-point result, restoring the sign of x.
    if ex > 0 {
        uxi -= 1 << 52;
        uxi |= (ex as u64) << 52;
    } else {
        uxi >>= (1 - ex) as u32;
    }
    uxi |= sx << 63;
    f64::from_bits(uxi)
}

//==================================================================================================
// Unit Tests
//==================================================================================================

#[cfg(all(test, feature = "std"))]
mod tests {
    use super::*;

    #[test]
    fn test_basic() {
        assert!((fmod(5.3, 2.0) - 1.3).abs() < 1e-10);
    }

    #[test]
    fn test_negative() {
        assert!((fmod(-5.3, 2.0) - (-1.3)).abs() < 1e-10);
    }

    #[test]
    fn test_exact() {
        assert!((fmod(6.0, 3.0)).abs() < 1e-10);
    }

    #[test]
    fn test_zero_divisor() {
        assert!(fmod(5.0, 0.0).is_nan());
    }

    #[test]
    fn test_nan() {
        assert!(fmod(f64::NAN, 2.0).is_nan());
        assert!(fmod(2.0, f64::NAN).is_nan());
    }

    #[test]
    fn test_infinite_divisor_returns_x() {
        // fmod(x, ±inf) == x for finite x.
        assert_eq!(fmod(2.0, f64::INFINITY), 2.0);
        assert_eq!(fmod(2.0, f64::NEG_INFINITY), 2.0);
        assert_eq!(fmod(-3.5, f64::INFINITY), -3.5);
        assert_eq!(fmod(1e300, f64::INFINITY), 1e300);
    }

    #[test]
    fn test_infinite_dividend_is_nan() {
        assert!(fmod(f64::INFINITY, 2.0).is_nan());
        assert!(fmod(f64::NEG_INFINITY, 2.0).is_nan());
    }

    #[test]
    fn test_signed_zero_dividend() {
        // fmod(±0, y) == ±0, preserving the sign of zero.
        assert_eq!(fmod(0.0, 3.0).to_bits(), 0.0_f64.to_bits());
        assert_eq!(fmod(-0.0, 3.0).to_bits(), (-0.0_f64).to_bits());
    }

    #[test]
    fn test_exact_matches_rem_operator() {
        // Rust's `%` on f64 is the C `fmod`; the result must be bit-identical.
        let xs: [f64; 9] = [5.3, -5.3, 6.0, 1e300, -1e-300, 12345.678, -0.1, 3.0, 2.5];
        let ys: [f64; 7] = [2.0, 3.0, 0.5, 1e-100, 7.0, -2.0, 1.25];
        for &x in &xs {
            for &y in &ys {
                assert_eq!(fmod(x, y).to_bits(), (x % y).to_bits(), "fmod({x}, {y}) mismatch");
            }
        }
    }

    #[test]
    fn test_subnormal_operands() {
        let tiny: f64 = f64::from_bits(0x0000_0000_0000_0007); // subnormal
        assert_eq!(fmod(tiny, tiny).to_bits(), 0.0_f64.to_bits());
        assert_eq!(fmod(1.0, tiny).to_bits(), (1.0_f64 % tiny).to_bits());
    }
}
