// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::tm_struct::{
    tm,
    FULL_DAY_NAMES,
    FULL_MONTH_NAMES,
};
use ::sysapi::ffi::{
    c_char,
    c_int,
};

//==================================================================================================
// Constants
//==================================================================================================

/// Abbreviated weekday names, indexed by `tm_wday` (`0` = Sunday).
const DAY_ABBR: [&[u8]; 7] = [b"Sun", b"Mon", b"Tue", b"Wed", b"Thu", b"Fri", b"Sat"];

/// Abbreviated month names, indexed by `tm_mon` (`0` = January).
const MON_ABBR: [&[u8]; 12] = [
    b"Jan", b"Feb", b"Mar", b"Apr", b"May", b"Jun", b"Jul", b"Aug", b"Sep", b"Oct", b"Nov", b"Dec",
];

//==================================================================================================
// Private Standalone Functions
//==================================================================================================

/// Returns `true` if `b` is an ASCII whitespace character.
fn is_space(b: u8) -> bool {
    matches!(b, b' ' | b'\t' | b'\n' | b'\r' | 0x0b | 0x0c)
}

/// Reads the byte pointed to by `p`.
///
/// The caller must ensure that `p` points to a valid, readable byte.
unsafe fn peek(p: *const c_char) -> u8 {
    *p.cast::<u8>()
}

/// Returns `true` if the string at `s` begins with `name`, compared case-insensitively.
///
/// The caller must ensure that `s` points to a valid NUL-terminated string.
unsafe fn starts_with_ci(s: *const c_char, name: &[u8]) -> bool {
    let mut k: usize = 0;
    while k < name.len() {
        let sb: u8 = peek(s.add(k));
        if sb == 0 || !sb.eq_ignore_ascii_case(&name[k]) {
            return false;
        }
        k += 1;
    }
    true
}

/// Searches `names` for the first entry that prefixes the string at `s` (case-insensitive).
///
/// Returns the matching index and the length of the matched name. The caller must ensure that `s`
/// points to a valid NUL-terminated string.
unsafe fn find_name(s: *const c_char, names: &[&[u8]]) -> Option<(c_int, usize)> {
    for (idx, name) in names.iter().enumerate() {
        if starts_with_ci(s, name) {
            return Some((c_int::try_from(idx).unwrap_or(0), name.len()));
        }
    }
    None
}

/// Matches the string at `s` against the full names first, then the abbreviated ones.
///
/// The caller must ensure that `s` points to a valid NUL-terminated string.
unsafe fn match_name(s: *const c_char, full: &[&[u8]], abbr: &[&[u8]]) -> Option<(c_int, usize)> {
    if let Some(found) = find_name(s, full) {
        return Some(found);
    }
    find_name(s, abbr)
}

/// Parses up to `maxw` decimal digits, skipping any leading ASCII whitespace.
///
/// Returns the parsed value together with the pointer positioned just past the last digit, or
/// `None` if no digit was found. The caller must ensure that `s` points to a valid NUL-terminated
/// string.
unsafe fn read_num(mut s: *const c_char, maxw: u32) -> Option<(c_int, *const c_char)> {
    while is_space(peek(s)) {
        s = s.add(1);
    }
    let mut val: c_int = 0;
    let mut count: u32 = 0;
    while count < maxw {
        let b: u8 = peek(s);
        if !b.is_ascii_digit() {
            break;
        }
        val = val * 10 + c_int::from(b - b'0');
        s = s.add(1);
        count += 1;
    }
    if count == 0 {
        None
    } else {
        Some((val, s))
    }
}

//==================================================================================================
// Standalone Functions
//==================================================================================================

