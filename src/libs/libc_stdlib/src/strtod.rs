// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::set_errno;
use ::sysapi::{
    errno::ERANGE,
    ffi::c_char,
};

//==================================================================================================
// Private Functions
//==================================================================================================

/// Returns `true` if the given character is an ASCII whitespace character.
fn is_whitespace(c: c_char) -> bool {
    let b: u8 = c as u8;
    b == b' ' || b == b'\t' || b == b'\n' || b == b'\r' || b == 0x0b || b == 0x0c
}

/// Returns `true` if the given character is an ASCII digit.
fn is_digit(c: c_char) -> bool {
    let b: u8 = c as u8;
    b.is_ascii_digit()
}

/// Returns `true` if the given character is an ASCII hexadecimal digit.
fn is_hex_digit(c: c_char) -> bool {
    let b: u8 = c as u8;
    b.is_ascii_hexdigit()
}

/// Returns the numeric value of an ASCII digit character.
fn digit_val(c: c_char) -> u8 {
    (c as u8) - b'0'
}

/// Returns the numeric value of an ASCII hexadecimal digit character.
fn hex_digit_val(c: c_char) -> u8 {
    match c as u8 {
        b'0'..=b'9' => (c as u8) - b'0',
        b'a'..=b'f' => (c as u8) - b'a' + 10,
        b'A'..=b'F' => (c as u8) - b'A' + 10,
        _ => 0,
    }
}

/// Returns `true` if the bytes at `p` case-insensitively match the ASCII keyword `kw`.
///
/// # Safety
///
/// `p` must point into a valid, NUL-terminated string. Comparison stops at the first mismatch, and
/// because a keyword never contains a NUL byte, at most `kw.len()` bytes are read and never past the
/// terminator.
unsafe fn match_keyword(p: *const c_char, kw: &[u8]) -> bool {
    let mut i: usize = 0;
    while i < kw.len() {
        let c: u8 = *p.add(i) as u8;
        if c == 0 || c.to_ascii_lowercase() != kw[i] {
            return false;
        }
        i += 1;
    }
    true
}

/// Parses a hexadecimal floating-point subject sequence starting at `p`.
///
/// # Safety
///
/// `p` must point into a valid, NUL-terminated string.
unsafe fn parse_hex_float(p: *const c_char) -> Option<(*const c_char, f64, bool)> {
    if *p as u8 != b'0' {
        return None;
    }
    let marker: u8 = *p.add(1) as u8;
    if marker != b'x' && marker != b'X' {
        return None;
    }

    let mut q: *const c_char = p.add(2);
    let mut value: f64 = 0.0;
    let mut parsed_digit: bool = false;
    let mut parsed_nonzero_digit: bool = false;

    while is_hex_digit(*q) {
        let digit: u8 = hex_digit_val(*q);
        parsed_digit = true;
        parsed_nonzero_digit |= digit != 0;
        value = value * 16.0 + f64::from(digit);
        q = q.add(1);
    }

    if *q as u8 == b'.' {
        q = q.add(1);
        let mut divisor: f64 = 16.0;
        while is_hex_digit(*q) {
            let digit: u8 = hex_digit_val(*q);
            parsed_digit = true;
            parsed_nonzero_digit |= digit != 0;
            value += f64::from(digit) / divisor;
            divisor *= 16.0;
            q = q.add(1);
        }
    }

    if !parsed_digit {
        return None;
    }

    if *q as u8 == b'p' || *q as u8 == b'P' {
        let exponent_marker: *const c_char = q;
        q = q.add(1);

        let exp_negative: bool = *q as u8 == b'-';
        if *q as u8 == b'+' || *q as u8 == b'-' {
            q = q.add(1);
        }

        if is_digit(*q) {
            let mut exp: i32 = 0;
            while is_digit(*q) {
                exp = exp
                    .saturating_mul(10)
                    .saturating_add(i32::from(digit_val(*q)));
                q = q.add(1);
            }
            if exp_negative {
                exp = exp.saturating_neg();
            }
            value *= pow2(exp);
        } else {
            q = exponent_marker;
        }
    }

    Some((q, value, parsed_nonzero_digit))
}

//==================================================================================================
// Standalone Functions
//==================================================================================================

