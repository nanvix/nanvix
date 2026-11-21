// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::{
    inttypes::uintmax_t,
    strtoimax::{
        digit_value,
        parse_prefix,
        Prefix,
    },
};
use ::sysapi::ffi::{
    c_char,
    c_int,
};

//==================================================================================================
// Public Functions
//==================================================================================================

/// Converts the initial portion of the string pointed to by `nptr` to [`uintmax_t`].
///
/// Skips leading whitespace, handles an optional `+`/`-` sign, auto-detects the base when `base`
/// is `0` (hexadecimal for `0x`/`0X` prefix, octal for `0` prefix, decimal otherwise), and parses
/// digits for bases 2 through 36. On overflow the result is clamped to [`u64::MAX`]. A leading `-`
/// sign causes the parsed magnitude to be negated using unsigned wrapping arithmetic, matching the
/// C standard behavior for `strtoumax`.
///
/// # Safety
///
/// - `nptr` must point to a valid, null-terminated C string.
/// - If `endptr` is non-null, it must point to a valid, writable `*mut c_char` location.
#[cfg_attr(not(feature = "std"), unsafe(no_mangle))]
pub unsafe extern "C" fn strtoumax(
    nptr: *const c_char,
    endptr: *mut *mut c_char,
    base: c_int,
) -> uintmax_t {
    debug_assert!(!nptr.is_null());

    let Some(prefix): Option<Prefix> = parse_prefix(nptr, endptr, base) else {
        return 0;
    };
    let mut idx: usize = prefix.idx;
    let negative: bool = prefix.negative;
    let effective_base: i64 = prefix.base;

    // Convert the effective base to u64 for accumulation.
    let base_u64: u64 = match u64::try_from(effective_base) {
        Ok(v) => v,
        Err(_) => {
            if !endptr.is_null() {
                *endptr = nptr as *mut c_char;
            }
            return 0;
        },
    };

    // Parse digits, accumulating in u64.
    let mut result: u64 = 0;
    let mut overflow = false;
    let mut any_digits = false;

    loop {
        let c = i64::from(*nptr.add(idx));
        let digit_i64 = match digit_value(c) {
            Some(d) if d < effective_base => d,
            _ => break,
        };

        // digit_i64 is in 0..=35 so conversion to u64 is infallible.
        let Ok(digit) = u64::try_from(digit_i64) else {
            break;
        };

        any_digits = true;

        if !overflow {
            match result.checked_mul(base_u64) {
                Some(v) => match v.checked_add(digit) {
                    Some(v2) => result = v2,
                    None => overflow = true,
                },
                None => overflow = true,
            }
        }

        idx += 1;
    }

    // Set endptr.
    if !endptr.is_null() {
        if any_digits {
            *endptr = nptr.add(idx) as *mut c_char;
        } else {
            *endptr = nptr as *mut c_char;
        }
    }

    if overflow {
        // SAFETY: `__errno_location()` returns a valid pointer to thread-local `errno`.
        *::sysapi::errno::__errno_location() = ::sysapi::errno::ERANGE;
        u64::MAX
    } else if negative {
        result.wrapping_neg()
    } else {
        result
    }
}

//==================================================================================================
// Tests
//==================================================================================================

#[cfg(all(test, feature = "std"))]
mod tests;
