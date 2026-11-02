// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::Mntent;
use ::sysapi::ffi::{
    c_char,
    c_int,
};

//==================================================================================================
// Constants
//==================================================================================================

/// ASCII `#`, compared in `i32` space to avoid `c_char` sign-dependent casts.
const HASH: i32 = b'#' as i32;
/// ASCII `-`.
const MINUS: i32 = b'-' as i32;
/// ASCII `,`.
const COMMA: i32 = b',' as i32;
/// ASCII `=`.
const EQUALS: i32 = b'=' as i32;
/// ASCII `0`.
const ZERO: i32 = b'0' as i32;
/// ASCII `7`, the largest octal digit.
const SEVEN: i32 = b'7' as i32;
/// ASCII `9`.
const NINE: i32 = b'9' as i32;
/// ASCII `\`.
const BACKSLASH: i32 = b'\\' as i32;

//==================================================================================================
// Internal Helpers
//==================================================================================================

/// Returns `true` if `c` is an ASCII blank or line-separator character.
fn is_space(c: c_char) -> bool {
    matches!(i32::from(c), 0x20 | 0x09 | 0x0a | 0x0d | 0x0b | 0x0c)
}

/// Returns `true` if `c` is an ASCII octal digit (`0`–`7`).
fn is_octal(c: c_char) -> bool {
    (ZERO..=SEVEN).contains(&i32::from(c))
}

/// Decodes glibc-style octal escapes (`\NNN`) in the null-terminated string `s`, in place.
///
/// A backslash followed by exactly three octal digits encoding a byte value (`0..=255`) is replaced
/// by that byte; any other backslash is kept verbatim. This mirrors how `mount(8)`/glibc store
/// whitespace and backslashes in `mnt_*` fields (e.g. a space as `\040`). Decoding never lengthens
/// the string, so the rewrite always stays within the original buffer.
///
/// # Safety
///
/// `s` must be a writable, null-terminated C string.
unsafe fn unescape_in_place(s: *mut c_char) {
    let mut read: usize = 0;
    let mut write: usize = 0;
    loop {
        let c: c_char = *s.add(read);
        if c == 0 {
            break;
        }
        // A backslash followed by three octal digits is decoded to a single byte. The `&&`
        // short-circuits before reading past the terminator, since a null byte is not octal.
        if i32::from(c) == BACKSLASH
            && is_octal(*s.add(read + 1))
            && is_octal(*s.add(read + 2))
            && is_octal(*s.add(read + 3))
        {
            let d0: i32 = i32::from(*s.add(read + 1)) - ZERO;
            let d1: i32 = i32::from(*s.add(read + 2)) - ZERO;
            let d2: i32 = i32::from(*s.add(read + 3)) - ZERO;
            let value: i32 = d0 * 64 + d1 * 8 + d2;
            if let Ok(byte) = u8::try_from(value) {
                *s.add(write) = c_char::from_ne_bytes(byte.to_ne_bytes());
                write += 1;
                read += 4;
                continue;
            }
        }
        *s.add(write) = c;
        write += 1;
        read += 1;
    }
    *s.add(write) = 0;
}

/// Returns the length of the null-terminated C string `s`, excluding the terminator.
///
/// # Safety
///
/// `s` must be a valid null-terminated C string.
unsafe fn c_strlen(s: *const c_char) -> usize {
    let mut n: usize = 0;
    while *s.add(n) != 0 {
        n += 1;
    }
    n
}

/// Returns `true` if the first `n` bytes at `a` and `b` are equal.
///
/// # Safety
///
/// `a` and `b` must be readable for at least `n` bytes.
unsafe fn bytes_equal(a: *const c_char, b: *const c_char, n: usize) -> bool {
    let mut i: usize = 0;
    while i < n {
        if *a.add(i) != *b.add(i) {
            return false;
        }
        i += 1;
    }
    true
}

/// Extracts the next whitespace-delimited token from `*p`, null-terminating it in place and
/// advancing `*p` past it.
///
/// # Return Value
///
/// A pointer to the token, or null if no token remains.
///
/// # Safety
///
/// `*p` must point into a writable, null-terminated buffer.
unsafe fn take_token(p: &mut *mut c_char) -> *mut c_char {
    while is_space(**p) {
        *p = (*p).add(1);
    }
    if **p == 0 {
        return core::ptr::null_mut();
    }
    let start: *mut c_char = *p;
    while **p != 0 && !is_space(**p) {
        *p = (*p).add(1);
    }
    if **p != 0 {
        **p = 0;
        *p = (*p).add(1);
    }
    start
}