///
/// # Description
///
/// Converts the initial portion of the string pointed to by `nptr` to `f64`.
///
/// The expected form of the subject sequence is an optional sign (`+` or `-`), followed by one of:
/// - A non-empty sequence of decimal digits optionally containing a decimal-point character,
///   optionally followed by an exponent part (`e` or `E` with optional sign and digits).
/// - A `0x` or `0X` prefix followed by a non-empty sequence of hexadecimal digits optionally
///   containing a decimal-point character, optionally followed by a binary exponent part (`p` or
///   `P` with optional sign and digits).
/// - One of `INF` or `INFINITY`, ignoring case.
/// - One of `NAN` or `NAN(n-char-sequence)`, ignoring case.
///
/// # Parameters
///
/// - `nptr`: Pointer to the null-terminated string to be converted.
/// - `endptr`: If not null, receives a pointer to the first character not converted.
///
/// # Returns
///
/// The converted `f64` value. Returns `0.0` if no valid conversion could be performed.
///
/// # Safety
///
/// This function is unsafe because it dereferences raw pointers. The caller must ensure that
/// `nptr` points to a valid null-terminated string.
///
/// # References
///
/// - https://pubs.opengroup.org/onlinepubs/9799919799/functions/strtod.html
///
#[cfg_attr(not(feature = "std"), unsafe(no_mangle))]
#[allow(clippy::cast_precision_loss)]
pub unsafe extern "C" fn strtod(nptr: *const c_char, endptr: *mut *mut c_char) -> f64 {
    if nptr.is_null() {
        if !endptr.is_null() {
            *endptr = core::ptr::null_mut();
        }
        return 0.0;
    }

    let mut p: *const c_char = nptr;

    // Skip leading whitespace.
    while is_whitespace(*p) {
        p = p.add(1);
    }

    // Handle optional sign.
    let negative: bool = *p as u8 == b'-';
    if *p as u8 == b'+' || *p as u8 == b'-' {
        p = p.add(1);
    }

    // Recognize the `INF`/`INFINITY` and `NAN` spellings (case-insensitive) before digit parsing.
    if match_keyword(p, b"inf") {
        // Consume the longer `infinity` spelling when present, otherwise just `inf`.
        let len: usize = if match_keyword(p, b"infinity") { 8 } else { 3 };
        p = p.add(len);
        if !endptr.is_null() {
            *endptr = p.cast_mut();
        }
        return if negative {
            f64::NEG_INFINITY
        } else {
            f64::INFINITY
        };
    }
    if match_keyword(p, b"nan") {
        p = p.add(3);
        // Optionally consume a parenthesized n-char-sequence: '(' [0-9A-Za-z_]* ')'.
        if *p as u8 == b'(' {
            let mut q: *const c_char = p.add(1);
            loop {
                let c: u8 = *q as u8;
                if c != b'_' && !c.is_ascii_alphanumeric() {
                    break;
                }
                q = q.add(1);
            }
            if *q as u8 == b')' {
                p = q.add(1);
            }
        }
        if !endptr.is_null() {
            *endptr = p.cast_mut();
        }
        return if negative { -f64::NAN } else { f64::NAN };
    }

    let start: *const c_char = p;

    if let Some((end, value, parsed_nonzero_digit)) = parse_hex_float(p) {
        let result: f64 = if negative { -value } else { value };
        if result.is_infinite()
            || (parsed_nonzero_digit && (result == 0.0 || result.abs() < f64::MIN_POSITIVE))
        {
            set_errno(ERANGE);
        }
        if !endptr.is_null() {
            *endptr = end.cast_mut();
        }
        return result;
    }

    // Parse integer part.
    let mut int_part: f64 = 0.0;
    let mut parsed_nonzero_digit: bool = false;
    while is_digit(*p) {
        let digit: u8 = digit_val(*p);
        parsed_nonzero_digit |= digit != 0;
        int_part = int_part * 10.0 + f64::from(digit);
        p = p.add(1);
    }

    // Parse fractional part.
    let mut frac_part: f64 = 0.0;
    if *p as u8 == b'.' {
        p = p.add(1);
        let mut divisor: f64 = 10.0;
        while is_digit(*p) {
            let digit: u8 = digit_val(*p);
            parsed_nonzero_digit |= digit != 0;
            frac_part += f64::from(digit) / divisor;
            divisor *= 10.0;
            p = p.add(1);
        }
    }

    // If no digits were parsed at all, set endptr to nptr.
    if p == start || (p == start.add(1) && *start as u8 == b'.') {
        // Check if the only character parsed was a lone '.'.
        let parsed_any_digits: bool = p != start && !(p == start.add(1) && *start as u8 == b'.');
        if !parsed_any_digits {
            if !endptr.is_null() {
                *endptr = nptr.cast_mut();
            }
            return 0.0;
        }
    }

    let mut result: f64 = int_part + frac_part;

    // Parse exponent part.
    if *p as u8 == b'e' || *p as u8 == b'E' {
        let saved: *const c_char = p;
        p = p.add(1);

        let exp_negative: bool = *p as u8 == b'-';
        if *p as u8 == b'+' || *p as u8 == b'-' {
            p = p.add(1);
        }

        if is_digit(*p) {
            let mut exp: i32 = 0;
            while is_digit(*p) {
                exp = exp
                    .saturating_mul(10)
                    .saturating_add(i32::from(digit_val(*p)));
                p = p.add(1);
            }

            if exp_negative {
                exp = exp.saturating_neg();
            }

            result *= pow10(exp);
        } else {
            // No digits after 'e'/'E', rewind.
            p = saved;
        }
    }

    if negative {
        result = -result;
    }

    // A finite subject sequence whose magnitude is too large to represent overflows to infinity;
    // POSIX requires `errno` to be set to `ERANGE` in that case. The `INF`/`NAN` spellings return
    // earlier, so reaching infinity here can only be the result of overflow.
    if result.is_infinite() {
        set_errno(ERANGE);
    }
    if parsed_nonzero_digit && (result == 0.0 || result.abs() < f64::MIN_POSITIVE) {
        set_errno(ERANGE);
    }

    if !endptr.is_null() {
        *endptr = p.cast_mut();
    }

    result
}

