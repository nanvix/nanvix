// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Constants
//==================================================================================================

/// `2 / pi`, for the argument reduction quotient.
const INVPIO2: f64 = 6.366_197_723_675_814e-1;
/// `pi/2` split into successively finer parts for an exact Cody-Waite reduction.
const PIO2_1: f64 = 1.570_796_326_734_125_6;
const PIO2_1T: f64 = 6.077_100_506_506_192e-11;
const PIO2_2: f64 = 6.077_100_506_303_966e-11;
const PIO2_2T: f64 = 2.022_266_248_795_950_6e-21;
const PIO2_3: f64 = 2.022_266_248_711_166_5e-21;
const PIO2_3T: f64 = 8.478_427_660_368_9e-32;

/// `pi/4` split into a high part and a low correction, for the near-`pi/4` branch.
const PIO4: f64 = 7.853_981_633_974_483e-1;
const PIO4LO: f64 = 3.061_616_997_868_383e-17;

/// Minimax coefficients for the tangent kernel on `[0, pi/4]`.
const KT: [f64; 13] = [
    3.333_333_333_333_341e-1,
    1.333_333_333_332_012_4e-1,
    5.396_825_397_622_605e-2,
    2.186_948_829_485_954_2e-2,
    8.863_239_823_599_3e-3,
    3.592_079_107_591_312_4e-3,
    1.456_209_454_325_290_3e-3,
    5.880_412_408_202_641e-4,
    2.464_631_348_184_699e-4,
    7.817_944_429_395_571e-5,
    7.140_724_913_826_082e-5,
    -1.855_863_748_552_754_6e-5,
    2.590_730_518_636_337e-5,
];

//==================================================================================================
// Internal Helpers
//==================================================================================================

/// Cody-Waite reduction of `x` (with high word `ix`) to `n*(pi/2) + (y0 + y1)`
/// with `|y0 + y1| <= pi/4`, valid for `|x| < 2^20*(pi/2)`. Returns `(n, y0, y1)`.
#[allow(clippy::cast_possible_truncation, clippy::cast_possible_wrap)]
fn rem_pio2_medium(x: f64, ix: u32) -> (i32, f64, f64) {
    let fnn: f64 = crate::round::round(x * INVPIO2);
    let n: i32 = fnn as i32;
    let mut r: f64 = x - fnn * PIO2_1;
    let mut w: f64 = fnn * PIO2_1T; // 1st round, good to 85 bits
    let j: i32 = (ix >> 20) as i32;
    let mut y0: f64 = r - w;
    let mut high: u32 = (y0.to_bits() >> 32) as u32;
    let mut i: i32 = j - (((high >> 20) & 0x7ff) as i32);
    if i > 16 {
        // 2nd iteration needed, good to 118 bits.
        let t: f64 = r;
        w = fnn * PIO2_2;
        r = t - w;
        w = fnn * PIO2_2T - ((t - r) - w);
        y0 = r - w;
        high = (y0.to_bits() >> 32) as u32;
        i = j - (((high >> 20) & 0x7ff) as i32);
        if i > 49 {
            // 3rd iteration needed, good to 151 bits.
            let t2: f64 = r;
            w = fnn * PIO2_3;
            r = t2 - w;
            w = fnn * PIO2_3T - ((t2 - r) - w);
            y0 = r - w;
        }
    }
    let y1: f64 = (r - y0) - w;
    (n, y0, y1)
}

/// Tangent kernel on the reduced argument `x + y` (with `|x| <= pi/4`). `iy` is
/// `1` when `tan` is wanted and `-1` when `-1/tan` (an odd quadrant) is wanted.
#[allow(clippy::cast_possible_truncation, clippy::cast_possible_wrap)]
fn kernel_tan(mut x: f64, mut y: f64, iy: i32) -> f64 {
    let hx: i32 = (x.to_bits() >> 32) as i32;
    let ix: i32 = hx & 0x7fff_ffff;
    if ix < 0x3e30_0000 {
        // |x| < 2^-28.
        if (x as i32) == 0 {
            let low: u32 = x.to_bits() as u32;
            if (((ix as u32) | low) | ((iy + 1) as u32)) == 0 {
                // x == 0 and an odd quadrant: this is a pole, return +inf.
                return 1.0 / f64::from_bits(x.to_bits() & 0x7fff_ffff_ffff_ffff);
            }
            return if iy == 1 { x } else { -1.0 / x };
        }
    }
    if ix >= 0x3FE5_9428 {
        // |x| >= 0.6744: reflect about pi/4 for better polynomial accuracy.
        if hx < 0 {
            x = -x;
            y = -y;
        }
        let z: f64 = PIO4 - x;
        let w: f64 = PIO4LO - y;
        x = z + w;
        y = 0.0;
    }
    let z: f64 = x * x;
    let w: f64 = z * z;
    let r: f64 = KT[1] + w * (KT[3] + w * (KT[5] + w * (KT[7] + w * (KT[9] + w * KT[11]))));
    let v: f64 = z * (KT[2] + w * (KT[4] + w * (KT[6] + w * (KT[8] + w * (KT[10] + w * KT[12])))));
    let s: f64 = z * x;
    let mut r: f64 = y + z * (s * (r + v) + y);
    r += KT[0] * s;
    let w: f64 = x + r;
    if ix >= 0x3FE5_9428 {
        let v: f64 = f64::from(iy);
        return f64::from(1 - ((hx >> 30) & 2)) * (v - 2.0 * (x - (w * w / (w + v) - r)));
    }
    if iy == 1 {
        return w;
    }
    // iy == -1: compute -1.0/(x+r) accurately.
    let z: f64 = f64::from_bits(w.to_bits() & 0xffff_ffff_0000_0000); // high word of w only
    let v: f64 = r - (z - x); // z + v = r + x
    let a: f64 = -1.0 / w;
    let t: f64 = f64::from_bits(a.to_bits() & 0xffff_ffff_0000_0000); // high word of a only
    let s: f64 = 1.0 + t * z;
    t + a * (s + t * v)
}

