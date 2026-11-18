// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::{
    set_errno,
    strtol::{
        detect_base,
        digit_value,
        is_whitespace,
    },
};
use ::sysapi::{
    errno::{
        EINVAL,
        ERANGE,
    },
    ffi::{
        c_char,
        c_int,
        c_longlong,
    },
};

//==================================================================================================
// Standalone Functions
//==================================================================================================

///
/// # Description
///
/// Converts the initial portion of the string pointed to by `nptr` to a `c_longlong` value
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
/// The converted value. On overflow, returns `c_longlong::MAX` or `c_longlong::MIN` and sets
/// `errno` to `ERANGE`.
///
/// # Safety
///
/// This function is unsafe because it dereferences raw pointers.
///
/// # References
///
/// - https://pubs.opengroup.org/onlinepubs/9799919799/functions/strtoll.html
///
#[cfg_attr(not(feature = "std"), unsafe(no_mangle))]
pub unsafe extern "C" fn strtoll(
    nptr: *const c_char,
    endptr: *mut *mut c_char,
    base: c_int,
) -> c_longlong {
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

    // Set endptr.
    if !endptr.is_null() {
        *endptr = p.cast_mut();
    }

    // Check for overflow.
    let max_val = i128::from(c_longlong::MAX);
    let min_magnitude = i128::from(c_longlong::MAX) + 1; // |c_longlong::MIN|

    if overflowed || (!negative && result > max_val) || (negative && result > min_magnitude) {
        set_errno(ERANGE);
        return if negative {
            c_longlong::MIN
        } else {
            c_longlong::MAX
        };
    }

    // Apply sign and convert to c_longlong.
    let signed_result = if negative { -result } else { result };
    match c_longlong::try_from(signed_result) {
        Ok(v) => v,
        Err(_) => {
            set_errno(ERANGE);
            if negative {
                c_longlong::MIN
            } else {
                c_longlong::MAX
            }
        },
    }
}

#[cfg(all(test, feature = "std"))]
mod tests;
