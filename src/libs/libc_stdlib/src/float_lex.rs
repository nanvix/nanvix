// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use ::sysapi::ffi::c_char;

//==================================================================================================
// Helpers
//==================================================================================================

/// Returns `true` if the given character is an ASCII whitespace character.
pub(crate) fn is_whitespace(c: c_char) -> bool {
    let byte: u8 = crate::c_char_to_u8(c);
    byte == b' ' || byte == b'\t' || byte == b'\n' || byte == b'\r' || byte == 0x0b || byte == 0x0c
}

/// Returns `true` if the given character is an ASCII digit.
pub(crate) fn is_digit(c: c_char) -> bool {
    crate::c_char_to_u8(c).is_ascii_digit()
}

/// Returns `true` if the given character is an ASCII hexadecimal digit.
pub(crate) fn is_hex_digit(c: c_char) -> bool {
    crate::c_char_to_u8(c).is_ascii_hexdigit()
}

/// Returns the numeric value of an ASCII digit character.
pub(crate) fn digit_val(c: c_char) -> u8 {
    crate::c_char_to_u8(c) - b'0'
}

/// Returns the numeric value of an ASCII hexadecimal digit character.
pub(crate) fn hex_digit_val(c: c_char) -> u8 {
    let byte: u8 = crate::c_char_to_u8(c);
    match byte {
        b'0'..=b'9' => byte - b'0',
        b'a'..=b'f' => byte - b'a' + 10,
        b'A'..=b'F' => byte - b'A' + 10,
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
pub(crate) unsafe fn match_keyword(p: *const c_char, kw: &[u8]) -> bool {
    let mut i: usize = 0;
    while i < kw.len() {
        let c: u8 = crate::c_char_to_u8(unsafe { *p.add(i) });
        if c == 0 || c.to_ascii_lowercase() != kw[i] {
            return false;
        }
        i += 1;
    }
    true
}