/// Extracts the next token and parses it as a base-10 integer, defaulting to `0` when the token is
/// absent or malformed.
///
/// # Safety
///
/// `*p` must point into a writable, null-terminated buffer.
unsafe fn take_int(p: &mut *mut c_char) -> c_int {
    let token: *mut c_char = take_token(p);
    if token.is_null() {
        return 0;
    }

    let mut index: usize = 0;
    let negative: bool = i32::from(*token) == MINUS;
    if negative {
        index = 1;
    }

    // Accumulate the magnitude in `i64` so that `i32::MIN` — whose magnitude exceeds `c_int::MAX`,
    // and which `write_int` is able to emit — round-trips correctly. Saturate, then clamp into the
    // `c_int` range.
    let mut magnitude: i64 = 0;
    loop {
        let digit: i32 = i32::from(*token.add(index));
        if !(ZERO..=NINE).contains(&digit) {
            break;
        }
        magnitude = magnitude
            .saturating_mul(10)
            .saturating_add(i64::from(digit - ZERO));
        index += 1;
    }

    let signed: i64 = if negative { -magnitude } else { magnitude };
    let clamped: i64 = signed.clamp(i64::from(c_int::MIN), i64::from(c_int::MAX));
    c_int::try_from(clamped).unwrap_or(0)
}

//==================================================================================================
// Crate-Visible Functions
//==================================================================================================

/// Parses one `fstab`/`mtab` line in place, filling `ent` with pointers into `line`.
///
/// # Return Value
///
/// `true` when a populated entry was parsed, `false` for blank or comment lines or lines that lack
/// the four mandatory string fields.
///
/// # Safety
///
/// `line` must be a writable, null-terminated buffer and `ent` must be valid for writes.
pub(crate) unsafe fn parse_line(line: *mut c_char, ent: *mut Mntent) -> bool {
    let mut p: *mut c_char = line;

    // Detect blank and comment lines.
    while is_space(*p) {
        p = p.add(1);
    }
    if *p == 0 || i32::from(*p) == HASH {
        return false;
    }

    let fsname: *mut c_char = take_token(&mut p);
    let dir: *mut c_char = take_token(&mut p);
    let typ: *mut c_char = take_token(&mut p);
    let opts: *mut c_char = take_token(&mut p);
    if fsname.is_null() || dir.is_null() || typ.is_null() || opts.is_null() {
        return false;
    }

    // Decode glibc-style octal escapes in the four string fields (e.g. `\040` -> space). This is
    // safe to do after tokenizing because escaped whitespace contains no literal separator, so the
    // fields were not split on it.
    unescape_in_place(fsname);
    unescape_in_place(dir);
    unescape_in_place(typ);
    unescape_in_place(opts);

    let freq: c_int = take_int(&mut p);
    let passno: c_int = take_int(&mut p);

    (*ent).mnt_fsname = fsname;
    (*ent).mnt_dir = dir;
    (*ent).mnt_type = typ;
    (*ent).mnt_opts = opts;
    (*ent).mnt_freq = freq;
    (*ent).mnt_passno = passno;
    true
}

/// Searches the comma-separated option list `opts` for an option named `opt`.
///
/// # Return Value
///
/// A pointer to the matching option within `opts`, or null when not present.
///
/// # Safety
///
/// `opts` and `opt` must be null or valid null-terminated C strings.
pub(crate) unsafe fn option_search(opts: *const c_char, opt: *const c_char) -> *mut c_char {
    if opts.is_null() || opt.is_null() {
        return core::ptr::null_mut();
    }
    let optlen: usize = c_strlen(opt);
    if optlen == 0 {
        return core::ptr::null_mut();
    }

    let mut p: *const c_char = opts;
    loop {
        // Skip separators.
        while i32::from(*p) == COMMA {
            p = p.add(1);
        }
        if *p == 0 {
            return core::ptr::null_mut();
        }

        let token: *const c_char = p;
        while *p != 0 && i32::from(*p) != COMMA {
            p = p.add(1);
        }
        let tokenlen: usize = usize::try_from(p.offset_from(token)).unwrap_or(0);

        // Match the option name, requiring a whole-token (or `name=value`) match.
        if tokenlen >= optlen && bytes_equal(token, opt, optlen) {
            let trailing: i32 = i32::from(*token.add(optlen));
            if trailing == 0 || trailing == COMMA || trailing == EQUALS {
                return token.cast_mut();
            }
        }

        if *p == 0 {
            return core::ptr::null_mut();
        }
    }
}

//==================================================================================================
// Tests
//==================================================================================================

#[cfg(all(test, feature = "std"))]
mod test;
