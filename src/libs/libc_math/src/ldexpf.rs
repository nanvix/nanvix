// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

use sysapi::ffi::c_int;

//==================================================================================================
// Public Functions
//==================================================================================================

/// Multiplies a single-precision floating-point number by a power of two.
///
/// # Parameters
///
/// - `x`: Base value.
/// - `exp`: Exponent.
///
/// # Returns
///
/// The value `x * 2^exp`.
#[cfg_attr(not(feature = "std"), unsafe(no_mangle))]
pub extern "C" fn ldexpf(x: f32, exp: c_int) -> f32 {
    if x == 0.0 || x.is_nan() || x.to_bits() & 0x7F80_0000 == 0x7F80_0000 {
        return x;
    }

    let mut n: i64 = i64::from(exp);
    let mut result: f32 = x;

    const SCALE_UP: f32 = f32::from_bits(0x7F00_0000u32); // 2^127
    const SCALE_DN: f32 = f32::from_bits(0x0080_0000u32); // 2^-126

    if n > 127 {
        result *= SCALE_UP;
        n -= 127;
        if n > 127 {
            result *= SCALE_UP;
            n -= 127;
            if n > 127 {
                n = 127;
            }
        }
    } else if n < -126 {
        result *= SCALE_DN;
        n += 126;
        if n < -126 {
            result *= SCALE_DN;
            n += 126;
            if n < -126 {
                n = -126;
            }
        }
    }

    let biased: u64 = (n + 127) as u64;
    #[allow(clippy::cast_possible_truncation)]
    let biased_u32: u32 = biased as u32;
    result * f32::from_bits(biased_u32 << 23)
}

//==================================================================================================
// Unit Tests
//==================================================================================================

#[cfg(all(test, feature = "std"))]
mod tests {
    use super::*;

    #[test]
    fn test_basic() {
        assert!((ldexpf(1.0, 3) - 8.0).abs() < 1e-5);
    }

    #[test]
    fn test_negative_exp() {
        assert!((ldexpf(8.0, -3) - 1.0).abs() < 1e-5);
    }

    #[test]
    fn test_zero() {
        assert_eq!(ldexpf(0.0, 10), 0.0);
    }
}
