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
        c_ulong,
    },
};

//==================================================================================================
// Private Functions
//==================================================================================================

/// Returns `true` if the given character is an ASCII whitespace character.
fn is_whitespace(c: c_char) -> bool {
    let b: u8 = crate::c_char_to_u8(c);
    b == b' ' || b == b'\t' || b == b'\n' || b == b'\r' || b == 0x0b || b == 0x0c
}

/// Returns the numeric value of a digit character for the given base, or `None` if invalid.
fn digit_value(c: c_char, base: c_int) -> Option<u8> {
    let b: u8 = crate::c_char_to_u8(c);
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
    let negative: bool = crate::c_char_to_u8(*p) == b'-';
    if crate::c_char_to_u8(*p) == b'+' || crate::c_char_to_u8(*p) == b'-' {
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

/// Detects the numeric base from a string prefix and advances the pointer past the prefix.
///
/// # Safety
///
/// The caller must ensure `p` points to a valid, dereferenceable position in a null-terminated
/// string.
unsafe fn detect_base(p: &mut *const c_char, base: c_int) -> c_int {
    let mut actual_base = base;

    if crate::c_char_to_u8(**p) == b'0' {
        let next: u8 = crate::c_char_to_u8(*p.add(1));
        if (next == b'x' || next == b'X') && (actual_base == 0 || actual_base == 16) {
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
mod tests {
    use super::strtoul;
    use crate::set_errno;
    use ::sysapi::{
        errno::ERANGE,
        ffi::{
            c_char,
            c_int,
            c_ulong,
        },
    };

    fn get_errno() -> c_int {
        unsafe { *sysapi::errno::__errno_location() }
    }

    #[test]
    fn basic_decimal() {
        let s = b"123\0";
        let result = unsafe { strtoul(s.as_ptr().cast::<c_char>(), core::ptr::null_mut(), 10) };
        assert_eq!(result, 123);
    }

    #[test]
    fn hex_value() {
        let s = b"0xff\0";
        let result = unsafe { strtoul(s.as_ptr().cast::<c_char>(), core::ptr::null_mut(), 16) };
        assert_eq!(result, 255);
    }

    #[test]
    fn overflow() {
        set_errno(0);
        // Value exceeds 64-bit `c_ulong` (tests run on the 64-bit host), so it overflows on every
        // supported target width.
        let s = b"9999999999999999999999999\0";
        let result = unsafe { strtoul(s.as_ptr().cast::<c_char>(), core::ptr::null_mut(), 10) };
        assert_eq!(result, c_ulong::MAX);
        assert_eq!(get_errno(), ERANGE);
    }

    #[test]
    fn negative_wraps() {
        let s = b"-1\0";
        let result = unsafe { strtoul(s.as_ptr().cast::<c_char>(), core::ptr::null_mut(), 10) };
        assert_eq!(result, c_ulong::MAX);
    }
}
