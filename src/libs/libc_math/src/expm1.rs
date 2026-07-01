// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Constants
//==================================================================================================

/// Overflow threshold: `expm1(x)` overflows to `+inf` for `x > O_THRESHOLD`.
const O_THRESHOLD: f64 = 7.097_827_128_933_84e2;
/// `ln(2)` split into a leading part (exact in its high bits) and a low correction.
const LN2_HI: f64 = 6.931_471_803_691_238e-1;
const LN2_LO: f64 = 1.908_214_929_270_587_7e-10;
/// `1 / ln(2)`.
const INVLN2: f64 = 1.442_695_040_888_963_4;
/// Minimax coefficients for the auxiliary rational used by the reduction.
const Q1: f64 = -3.333_333_333_333_313e-2;
const Q2: f64 = 1.587_301_587_254_814_6e-3;
const Q3: f64 = -7.936_507_578_674_88e-5;
const Q4: f64 = 4.008_217_827_329_362e-6;
const Q5: f64 = -2.010_992_181_836_243_7e-7;

//==================================================================================================
// Public Functions
//==================================================================================================

/// Computes `e^x - 1`.
///
/// # Description
///
/// Computes `e^x - 1` directly (rather than as `exp(x) - 1`) so that no
/// significant digits are lost to cancellation when `x` is near zero. This is the
/// fdlibm algorithm and is accurate to within one unit in the last place.
///
/// # Parameters
///
/// - `x`: Input value.
///
/// # Returns
///
/// The value `expm1(x) = e^x - 1`. Overflow saturates to `+inf`.
#[cfg_attr(not(feature = "std"), unsafe(no_mangle))]
#[allow(clippy::cast_possible_truncation)]
pub extern "C" fn expm1(x: f64) -> f64 {
    let hx_full: u32 = (x.to_bits() >> 32) as u32;
    let xsb: u32 = hx_full & 0x8000_0000; // sign bit of x
    let hx: u32 = hx_full & 0x7fff_ffff; // high word of |x|

    let k: i32;
    let c: f64;
    let xr: f64;

    // Filter out huge and non-finite arguments.
    if hx >= 0x4043_687A {
        // |x| >= 56*ln2
        if hx >= 0x4086_2E42 {
            // |x| >= 709.78...
            if hx >= 0x7ff0_0000 {
                if x.is_nan() {
                    return x + x;
                }
                return if xsb == 0 { x } else { -1.0 }; // expm1(+inf)=inf, expm1(-inf)=-1
            }
            if x > O_THRESHOLD {
                return f64::INFINITY; // overflow
            }
        }
        if xsb != 0 {
            // x < -56*ln2: expm1(x) rounds to -1.
            return -1.0;
        }
    }

    // Argument reduction: x = k*ln(2) + (hi - lo), with c the reduction round-off.
    if hx > 0x3fd6_2e42 {
        // |x| > 0.5*ln2
        let hi: f64;
        let lo: f64;
        if hx < 0x3FF0_A2B2 {
            // |x| < 1.5*ln2: a single step of ln2 suffices.
            if xsb == 0 {
                hi = x - LN2_HI;
                lo = LN2_LO;
                k = 1;
            } else {
                hi = x + LN2_HI;
                lo = -LN2_LO;
                k = -1;
            }
        } else {
            k = (INVLN2 * x + if xsb == 0 { 0.5 } else { -0.5 }) as i32;
            let t: f64 = f64::from(k);
            hi = x - t * LN2_HI; // t*LN2_HI is exact here
            lo = t * LN2_LO;
        }
        xr = hi - lo;
        c = (hi - xr) - lo;
    } else if hx < 0x3c90_0000 {
        // |x| < 2^-54: expm1(x) rounds to x (preserving a signed zero).
        return x;
    } else {
        k = 0;
        xr = x;
        c = 0.0;
    }

    // x is now in the primary range.
    let hfx: f64 = 0.5 * xr;
    let hxs: f64 = xr * hfx;
    let r1: f64 = 1.0 + hxs * (Q1 + hxs * (Q2 + hxs * (Q3 + hxs * (Q4 + hxs * Q5))));
    let t: f64 = 3.0 - r1 * hfx;
    let mut e: f64 = hxs * ((r1 - t) / (6.0 - xr * t));
    if k == 0 {
        return xr - (xr * e - hxs);
    }
    let twopk: f64 = f64::from_bits(((1023 + k) as u64) << 52); // 2^k
    e = xr * (e - c) - c;
    e -= hxs;
    if k == -1 {
        return 0.5 * (xr - e) - 0.5;
    }
    if k == 1 {
        if xr < -0.25 {
            return -2.0 * (e - (xr + 0.5));
        }
        return 1.0 + 2.0 * (xr - e);
    }
    if k <= -2 || k > 56 {
        // exp(x) - 1 is accurate enough here.
        let mut y: f64 = 1.0 - (e - xr);
        if k == 1024 {
            y = y * 2.0 * f64::from_bits(0x7fe0_0000_0000_0000); // 2^1023
        } else {
            y *= twopk;
        }
        return y - 1.0;
    }
    if k < 20 {
        let t2: f64 = 1.0 - f64::from_bits(((1023 - k) as u64) << 52); // 1 - 2^-k (exact)
        let y: f64 = t2 - (e - xr);
        y * twopk
    } else {
        let t2: f64 = f64::from_bits(((1023 - k) as u64) << 52); // 2^-k
        let mut y: f64 = xr - (e + t2);
        y += 1.0;
        y * twopk
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
        assert_eq!(expm1(0.0), 0.0);
    }

    #[test]
    fn test_one() {
        assert!((expm1(1.0) - 1.718_281_828_459_045).abs() < 1e-10);
    }

    #[test]
    fn test_negative() {
        assert!((expm1(-1.0) + 0.632_120_558_828_557_7).abs() < 1e-10);
    }

    #[test]
    fn test_nan() {
        assert!(expm1(f64::NAN).is_nan());
    }

    #[test]
    fn test_signed_zero() {
        assert_eq!(expm1(0.0).to_bits(), 0.0_f64.to_bits());
        assert_eq!(expm1(-0.0).to_bits(), (-0.0_f64).to_bits());
    }

    #[test]
    fn test_small_precision() {
        // Near zero, expm1(x) must not lose precision the way exp(x) - 1 does.
        for &x in &[1e-8_f64, -1e-8, 1e-5, -1e-5, 1e-12] {
            let want: f64 = x.exp_m1();
            assert!((expm1(x) - want).abs() <= want.abs() * 1e-14 + 1e-300);
        }
    }

    #[test]
    fn test_matches_std_over_range() {
        let mut x: f64 = -40.0;
        while x <= 700.0 {
            let got: f64 = expm1(x);
            let want: f64 = x.exp_m1();
            assert!(
                (got - want).abs() <= want.abs() * 1e-14 + 1e-300,
                "expm1({x}) = {got}, want {want}"
            );
            x += 0.017;
        }
    }
}
