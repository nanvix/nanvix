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
mod tests;