/// Computes 10 raised to the power `exp`.
#[allow(clippy::cast_precision_loss)]
fn pow10(exp: i32) -> f64 {
    let mut result: f64 = 1.0;
    let mut base: f64 = 10.0;
    let mut n: u32 = exp.unsigned_abs();

    while n > 0 {
        if n & 1 != 0 {
            result *= base;
        }
        base *= base;
        n >>= 1;
    }

    if exp < 0 {
        1.0 / result
    } else {
        result
    }
}

/// Computes 2 raised to the power `exp`.
fn pow2(exp: i32) -> f64 {
    let mut result: f64 = 1.0;
    let mut base: f64 = 2.0;
    let mut n: u32 = exp.unsigned_abs();

    while n > 0 {
        if n & 1 != 0 {
            result *= base;
        }
        base *= base;
        n >>= 1;
    }

    if exp < 0 {
        1.0 / result
    } else {
        result
    }
}

#[cfg(all(test, feature = "std"))]
mod tests {
    use super::strtod;
    use crate::set_errno;
    use ::sysapi::{
        errno::ERANGE,
        ffi::{
            c_char,
            c_int,
        },
    };

    fn get_errno() -> c_int {
        unsafe { *::sysapi::errno::__errno_location() }
    }

    /// Helper to compare floats within an epsilon.
    fn approx_eq(a: f64, b: f64, eps: f64) -> bool {
        (a - b).abs() < eps
    }

    #[test]
    fn basic_decimal() {
        let s = b"3.25\0";
        let result: f64 = unsafe { strtod(s.as_ptr().cast::<c_char>(), core::ptr::null_mut()) };
        assert!(approx_eq(result, 3.25, 1e-10), "expected 3.25, got {result}");
    }

    #[test]
    fn negative_value() {
        let s = b"-1.5\0";
        let result: f64 = unsafe { strtod(s.as_ptr().cast::<c_char>(), core::ptr::null_mut()) };
        assert!(approx_eq(result, -1.5, 1e-10), "expected -1.5, got {result}");
    }

    #[test]
    fn exponent_positive() {
        let s = b"1e10\0";
        let result: f64 = unsafe { strtod(s.as_ptr().cast::<c_char>(), core::ptr::null_mut()) };
        assert!(approx_eq(result, 1e10, 1.0), "expected 1e10, got {result}");
    }

    #[test]
    fn exponent_negative() {
        let s = b"1.5e-3\0";
        let result: f64 = unsafe { strtod(s.as_ptr().cast::<c_char>(), core::ptr::null_mut()) };
        assert!(approx_eq(result, 1.5e-3, 1e-12), "expected 1.5e-3, got {result}");
    }

