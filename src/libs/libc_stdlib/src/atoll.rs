// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use ::sysapi::ffi::{
    c_char,
    c_longlong,
};

//==================================================================================================
// Private Functions
//==================================================================================================

/// Returns `true` if the given character is an ASCII whitespace character.
fn is_whitespace(c: c_char) -> bool {
    let b: u8 = crate::c_char_to_u8(c);
    b == b' ' || b == b'\t' || b == b'\n' || b == b'\r' || b == 0x0b || b == 0x0c
}

//==================================================================================================
// Standalone Functions
//==================================================================================================

///
/// # Description
///
/// Converts the initial portion of the string pointed to by `nptr` to `c_longlong`.
///
/// # Parameters
///
/// - `nptr`: Pointer to the string to be converted.
///
/// # Returns
///
/// The converted value. Returns `0` if no valid conversion could be performed.
///
/// # Safety
///
/// This function is unsafe because it dereferences a raw pointer. The caller must ensure that
/// `nptr` points to a valid null-terminated string.
///
/// # References
///
/// - https://pubs.opengroup.org/onlinepubs/9799919799/functions/atoll.html
///
#[cfg_attr(not(feature = "std"), unsafe(no_mangle))]
pub unsafe extern "C" fn atoll(nptr: *const c_char) -> c_longlong {
    if nptr.is_null() {
        return 0;
    }

    let mut p = nptr;

    // Skip leading whitespace.
    while is_whitespace(*p) {
        p = p.add(1);
    }

    // Handle optional sign.
    let negative: bool = crate::c_char_to_u8(*p) == b'-';
    if crate::c_char_to_u8(*p) == b'+' || crate::c_char_to_u8(*p) == b'-' {
        p = p.add(1);
    }

    // Parse digits using wrapping arithmetic (overflow is undefined behavior in C).
    let mut result: c_longlong = 0;
    while crate::c_char_to_u8(*p).is_ascii_digit() {
        let digit: c_longlong = c_longlong::from(crate::c_char_to_u8(*p) - b'0');
        result = result.wrapping_mul(10).wrapping_add(digit);
        p = p.add(1);
    }

    if negative {
        result.wrapping_neg()
    } else {
        result
    }
}

#[cfg(all(test, feature = "std"))]
mod tests {
    use super::atoll;
    use ::sysapi::ffi::c_char;

    #[test]
    fn positive_value() {
        let s = b"123\0";
        assert_eq!(unsafe { atoll(s.as_ptr().cast::<c_char>()) }, 123);
    }

    #[test]
    fn negative_value() {
        let s = b"-456\0";
        assert_eq!(unsafe { atoll(s.as_ptr().cast::<c_char>()) }, -456);
    }

    #[test]
    fn large_value() {
        let s = b"1000000000000\0";
        assert_eq!(unsafe { atoll(s.as_ptr().cast::<c_char>()) }, 1_000_000_000_000);
    }
}
