// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::inttypes::intmax_t;

//==================================================================================================
// Types
//==================================================================================================

/// Result of an integer division performed by [`imaxdiv`].
#[repr(C)]
#[allow(non_camel_case_types)]
pub struct imaxdiv_t {
    /// Quotient of the division.
    pub quot: intmax_t,
    /// Remainder of the division.
    pub rem: intmax_t,
}

//==================================================================================================
// Public Functions
//==================================================================================================

/// Computes the quotient and remainder of the division `numer / denom`.
///
/// For every division that the C standard defines, the result satisfies
/// `numer == quot * denom + rem`, with the quotient truncated towards zero.
///
/// The C standard leaves division by zero and a non-representable quotient (namely
/// `INTMAX_MIN / -1`) undefined. To avoid trapping, this function performs no division in those
/// cases and instead returns `imaxdiv_t { quot: 0, rem: 0 }`; the `numer == quot * denom + rem`
/// identity does not hold for those inputs.
#[cfg_attr(not(feature = "std"), unsafe(no_mangle))]
pub extern "C" fn imaxdiv(numer: intmax_t, denom: intmax_t) -> imaxdiv_t {
    match (numer.checked_div(denom), numer.checked_rem(denom)) {
        (Some(quot), Some(rem)) => imaxdiv_t { quot, rem },
        _ => imaxdiv_t { quot: 0, rem: 0 },
    }
}

//==================================================================================================
// Tests
//==================================================================================================

#[cfg(all(test, feature = "std"))]
mod tests;