    #[test]
    fn hexadecimal_without_exponent() {
        let s = b"0x10tail\0";
        let mut end: *mut c_char = core::ptr::null_mut();
        let result: f64 = unsafe { strtod(s.as_ptr().cast::<c_char>(), &mut end) };
        assert!(approx_eq(result, 16.0, 1e-10), "expected 16.0, got {result}");
        assert_eq!(unsafe { *end } as u8, b't');
    }

    #[test]
    fn hexadecimal_with_binary_exponent() {
        let s = b"0x1.8p+1\0";
        let result: f64 = unsafe { strtod(s.as_ptr().cast::<c_char>(), core::ptr::null_mut()) };
        assert!(approx_eq(result, 3.0, 1e-10), "expected 3.0, got {result}");
    }

    #[test]
    fn no_decimal() {
        let s = b"42\0";
        let result: f64 = unsafe { strtod(s.as_ptr().cast::<c_char>(), core::ptr::null_mut()) };
        assert!(approx_eq(result, 42.0, 1e-10), "expected 42.0, got {result}");
    }

    #[test]
    fn endptr_set() {
        let s = b"3.25abc\0";
        let mut end: *mut c_char = core::ptr::null_mut();
        let result: f64 = unsafe { strtod(s.as_ptr().cast::<c_char>(), &mut end) };
        assert!(approx_eq(result, 3.25, 1e-10), "expected 3.25, got {result}");
        assert!(!end.is_null());
        assert_eq!(unsafe { *end } as u8, b'a');
    }

    #[test]
    fn leading_whitespace() {
        let s = b"  +42.5\0";
        let result: f64 = unsafe { strtod(s.as_ptr().cast::<c_char>(), core::ptr::null_mut()) };
        assert!(approx_eq(result, 42.5, 1e-10), "expected 42.5, got {result}");
    }

    #[test]
    fn no_valid_conversion() {
        let s = b"abc\0";
        let mut end: *mut c_char = core::ptr::null_mut();
        let result: f64 = unsafe { strtod(s.as_ptr().cast::<c_char>(), &mut end) };
        assert!(approx_eq(result, 0.0, 1e-10), "expected 0.0, got {result}");
        assert_eq!(end, s.as_ptr().cast_mut().cast::<c_char>());
    }

    #[test]
    fn parses_infinity() {
        let s = b"inf\0";
        let result: f64 = unsafe { strtod(s.as_ptr().cast::<c_char>(), core::ptr::null_mut()) };
        assert!(result.is_infinite() && result > 0.0, "expected +inf, got {result}");
    }

    #[test]
    fn parses_negative_infinity_long_form() {
        let s = b"-INFINITY\0";
        let mut end: *mut c_char = core::ptr::null_mut();
        let result: f64 = unsafe { strtod(s.as_ptr().cast::<c_char>(), &mut end) };
        assert!(result.is_infinite() && result < 0.0, "expected -inf, got {result}");
        // The whole token is consumed.
        assert_eq!(unsafe { *end } as u8, 0);
    }

    #[test]
    fn parses_nan() {
        let s = b"NaN\0";
        let result: f64 = unsafe { strtod(s.as_ptr().cast::<c_char>(), core::ptr::null_mut()) };
        assert!(result.is_nan(), "expected NaN, got {result}");
    }

    #[test]
    fn parses_nan_with_sequence() {
        let s = b"nan(123)x\0";
        let mut end: *mut c_char = core::ptr::null_mut();
        let result: f64 = unsafe { strtod(s.as_ptr().cast::<c_char>(), &mut end) };
        assert!(result.is_nan(), "expected NaN, got {result}");
        // The parenthesized sequence is consumed; endptr points just past it.
        assert_eq!(unsafe { *end } as u8, b'x');
    }

    #[test]
    fn overflow_sets_erange() {
        set_errno(0);
        let s = b"1e400\0";
        let result: f64 = unsafe { strtod(s.as_ptr().cast::<c_char>(), core::ptr::null_mut()) };
        assert!(result.is_infinite(), "expected inf, got {result}");
        assert_eq!(get_errno(), ERANGE);
    }

    #[test]
    fn underflow_sets_erange() {
        set_errno(0);
        let s = b"1e-4000\0";
        let result: f64 = unsafe { strtod(s.as_ptr().cast::<c_char>(), core::ptr::null_mut()) };
        assert_eq!(result, 0.0);
        assert_eq!(get_errno(), ERANGE);
    }
}
