// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::inttypes::intmax_t;

//==================================================================================================
// Public Functions
//==================================================================================================

/// Computes the absolute value of `n`.
///
/// If `n` is [`i64::MIN`], the result is [`i64::MIN`] due to two's complement wrapping.
#[cfg_attr(not(feature = "std"), unsafe(no_mangle))]
pub extern "C" fn imaxabs(n: intmax_t) -> intmax_t {
    n.wrapping_abs()
}

//==================================================================================================
// Tests
//==================================================================================================

#[cfg(all(test, feature = "std"))]
mod tests {
    use super::*;

    #[test]
    fn test_positive() {
        assert_eq!(imaxabs(42), 42);
    }

    #[test]
    fn test_negative() {
        assert_eq!(imaxabs(-42), 42);
    }

    #[test]
    fn test_zero() {
        assert_eq!(imaxabs(0), 0);
    }

    #[test]
    fn test_min() {
        // wrapping_abs of i64::MIN returns i64::MIN.
        assert_eq!(imaxabs(i64::MIN), i64::MIN);
    }
}
