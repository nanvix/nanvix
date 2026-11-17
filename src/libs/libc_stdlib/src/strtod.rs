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
    ffi::c_char,
};

/// Retains the binary significand plus guard bits; later digits contribute a sticky bit.
const MAX_HEX_DIGITS: u32 = 16;

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
    let mut significand: u64 = 0;
    let mut kept: u32 = 0;
    let mut exponent: i64 = 0;
    let mut sticky: bool = false;
    let mut fractional: bool = false;
    let mut parsed_digit: bool = false;

    loop {
        if is_hex_digit(*q) {
            let digit: u8 = hex_digit_val(*q);
            parsed_digit = true;
            if fractional {
                exponent = exponent.saturating_sub(4);
            }
            if significand != 0 || digit != 0 {
                if kept < MAX_HEX_DIGITS {
                    significand = (significand << 4) | u64::from(digit);
                    kept += 1;
                } else {
                    exponent = exponent.saturating_add(4);
                    sticky |= digit != 0;
                }
            }
        } else if *q as u8 == b'.' && !fractional {
            fractional = true;
        } else {
            break;
        }
        q = q.add(1);
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
            let mut exp: i64 = 0;
            while is_digit(*q) {
                exp = exp
                    .saturating_mul(10)
                    .saturating_add(i64::from(digit_val(*q)));
                q = q.add(1);
            }
            if exp_negative {
                exp = exp.saturating_neg();
            }
            exponent = exponent.saturating_add(exp);
        } else {
            q = exponent_marker;
        }
    }

    Some((q, hex_value(significand, exponent, sticky), significand != 0))
}

/// Rounds the retained hexadecimal bits once, at the final normal or subnormal precision.
fn hex_value(significand: u64, exponent: i64, sticky: bool) -> f64 {
    if significand == 0 {
        return 0.0;
    }
    let bits: u32 = u64::BITS - significand.leading_zeros();
    let mut top: i64 = exponent.saturating_add(i64::from(bits) - 1);
    if top >= i64::from(f64::MAX_EXP) {
        return f64::INFINITY;
    }
    let normal: bool = top >= i64::from(f64::MIN_EXP - 1);
    let shift: i64 = if normal {
        i64::from(bits) - i64::from(f64::MANTISSA_DIGITS)
    } else {
        (i64::from(f64::MIN_EXP) - i64::from(f64::MANTISSA_DIGITS)).saturating_sub(exponent)
    };
    if shift > i64::from(u64::BITS) {
        return 0.0;
    }
    let mut rounded: u64 = if shift > 0 {
        let (whole, remainder): (u64, u64) = if shift == i64::from(u64::BITS) {
            (0, significand)
        } else {
            (significand >> shift, significand & ((1u64 << shift) - 1))
        };
        let halfway: u64 = 1u64 << (shift - 1);
        whole
            + u64::from(remainder > halfway || (remainder == halfway && (sticky || whole & 1 != 0)))
    } else {
        significand << -shift
    };
    if !normal {
        return f64::from_bits(rounded);
    }
    if rounded == 1u64 << f64::MANTISSA_DIGITS {
        rounded >>= 1;
        top += 1;
    }
    if top >= i64::from(f64::MAX_EXP) {
        return f64::INFINITY;
    }
    let fraction_bits: u32 = f64::MANTISSA_DIGITS - 1;
    let biased: u64 = (top + i64::from(f64::MAX_EXP - 1)) as u64;
    f64::from_bits((biased << fraction_bits) | (rounded & ((1u64 << fraction_bits) - 1)))
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
    let mut parsed_nonzero_digit: bool = false;
    while is_digit(*p) {
        let digit: u8 = digit_val(*p);
        parsed_nonzero_digit |= digit != 0;
        p = p.add(1);
    }

    // Parse fractional part.
    if *p as u8 == b'.' {
        p = p.add(1);
        while is_digit(*p) {
            let digit: u8 = digit_val(*p);
            parsed_nonzero_digit |= digit != 0;
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

    // Parse exponent part.
    if *p as u8 == b'e' || *p as u8 == b'E' {
        let saved: *const c_char = p;
        p = p.add(1);

        if *p as u8 == b'+' || *p as u8 == b'-' {
            p = p.add(1);
        }

        if is_digit(*p) {
            while is_digit(*p) {
                p = p.add(1);
            }
        } else {
            // No digits after 'e'/'E', rewind.
            p = saved;
        }
    }

    // The scanned subject is ASCII and valid decimal syntax. Core's allocation-free parser
    // combines the significand and exponent before rounding, even for arbitrarily long inputs.
    let subject: &str = unsafe {
        core::str::from_utf8_unchecked(core::slice::from_raw_parts(
            start.cast::<u8>(),
            p.offset_from(start) as usize,
        ))
    };
    let mut result: f64 = match subject.parse::<f64>() {
        Ok(value) => value,
        Err(_) => {
            set_errno(EINVAL);
            if !endptr.is_null() {
                *endptr = nptr.cast_mut();
            }
            return 0.0;
        },
    };

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

#[cfg(all(test, feature = "std"))]
mod tests;