///
/// # Description
///
/// Parses the string `s` according to the format string `format`, storing the recognized fields in
/// the broken-down time structure pointed to by `tm`. The supported conversion specifiers mirror
/// the common POSIX/glibc set: `%a %A %b %B %h %c %C %d %e %D %F %H %I %j %m %M %n %p %r %R %S %t
/// %T %U %w %W %x %X %y %Y %%`. The `E` and `O` locale modifiers are accepted and ignored.
///
/// Only the fields named by the format are modified; the caller is expected to initialize `tm`.
///
/// # Parameters
///
/// - `s`: Pointer to the input string to parse.
/// - `format`: Pointer to the format string.
/// - `tm`: Pointer to the broken-down time structure to populate.
///
/// # Returns
///
/// On success, returns a pointer to the first character in `s` that was not consumed. On failure
/// (a mismatch or a null argument), returns a null pointer.
///
/// # Safety
///
/// This function is unsafe because it dereferences the raw pointers `s`, `format`, and `tm`. The
/// caller must ensure that `s` and `format` point to valid NUL-terminated strings and that `tm`
/// points to a valid, writable `tm` structure.
///
#[cfg_attr(not(feature = "std"), unsafe(no_mangle))]
pub unsafe extern "C" fn strptime(
    s: *const c_char,
    format: *const c_char,
    tm: *mut tm,
) -> *mut c_char {
    if s.is_null() || format.is_null() || tm.is_null() {
        return core::ptr::null_mut();
    }

    let mut s: *const c_char = s;
    let mut f: *const c_char = format;

    // Deferred year reconstruction for the %C / %y specifiers.
    let mut century: c_int = -1;
    let mut year2: c_int = -1;
    let mut am_pm: Option<bool> = None;
    let mut hour12_active: bool = false;

    loop {
        let fc: u8 = peek(f);
        if fc == 0 {
            break;
        }

        if is_space(fc) {
            while is_space(peek(s)) {
                s = s.add(1);
            }
            f = f.add(1);
            continue;
        }

        if fc != b'%' {
            if peek(s) != fc {
                return core::ptr::null_mut();
            }
            s = s.add(1);
            f = f.add(1);
            continue;
        }

        // Consume the '%' and any locale modifier.
        f = f.add(1);
        let mut spec: u8 = peek(f);
        if spec == b'E' || spec == b'O' {
            f = f.add(1);
            spec = peek(f);
        }
        if spec == 0 {
            break;
        }

        match spec {
            b'%' => {
                if peek(s) != b'%' {
                    return core::ptr::null_mut();
                }
                s = s.add(1);
            },
            b'n' | b't' => {
                while is_space(peek(s)) {
                    s = s.add(1);
                }
            },
            b'a' | b'A' => match match_name(s, &FULL_DAY_NAMES, &DAY_ABBR) {
                Some((idx, len)) => {
                    (*tm).tm_wday = idx;
                    s = s.add(len);
                },
                None => return core::ptr::null_mut(),
            },
            b'b' | b'B' | b'h' => match match_name(s, &FULL_MONTH_NAMES, &MON_ABBR) {
                Some((idx, len)) => {
                    (*tm).tm_mon = idx;
                    s = s.add(len);
                },
                None => return core::ptr::null_mut(),
            },
            b'd' | b'e' => match read_num(s, 2) {
                Some((v, ns)) if (1..=31).contains(&v) => {
                    (*tm).tm_mday = v;
                    s = ns;
                },
                _ => return core::ptr::null_mut(),
            },
            b'H' => match read_num(s, 2) {
                Some((v, ns)) if (0..=23).contains(&v) => {
                    (*tm).tm_hour = v;
                    hour12_active = false;
                    s = ns;
                },
                _ => return core::ptr::null_mut(),
            },
            b'I' => match read_num(s, 2) {
                Some((v, ns)) if (1..=12).contains(&v) => {
                    (*tm).tm_hour = v;
                    hour12_active = true;
                    s = ns;
                },
                _ => return core::ptr::null_mut(),
            },
            b'm' => match read_num(s, 2) {
                Some((v, ns)) if (1..=12).contains(&v) => {
                    (*tm).tm_mon = v - 1;
                    s = ns;
                },
                _ => return core::ptr::null_mut(),
            },
            b'M' => match read_num(s, 2) {
                Some((v, ns)) if (0..=59).contains(&v) => {
                    (*tm).tm_min = v;
                    s = ns;
                },
                _ => return core::ptr::null_mut(),
            },
            b'S' => match read_num(s, 2) {
                // A value of 60 is accepted to allow for a leap second.
                Some((v, ns)) if (0..=60).contains(&v) => {
                    (*tm).tm_sec = v;
                    s = ns;
                },
                _ => return core::ptr::null_mut(),
            },
            b'j' => match read_num(s, 3) {
                Some((v, ns)) if (1..=366).contains(&v) => {
                    (*tm).tm_yday = v - 1;
                    s = ns;
                },
                _ => return core::ptr::null_mut(),
            },
            b'w' => match read_num(s, 1) {
                Some((v, ns)) if (0..=6).contains(&v) => {
                    (*tm).tm_wday = v;
                    s = ns;
                },
                _ => return core::ptr::null_mut(),
            },
            b'U' | b'W' => match read_num(s, 2) {
                Some((v, ns)) if (0..=53).contains(&v) => s = ns,
                _ => return core::ptr::null_mut(),
            },
            b'C' => match read_num(s, 2) {
                Some((v, ns)) => {
                    century = v;
                    s = ns;
                },
                None => return core::ptr::null_mut(),
            },
            b'y' => match read_num(s, 2) {
                Some((v, ns)) => {
                    year2 = v;
                    s = ns;
                },
                None => return core::ptr::null_mut(),
            },
            b'Y' => match read_num(s, 4) {
                Some((v, ns)) => {
                    (*tm).tm_year = v - 1900;
                    century = -1;
                    year2 = -1;
                    s = ns;
                },
                None => return core::ptr::null_mut(),
            },
            b'p' | b'P' => {
                if starts_with_ci(s, b"AM") {
                    am_pm = Some(false);
                    s = s.add(2);
                } else if starts_with_ci(s, b"PM") {
                    am_pm = Some(true);
                    s = s.add(2);
                } else {
                    return core::ptr::null_mut();
                }
            },
            b'D' | b'x' => {
                let r: *mut c_char = strptime(s, c"%m/%d/%y".as_ptr(), tm);
                if r.is_null() {
                    return core::ptr::null_mut();
                }
                s = r.cast_const();
            },
            b'F' => {
                let r: *mut c_char = strptime(s, c"%Y-%m-%d".as_ptr(), tm);
                if r.is_null() {
                    return core::ptr::null_mut();
                }
                s = r.cast_const();
            },
            b'R' => {
                let r: *mut c_char = strptime(s, c"%H:%M".as_ptr(), tm);
                if r.is_null() {
                    return core::ptr::null_mut();
                }
                s = r.cast_const();
                hour12_active = false;
            },
            b'T' | b'X' => {
                let r: *mut c_char = strptime(s, c"%H:%M:%S".as_ptr(), tm);
                if r.is_null() {
                    return core::ptr::null_mut();
                }
                s = r.cast_const();
                hour12_active = false;
            },
            b'r' => {
                let r: *mut c_char = strptime(s, c"%I:%M:%S %p".as_ptr(), tm);
                if r.is_null() {
                    return core::ptr::null_mut();
                }
                s = r.cast_const();
            },
            b'c' => {
                let r: *mut c_char = strptime(s, c"%a %b %e %H:%M:%S %Y".as_ptr(), tm);
                if r.is_null() {
                    return core::ptr::null_mut();
                }
                s = r.cast_const();
                hour12_active = false;
            },
            _ => return core::ptr::null_mut(),
        }

        f = f.add(1);
    }

    if hour12_active {
        match am_pm {
            Some(false) if (*tm).tm_hour == 12 => (*tm).tm_hour = 0,
            Some(true) if (*tm).tm_hour < 12 => (*tm).tm_hour += 12,
            _ => {},
        }
    }

    // Reconstruct the year from the century and/or two-digit year, if either was seen.
    if century >= 0 && year2 >= 0 {
        (*tm).tm_year = century * 100 + year2 - 1900;
    } else if year2 >= 0 {
        (*tm).tm_year = if year2 < 69 { year2 + 100 } else { year2 };
    } else if century >= 0 {
        (*tm).tm_year = century * 100 - 1900;
    }

    s.cast_mut()
}

//==================================================================================================
// Unit Tests
//==================================================================================================

#[cfg(all(test, feature = "std"))]
mod test;
