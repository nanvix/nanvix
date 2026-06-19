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
        c_longlong,
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
    use super::strtoll;
    use crate::set_errno;
    use ::sysapi::{
        errno::ERANGE,
        ffi::{
            c_char,
            c_int,
            c_longlong,
        },
    };

    fn get_errno() -> c_int {
        unsafe { *sysapi::errno::__errno_location() }
    }

    #[test]
    fn decimal() {
        let s = b"123\0";
        let result = unsafe { strtoll(s.as_ptr().cast::<c_char>(), core::ptr::null_mut(), 10) };
        assert_eq!(result, 123);
    }

    #[test]
    fn large_positive() {
        let s = b"9223372036854775807\0"; // i64::MAX
        let result = unsafe { strtoll(s.as_ptr().cast::<c_char>(), core::ptr::null_mut(), 10) };
        assert_eq!(result, c_longlong::MAX);
    }

    #[test]
    fn large_negative() {
        let s = b"-9223372036854775808\0"; // i64::MIN
        let result = unsafe { strtoll(s.as_ptr().cast::<c_char>(), core::ptr::null_mut(), 10) };
        assert_eq!(result, c_longlong::MIN);
    }

    #[test]
    fn overflow_positive() {
        set_errno(0);
        let s = b"9223372036854775808\0"; // i64::MAX + 1
        let result = unsafe { strtoll(s.as_ptr().cast::<c_char>(), core::ptr::null_mut(), 10) };
        assert_eq!(result, c_longlong::MAX);
        assert_eq!(get_errno(), ERANGE);
    }

    #[test]
    fn overflow_negative() {
        set_errno(0);
        let s = b"-9223372036854775809\0"; // i64::MIN - 1
        let result = unsafe { strtoll(s.as_ptr().cast::<c_char>(), core::ptr::null_mut(), 10) };
        assert_eq!(result, c_longlong::MIN);
        assert_eq!(get_errno(), ERANGE);
    }
}
