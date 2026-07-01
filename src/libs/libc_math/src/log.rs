// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Constants
//==================================================================================================

/// `ln(2)` split into a leading part (exact in its high bits) and a low correction.
const LN2_HI: f64 = 6.931_471_803_691_238e-1;
const LN2_LO: f64 = 1.908_214_929_270_587_7e-10;
/// `2^54`, used to scale subnormal inputs up into the normal range.
const TWO54: f64 = 1.801_439_850_948_198_4e16;
/// Minimax coefficients for the log approximation on the reduced range.
const LG1: f64 = 6.666_666_666_666_735e-1;
const LG2: f64 = 3.999_999_999_940_942e-1;
const LG3: f64 = 2.857_142_874_366_239e-1;
const LG4: f64 = 2.222_219_843_214_978_4e-1;
const LG5: f64 = 1.818_357_216_161_805e-1;
const LG6: f64 = 1.531_383_769_920_937_3e-1;
const LG7: f64 = 1.479_819_860_511_658_6e-1;

//==================================================================================================
// Public Functions
//==================================================================================================

/// Computes the natural logarithm of `x`.
///
/// # Description
///
/// Writes `x = 2^k * (1 + f)` with `f` in `[-1/3, 1/3]` and evaluates
/// `log(1 + f)` from an odd minimax polynomial in `s = f / (2 + f)`, then adds
/// `k * ln(2)` using a two-part `ln(2)` so no precision is lost for large `k`.
/// This is the fdlibm algorithm and is accurate to within one unit in the last
/// place.
///
/// # Parameters
///
/// - `x`: Input value (must be positive).
///
/// # Returns
///
/// The natural logarithm of `x`. Returns NaN for negative inputs and `-inf` for zero.
#[cfg_attr(not(feature = "std"), unsafe(no_mangle))]
#[allow(
    clippy::cast_possible_truncation,
    clippy::cast_possible_wrap,
    clippy::cast_sign_loss,
    clippy::eq_op
)]
pub extern "C" fn log(x: f64) -> f64 {
    let mut xr: f64 = x;
    let mut hx: i32 = (x.to_bits() >> 32) as i32; // signed high word
    let lx: u32 = x.to_bits() as u32; // low word

    let mut k: i32 = 0;
    if hx < 0x0010_0000 {
        // |x| < 2^-1022: zero, negative, or subnormal.
        if (hx & 0x7fff_ffff) == 0 && lx == 0 {
            return f64::NEG_INFINITY; // log(+/-0) = -inf
        }
        if hx < 0 {
            return (x - x) / 0.0; // log(negative) = NaN
        }
        // Subnormal: scale up by 2^54 and adjust the exponent.
        k -= 54;
        xr = x * TWO54;
        hx = (xr.to_bits() >> 32) as i32;
    }
    if hx >= 0x7ff0_0000 {
        return x + x; // log(+inf) = +inf, log(NaN) = NaN
    }
    k += (hx >> 20) - 1023;
    hx &= 0x000f_ffff;
    let i: i32 = (hx + 0x9_5f64) & 0x10_0000;
    // Normalize the significand to [1, 2) (i == 0) or [0.5, 1) (i != 0).
    let new_hi: u64 = u64::from((hx | (i ^ 0x3ff0_0000)) as u32);
    xr = f64::from_bits((new_hi << 32) | (xr.to_bits() & 0xffff_ffff));
    k += i >> 20;
    let f: f64 = xr - 1.0;

    // Path for f very close to zero, avoiding cancellation.
    if (0x000f_ffff & (2 + hx)) < 3 {
        if f == 0.0 {
            if k == 0 {
                return 0.0;
            }
            let dk: f64 = f64::from(k);
            return dk * LN2_HI + dk * LN2_LO;
        }
        let r: f64 = f * f * (0.5 - 0.333_333_333_333_333_3 * f);
        if k == 0 {
            return f - r;
        }
        let dk: f64 = f64::from(k);
        return dk * LN2_HI - ((r - dk * LN2_LO) - f);
    }

    let s: f64 = f / (2.0 + f);
    let dk: f64 = f64::from(k);
    let z: f64 = s * s;
    let i2: i32 = hx - 0x6_147a;
    let w: f64 = z * z;
    let j: i32 = 0x6_b851 - hx;
    let t1: f64 = w * (LG2 + w * (LG4 + w * LG6));
    let t2: f64 = z * (LG1 + w * (LG3 + w * (LG5 + w * LG7)));
    let r: f64 = t2 + t1;
    if (i2 | j) > 0 {
        let hfsq: f64 = 0.5 * f * f;
        if k == 0 {
            f - (hfsq - s * (hfsq + r))
        } else {
            dk * LN2_HI - ((hfsq - (s * (hfsq + r) + dk * LN2_LO)) - f)
        }
    } else if k == 0 {
        f - s * (f - r)
    } else {
        dk * LN2_HI - ((s * (f - r) - dk * LN2_LO) - f)
    }
}

//==================================================================================================
// Unit Tests
//==================================================================================================

#[cfg(all(test, feature = "std"))]
mod tests {
    use super::*;

    #[test]
    fn test_one() {
        assert!((log(1.0)).abs() < 1e-10);
    }

    #[test]
    fn test_e() {
        assert!((log(2.718_281_828_459_045) - 1.0).abs() < 1e-10);
    }

    #[test]
    fn test_two() {
        assert!((log(2.0) - core::f64::consts::LN_2).abs() < 1e-10);
    }

    #[test]
    fn test_ten() {
        assert!((log(10.0) - 2.302_585_092_994_046).abs() < 1e-10);
    }

    #[test]
    fn test_matches_std_over_range() {
        // Multiplicative sweep across the exponent range, compared to the host libm.
        let mut x: f64 = 1e-300;
        while x < 1e300 {
            let got: f64 = log(x);
            let want: f64 = x.ln();
            assert!(
                (got - want).abs() <= want.abs() * 1e-14 + 1e-15,
                "log({x}) = {got}, want {want}"
            );
            x *= 1.037;
        }
    }

    #[test]
    fn test_subnormal() {
        let d: f64 = f64::from_bits(0x0000_0000_0000_1000); // subnormal
        assert!((log(d) - d.ln()).abs() <= d.ln().abs() * 1e-14);
    }

    #[test]
    fn test_zero() {
        assert_eq!(log(0.0), f64::NEG_INFINITY);
    }

    #[test]
    fn test_negative() {
        assert!(log(-1.0).is_nan());
    }

    #[test]
    fn test_infinity() {
        assert_eq!(log(f64::INFINITY), f64::INFINITY);
    }

    #[test]
    fn test_nan() {
        assert!(log(f64::NAN).is_nan());
    }
}
