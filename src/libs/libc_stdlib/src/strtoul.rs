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
        c_ulong,
    },
};

//==================================================================================================
// Standalone Functions
//==================================================================================================

///
/// # Description
///
/// Converts the initial portion of the string pointed to by `nptr` to a `c_ulong` value
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
/// The converted value. On overflow, returns `c_ulong::MAX` and sets `errno` to `ERANGE`.
///
/// # Safety
///
/// This function is unsafe because it dereferences raw pointers.
///
/// # References
///
/// - https://pubs.opengroup.org/onlinepubs/9799919799/functions/strtoul.html
///
#[cfg_attr(not(feature = "std"), unsafe(no_mangle))]
pub unsafe extern "C" fn strtoul(
    nptr: *const c_char,
    endptr: *mut *mut c_char,
    base: c_int,
) -> c_ulong {
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

    // Handle optional sign (C standard: strtoul accepts a leading minus and wraps).
    let negative = *p as u8 == b'-';
    if *p as u8 == b'+' || *p as u8 == b'-' {
        p = p.add(1);
    }

    // Determine actual base from prefix.
    let actual_base = detect_base(&mut p, base);

    // Parse digits.
    let start = p;
    let mut result: u64 = 0;
    let mut overflowed = false;
    let base_u64 = u64::from(actual_base as u32);

    while let Some(d) = digit_value(*p, actual_base) {
        if !overflowed {
            match result
                .checked_mul(base_u64)
                .and_then(|r| r.checked_add(u64::from(d)))
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
    // `u64::from` is a real widening on 32-bit targets (where `c_ulong` is `u32`) but a no-op on
    // 64-bit hosts; allow the resulting useless-conversion lint in the latter case.
    #[allow(clippy::useless_conversion)]
    let max_val = u64::from(c_ulong::MAX);
    if overflowed || result > max_val {
        set_errno(ERANGE);
        return c_ulong::MAX;
    }

    // Convert to c_ulong (wrapping for negative per C standard).
    let unsigned_result = match c_ulong::try_from(result) {
        Ok(v) => v,
        Err(_) => {
            set_errno(ERANGE);
            return c_ulong::MAX;
        },
    };

    if negative {
        unsigned_result.wrapping_neg()
    } else {
        unsigned_result
    }
}

#[cfg(all(test, feature = "std"))]
mod tests;