//==================================================================================================
// Public Functions
//==================================================================================================

/// Computes the tangent of `x` (in radians).
///
/// # Description
///
/// Reduces `x` modulo `pi/2` with a Cody-Waite scheme and evaluates a minimax
/// tangent kernel on the reduced argument. This is the fdlibm algorithm and is
/// accurate to within one unit in the last place for `|x| < 2^20*(pi/2)`; larger
/// arguments fall back to `sin(x)/cos(x)`.
///
/// # Parameters
///
/// - `x`: Input angle in radians.
///
/// # Returns
///
/// The tangent of `x`.
#[cfg_attr(not(feature = "std"), unsafe(no_mangle))]
#[allow(clippy::eq_op)]
pub extern "C" fn tan(x: f64) -> f64 {
    let ix: u32 = ((x.to_bits() >> 32) as u32) & 0x7fff_ffff;
    if ix <= 0x3fe9_21fb {
        // |x| <= pi/4.
        if ix < 0x3e40_0000 {
            // |x| < 2^-27: tan(x) rounds to x.
            return x;
        }
        return kernel_tan(x, 0.0, 1);
    }
    if ix >= 0x7ff0_0000 {
        return x - x; // tan(inf or NaN) is NaN
    }
    if ix >= 0x4139_21fb {
        // |x| >= 2^20*(pi/2): fall back to sin/cos for these rare huge arguments.
        return crate::sin::sin(x) / crate::cos::cos(x);
    }
    let (n, y0, y1) = rem_pio2_medium(x, ix);
    // n even -> tan(reduced); n odd -> -1/tan(reduced).
    kernel_tan(y0, y1, 1 - ((n & 1) << 1))
}

//==================================================================================================
// Unit Tests
//==================================================================================================

#[cfg(all(test, feature = "std"))]
mod tests {
    use super::*;

    const PI: f64 = core::f64::consts::PI;

    #[test]
    fn test_zero() {
        assert!((tan(0.0)).abs() < 1e-10);
    }

    #[test]
    fn test_pi_over_4() {
        assert!((tan(PI / 4.0) - 1.0).abs() < 1e-10);
    }

    #[test]
    fn test_negative() {
        assert!((tan(-PI / 4.0) - (-1.0)).abs() < 1e-10);
    }

    #[test]
    fn test_nan() {
        assert!(tan(f64::NAN).is_nan());
    }

    #[test]
    fn test_infinity() {
        assert!(tan(f64::INFINITY).is_nan());
    }

    #[test]
    fn test_matches_std_over_range() {
        // Sweep across many periods (within the Cody-Waite range) vs the host libm.
        let mut x: f64 = -1000.0;
        while x <= 1000.0 {
            let got: f64 = tan(x);
            let want: f64 = x.tan();
            // Skip points extremely close to a pole, where the true value is huge
            // and a relative comparison is dominated by reduction round-off.
            if want.abs() < 1e6 {
                assert!(
                    (got - want).abs() <= want.abs() * 1e-13 + 1e-13,
                    "tan({x}) = {got}, want {want}"
                );
            }
            x += 0.1;
        }
    }

    #[test]
    fn test_large_argument() {
        for &x in &[1.0e5_f64, 3.0e5, -7.0e5, 123456.789] {
            let got: f64 = tan(x);
            let want: f64 = x.tan();
            if want.abs() < 1e6 {
                assert!((got - want).abs() <= want.abs() * 1e-11 + 1e-11);
            }
        }
    }
}
