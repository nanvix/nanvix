// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::{
    inttypes::uintmax_t,
    strtoimax::{
        digit_value,
        is_whitespace,
    },
};
use ::sysapi::ffi::{
    c_char,
    c_int,
};

//==================================================================================================
// Constants
//==================================================================================================

const CHAR_ZERO: i64 = 0x30;
const CHAR_UPPER_X: i64 = 0x58;
const CHAR_LOWER_X: i64 = 0x78;
const CHAR_PLUS: i64 = 0x2B;
const CHAR_MINUS: i64 = 0x2D;

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

    // Reject invalid bases per the C contract: `base` must be 0 (auto-detect) or in 2..=36.
    // An invalid base performs no conversion: POSIX requires `errno` to be set to `EINVAL`, so set
    // it, set `*endptr` to `nptr`, and return 0.
    let requested_base: i64 = i64::from(base);
    if requested_base != 0 && !(2..=36).contains(&requested_base) {
        // SAFETY: `__errno_location()` returns a valid pointer to thread-local `errno`.
        *::sysapi::errno::__errno_location() = ::sysapi::errno::EINVAL;
        if !endptr.is_null() {
            *endptr = nptr as *mut c_char;
        }
        return 0;
    }

    let mut idx: usize = 0;

    // Skip leading whitespace.
    loop {
        let c = i64::from(*nptr.add(idx));
        if !is_whitespace(c) {
            break;
        }
        idx += 1;
    }

    // Handle optional sign.
    let negative = {
        let c = i64::from(*nptr.add(idx));
        if c == CHAR_MINUS {
            idx += 1;
            true
        } else {
            if c == CHAR_PLUS {
                idx += 1;
            }
            false
        }
    };

    // Determine effective base.
    let mut effective_base: i64 = requested_base;
    let c = i64::from(*nptr.add(idx));
    if c == CHAR_ZERO {
        let next = i64::from(*nptr.add(idx + 1));
        let next_is_x = next == CHAR_LOWER_X || next == CHAR_UPPER_X;
        // Only honor a `0x`/`0X` prefix when it is immediately followed by at least one
        // hexadecimal digit. Otherwise the leading `0` is itself the (only) digit and parsing
        // must stop at `x`, leaving `endptr` pointing at it (e.g. "0x" or "0xG").
        let prefix_has_hex_digit = next_is_x && {
            // SAFETY: `next` is `x`/`X` (non-null), so reading `idx + 2` stays within the string.
            let after = i64::from(*nptr.add(idx + 2));
            matches!(digit_value(after), Some(d) if d < 16)
        };
        if prefix_has_hex_digit && (effective_base == 0 || effective_base == 16) {
            effective_base = 16;
            idx += 2;
        } else if effective_base == 0 {
            effective_base = 8;
            // Don't advance past '0' — let the digit loop parse it.
        }
    } else if effective_base == 0 {
        effective_base = 10;
    }

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
