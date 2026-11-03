// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::set_errno;
use ::sysapi::{
    errno::{
        EINVAL,
        ERANGE,
    },
    ffi::{
        c_char,
        c_int,
        c_long,
    },
};

//==================================================================================================
// Private Functions
//==================================================================================================

/// Returns `true` if the given character is an ASCII whitespace character.
fn is_whitespace(c: c_char) -> bool {
    let b = c as u8;
    b == b' ' || b == b'\t' || b == b'\n' || b == b'\r' || b == 0x0b || b == 0x0c
}

/// Returns the numeric value of a digit character for the given base, or `None` if invalid.
fn digit_value(c: c_char, base: c_int) -> Option<u8> {
    let b = c as u8;
    let val = match b {
        b'0'..=b'9' => b - b'0',
        b'a'..=b'z' => b - b'a' + 10,
        b'A'..=b'Z' => b - b'A' + 10,
        _ => return None,
    };
    if c_int::from(val) < base {
        Some(val)
    } else {
        None
    }
}

//==================================================================================================
// Standalone Functions
//==================================================================================================

///
/// # Description
///
/// Converts the initial portion of the string pointed to by `nptr` to a `c_long` value
/// according to the given `base`.
///
/// # Parameters
///
/// - `nptr`: Pointer to the string to be converted.
/// - `endptr`: If not null, receives a pointer to the first character not converted.
/// - `base`: Number base to use (0 for auto-detection, or 2-36).
///
/// # Returns
///
/// The converted value. On overflow, returns `c_long::MAX` or `c_long::MIN` and sets `errno`
/// to `ERANGE`. If no conversion could be performed, returns `0` and sets `*endptr` to `nptr`.
///
/// # Safety
///
/// This function is unsafe because it dereferences raw pointers. The caller must ensure that
/// `nptr` points to a valid null-terminated string.
///
/// # References
///
/// - https://pubs.opengroup.org/onlinepubs/9799919799/functions/strtol.html
///
#[cfg_attr(not(feature = "std"), unsafe(no_mangle))]
pub unsafe extern "C" fn strtol(
    nptr: *const c_char,
    endptr: *mut *mut c_char,
    base: c_int,
) -> c_long {
    if nptr.is_null() {
        if !endptr.is_null() {
            *endptr = core::ptr::null_mut();
        }
        return 0;
    }

    // Validate base.
    if base != 0 && !(2..=36).contains(&base) {
        set_errno(EINVAL);
        if !endptr.is_null() {
            *endptr = nptr.cast_mut();
        }
        return 0;
    }

    let mut p = nptr;

    // Skip leading whitespace.
    while is_whitespace(*p) {
        p = p.add(1);
    }

    // Handle optional sign.
    let negative = *p as u8 == b'-';
    if *p as u8 == b'+' || *p as u8 == b'-' {
        p = p.add(1);
    }

    // Determine actual base from prefix.
    let actual_base = detect_base(&mut p, base);

    // Parse digits using i128 to detect overflow.
    let start = p;
    let mut result: i128 = 0;
    let mut overflowed = false;
    let base_i128 = i128::from(actual_base);

    while let Some(d) = digit_value(*p, actual_base) {
        if !overflowed {
            match result
                .checked_mul(base_i128)
                .and_then(|r| r.checked_add(i128::from(d)))
            {
                Some(r) => result = r,
                None => overflowed = true,
            }
        }
        p = p.add(1);
    }

    // If no digits were parsed, set endptr to nptr.
    if p == start {
        if !endptr.is_null() {
            *endptr = nptr.cast_mut();
        }
        return 0;
    }

    // Set endptr to first unconverted character.
    if !endptr.is_null() {
        *endptr = p.cast_mut();
    }

    // Check for overflow.
    let max_val = i128::from(c_long::MAX);
    let min_magnitude = i128::from(c_long::MAX) + 1; // |c_long::MIN|

    if overflowed || (!negative && result > max_val) || (negative && result > min_magnitude) {
        set_errno(ERANGE);
        return if negative { c_long::MIN } else { c_long::MAX };
    }

    // Apply sign and convert to c_long.
    let signed_result = if negative { -result } else { result };
    match c_long::try_from(signed_result) {
        Ok(v) => v,
        Err(_) => {
            set_errno(ERANGE);
            if negative {
                c_long::MIN
            } else {
                c_long::MAX
            }
        },
    }
}

/// Detects the numeric base from a string prefix and advances the pointer past the prefix.
///
/// # Safety
///
/// The caller must ensure `p` points to a valid, dereferenceable position in a null-terminated
/// string.
unsafe fn detect_base(p: &mut *const c_char, base: c_int) -> c_int {
    let mut actual_base = base;

    if **p as u8 == b'0' {
        let next = *p.add(1);
        if (next as u8 == b'x' || next as u8 == b'X') && (actual_base == 0 || actual_base == 16) {
            // Only consume the "0x" prefix if a valid hex digit follows.
            if digit_value(*p.add(2), 16).is_some() {
                actual_base = 16;
                *p = p.add(2);
            } else if actual_base == 0 {
                actual_base = 8;
            }
        } else if actual_base == 0 {
            actual_base = 8;
        }
    } else if actual_base == 0 {
        actual_base = 10;
    }

    actual_base
}

#[cfg(all(test, feature = "std"))]
mod tests;
