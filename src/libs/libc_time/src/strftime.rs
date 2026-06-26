// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Lint Configuration
//==================================================================================================

// Formatting inherently narrows wide arithmetic results back to the C ABI integer widths; on the
// 32-bit Nanvix target these casts are exact.
#![allow(clippy::cast_possible_truncation)]
#![allow(clippy::cast_possible_wrap)]

//==================================================================================================
// Imports
//==================================================================================================

use crate::tm_struct::{
    tm,
    DAY_NAMES,
    FULL_DAY_NAMES,
    FULL_MONTH_NAMES,
    MONTH_NAMES,
    TM_YEAR_BASE,
};
use ::sysapi::{
    ffi::c_char,
    sys_types::c_size_t,
};

//==================================================================================================
// Bounded Output Writer
//==================================================================================================

/// Bounded writer that appends bytes to the destination buffer, reserving space for the trailing
/// null byte and flagging overflow instead of writing past the end.
struct Writer {
    /// Destination buffer.
    buf: *mut c_char,
    /// Capacity of the destination buffer, including the trailing null byte.
    max: usize,
    /// Number of bytes written so far.
    pos: usize,
    /// Set once a write could not fit; all subsequent writes are suppressed.
    overflow: bool,
}

impl Writer {
    /// Appends a single byte, flagging overflow if there is no room left for it plus a final null.
    unsafe fn put(&mut self, byte: u8) {
        if self.overflow {
            return;
        }
        if self.pos + 1 >= self.max {
            self.overflow = true;
            return;
        }
        *self.buf.add(self.pos) = byte as c_char;
        self.pos += 1;
    }

    /// Appends a byte slice.
    unsafe fn put_bytes(&mut self, bytes: &[u8]) {
        for &b in bytes {
            self.put(b);
        }
    }

    /// Appends a non-negative integer with the given minimum field width and padding byte.
    unsafe fn put_uint(&mut self, value: u64, width: usize, pad: u8) {
        let mut digits: [u8; 20] = [0; 20];
        let mut count: usize = 0;
        let mut v: u64 = value;

        // Generate decimal digits least-significant first.
        loop {
            digits[count] = b'0' + (v % 10) as u8;
            count += 1;
            v /= 10;
            if v == 0 {
                break;
            }
        }

        // Left-pad to the requested width.
        let mut pad_count: usize = width.saturating_sub(count);
        while pad_count > 0 {
            self.put(pad);
            pad_count -= 1;
        }

        // Emit digits most-significant first.
        while count > 0 {
            count -= 1;
            self.put(digits[count]);
        }
    }

    /// Appends a signed integer (used for the four-or-more-digit year).
    unsafe fn put_int(&mut self, value: i64) {
        if value < 0 {
            self.put(b'-');
            self.put_uint(value.unsigned_abs(), 0, b'0');
        } else {
            self.put_uint(value as u64, 0, b'0');
        }
    }
}

//==================================================================================================
// Conversion Helpers
//==================================================================================================

/// Returns the 12-hour clock hour for the given 24-hour value, in `[1, 12]`.
fn hour12(hour24: i32) -> i32 {
    let h: i32 = hour24 % 12;
    if h == 0 {
        12
    } else {
        h
    }
}

/// Computes the week-of-year number for the `%U`/`%W` specifiers.
///
/// `week_start` is the weekday (`0` = Sunday) that begins each numbered week.
fn week_of_year(yday: i32, wday: i32, week_start: i32) -> i32 {
    let offset: i32 = (wday + 7 - week_start) % 7;
    (yday + 7 - offset) / 7
}

//==================================================================================================
// Core Formatter
//==================================================================================================

/// Expands `fmt` against the broken-down time `t`, writing the result through `w`.
unsafe fn run(w: &mut Writer, fmt: &[u8], t: &tm) {
    let mut i: usize = 0;
    while i < fmt.len() {
        let c: u8 = fmt[i];
        if c != b'%' {
            w.put(c);
            i += 1;
            continue;
        }

        i += 1;
        if i >= fmt.len() {
            // Trailing '%': emit verbatim.
            w.put(b'%');
            break;
        }

        // Skip the POSIX locale modifiers 'E' and 'O'; they select alternate
        // representations that collapse to the default in the C locale.
        let mut spec: u8 = fmt[i];
        if spec == b'E' || spec == b'O' {
            i += 1;
            if i >= fmt.len() {
                break;
            }
            spec = fmt[i];
        }

        emit_spec(w, spec, t);
        i += 1;
    }
}

/// Emits a single conversion specifier `spec`.
unsafe fn emit_spec(w: &mut Writer, spec: u8, t: &tm) {
    match spec {
        b'a' => {
            if let Some(name) = name_for(&DAY_NAMES, t.tm_wday, 7) {
                w.put_bytes(name);
            }
        },
        b'A' => {
            if (0..7).contains(&t.tm_wday) {
                w.put_bytes(FULL_DAY_NAMES[t.tm_wday as usize]);
            }
        },
        b'b' | b'h' => {
            if let Some(name) = name_for(&MONTH_NAMES, t.tm_mon, 12) {
                w.put_bytes(name);
            }
        },
        b'B' => {
            if (0..12).contains(&t.tm_mon) {
                w.put_bytes(FULL_MONTH_NAMES[t.tm_mon as usize]);
            }
        },
        b'c' => run(w, b"%a %b %e %H:%M:%S %Y", t),
        b'C' => w.put_uint(((t.tm_year as i64 + TM_YEAR_BASE) / 100) as u64, 2, b'0'),
        b'd' => w.put_uint(t.tm_mday as u64, 2, b'0'),
        b'D' => run(w, b"%m/%d/%y", t),
        b'e' => w.put_uint(t.tm_mday as u64, 2, b' '),
        b'F' => run(w, b"%Y-%m-%d", t),
        b'H' => w.put_uint(t.tm_hour as u64, 2, b'0'),
        b'I' => w.put_uint(hour12(t.tm_hour) as u64, 2, b'0'),
        b'j' => w.put_uint((t.tm_yday + 1) as u64, 3, b'0'),
        b'k' => w.put_uint(t.tm_hour as u64, 2, b' '),
        b'l' => w.put_uint(hour12(t.tm_hour) as u64, 2, b' '),
        b'm' => w.put_uint((t.tm_mon + 1) as u64, 2, b'0'),
        b'M' => w.put_uint(t.tm_min as u64, 2, b'0'),
        b'n' => w.put(b'\n'),
        b'p' => w.put_bytes(if t.tm_hour < 12 { b"AM" } else { b"PM" }),
        b'P' => w.put_bytes(if t.tm_hour < 12 { b"am" } else { b"pm" }),
        b'r' => run(w, b"%I:%M:%S %p", t),
        b'R' => run(w, b"%H:%M", t),
        b'S' => w.put_uint(t.tm_sec as u64, 2, b'0'),
        b't' => w.put(b'\t'),
        b'T' => run(w, b"%H:%M:%S", t),
        b'u' => {
            let iso: i32 = if t.tm_wday == 0 { 7 } else { t.tm_wday };
            w.put_uint(iso as u64, 0, b'0');
        },
        b'U' => w.put_uint(week_of_year(t.tm_yday, t.tm_wday, 0) as u64, 2, b'0'),
        b'w' => w.put_uint(t.tm_wday as u64, 0, b'0'),
        b'W' => w.put_uint(week_of_year(t.tm_yday, t.tm_wday, 1) as u64, 2, b'0'),
        b'y' => w.put_uint((t.tm_year as i64 + TM_YEAR_BASE).rem_euclid(100) as u64, 2, b'0'),
        b'Y' => w.put_int(t.tm_year as i64 + TM_YEAR_BASE),
        b'z' => w.put_bytes(b"+0000"),
        b'Z' => w.put_bytes(b"UTC"),
        b'%' => w.put(b'%'),
        // Unknown specifier: emit it verbatim, preceded by the percent sign.
        other => {
            w.put(b'%');
            w.put(other);
        },
    }
}

/// Returns the abbreviated name for `index` from `table` if `index` is within `[0, len)`.
fn name_for(table: &[&'static [u8; 3]], index: i32, len: i32) -> Option<&'static [u8]> {
    if index >= 0 && index < len {
        Some(&table[index as usize][..])
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
/// Formats the broken-down time `tm` according to the format string `format`, writing the result
/// (including a terminating null byte) into the buffer `s`.
///
/// The conversion specifiers follow POSIX in the C locale: `%a %A %b %B %c %C %d %D %e %F %H %I %j
/// %k %l %m %M %n %p %P %r %R %S %t %T %u %U %w %W %y %Y %z %Z %%`. The `E`/`O` locale modifiers
/// are accepted and ignored. Time zones are not modeled, so `%z`/`%Z` always render UTC.
///
/// # Parameters
///
/// - `s`: Destination buffer.
/// - `max`: Capacity of `s`, including space for the terminating null byte.
/// - `format`: Null-terminated format string.
/// - `timeptr`: Pointer to the broken-down time to format.
///
/// # Returns
///
/// The number of bytes written, excluding the terminating null byte. If the result (including the
/// null byte) does not fit within `max`, returns `0` and the buffer contents are unspecified.
///
/// # Safety
///
/// This function is unsafe because it dereferences raw pointers. The caller must ensure that `s`
/// has room for at least `max` bytes and that `format` and `timeptr` point to valid objects.
///
/// # References
///
/// - <https://pubs.opengroup.org/onlinepubs/9799919799/functions/strftime.html>
///
#[cfg_attr(not(feature = "std"), unsafe(no_mangle))]
pub unsafe extern "C" fn strftime(
    s: *mut c_char,
    max: c_size_t,
    format: *const c_char,
    timeptr: *const tm,
) -> c_size_t {
    if s.is_null() || format.is_null() || timeptr.is_null() {
        return 0;
    }
    if max == 0 {
        return 0;
    }

    let t: &tm = &*timeptr;

    // Collect the null-terminated format string into a byte slice.
    let mut len: usize = 0;
    while *format.add(len) != 0 {
        len += 1;
    }
    let fmt: &[u8] = core::slice::from_raw_parts(format.cast::<u8>(), len);

    let mut writer: Writer = Writer {
        buf: s,
        max: max as usize,
        pos: 0,
        overflow: false,
    };

    run(&mut writer, fmt, t);

    if writer.overflow {
        return 0;
    }

    // Terminate the string. `pos < max` holds because `put` reserves the final byte.
    *s.add(writer.pos) = 0;
    writer.pos as c_size_t
}

//==================================================================================================
// Tests
//==================================================================================================

#[cfg(all(test, feature = "std"))]
mod test {
    use super::strftime;
    use crate::tm_struct::{
        days_from_civil,
        tm,
    };
    use ::std::{
        ffi::CString,
        string::String,
        vec,
    };
    use ::sysapi::sys_types::c_size_t;

    /// Builds a broken-down time, deriving `tm_wday`/`tm_yday` from the civil date so the tests do
    /// not hard-code (and risk mis-stating) those fields.
    fn make_tm(year: i32, mon: i32, mday: i32, hour: i32, min: i32, sec: i32) -> tm {
        let mut t: tm = tm::new();
        t.tm_year = year - 1900;
        t.tm_mon = mon - 1;
        t.tm_mday = mday;
        t.tm_hour = hour;
        t.tm_min = min;
        t.tm_sec = sec;

        let days: i64 = days_from_civil(i64::from(year), i64::from(mon), i64::from(mday));
        // 1970-01-01 was a Thursday (weekday 4).
        t.tm_wday = (((days % 7) + 4) % 7 + 7) as i32 % 7;
        let jan1: i64 = days_from_civil(i64::from(year), 1, 1);
        t.tm_yday = (days - jan1) as i32;
        t
    }

    fn fmt(t: &tm, format: &str) -> String {
        let f: CString = CString::new(format).unwrap_or_else(|_| CString::default());
        let mut buf: vec::Vec<u8> = vec![0u8; 256];
        let n: c_size_t =
            unsafe { strftime(buf.as_mut_ptr().cast::<i8>(), 256 as c_size_t, f.as_ptr(), t) };
        String::from_utf8_lossy(&buf[..n as usize]).into_owned()
    }

    #[test]
    fn date_and_time() {
        let t: tm = make_tm(2021, 3, 14, 9, 5, 7);
        assert_eq!(fmt(&t, "%Y-%m-%d"), "2021-03-14");
        assert_eq!(fmt(&t, "%H:%M:%S"), "09:05:07");
        assert_eq!(fmt(&t, "%F %T"), "2021-03-14 09:05:07");
    }

    #[test]
    fn names() {
        let t: tm = make_tm(2021, 3, 14, 9, 5, 7);
        // 2021-03-14 is a Sunday.
        assert_eq!(fmt(&t, "%a"), "Sun");
        assert_eq!(fmt(&t, "%A"), "Sunday");
        assert_eq!(fmt(&t, "%b"), "Mar");
        assert_eq!(fmt(&t, "%B"), "March");
        assert_eq!(fmt(&t, "%h"), "Mar");
    }

    #[test]
    fn am_pm_and_12hour() {
        let am: tm = make_tm(2021, 3, 14, 9, 49, 8);
        assert_eq!(fmt(&am, "%I:%M:%S %p"), "09:49:08 AM");
        let pm: tm = make_tm(2021, 3, 14, 21, 49, 8);
        assert_eq!(fmt(&pm, "%I:%M:%S %p"), "09:49:08 PM");
        assert_eq!(fmt(&pm, "%H"), "21");
        assert_eq!(fmt(&pm, "%l"), " 9");
        assert_eq!(fmt(&pm, "%P"), "pm");
        let midnight: tm = make_tm(2021, 3, 14, 0, 0, 0);
        assert_eq!(fmt(&midnight, "%I %p"), "12 AM");
    }

    #[test]
    fn numeric_fields() {
        let t: tm = make_tm(2021, 3, 14, 9, 5, 7);
        assert_eq!(fmt(&t, "%j"), "073");
        assert_eq!(fmt(&t, "%y"), "21");
        assert_eq!(fmt(&t, "%C"), "20");
        assert_eq!(fmt(&t, "%w"), "0");
        assert_eq!(fmt(&t, "%u"), "7");
        assert_eq!(fmt(&t, "%e"), "14");
        let single: tm = make_tm(2021, 3, 3, 0, 0, 0);
        assert_eq!(fmt(&single, "%e"), " 3");
        assert_eq!(fmt(&single, "%d"), "03");
    }

    #[test]
    fn two_digit_year_stays_in_range() {
        // Years whose offset from `TM_YEAR_BASE` is negative must still wrap into `00..=99`.
        let mut t: tm = tm::new();
        t.tm_year = -1 - 1900;
        assert_eq!(fmt(&t, "%y"), "99");
    }

    #[test]
    fn literals_and_unknown() {
        let t: tm = make_tm(2021, 3, 14, 9, 5, 7);
        assert_eq!(fmt(&t, "100%% done"), "100% done");
        assert_eq!(fmt(&t, "a%nb%tc"), "a\nb\tc");
        assert_eq!(fmt(&t, "%z %Z"), "+0000 UTC");
    }

    #[test]
    fn truncation_returns_zero() {
        let t: tm = make_tm(2021, 3, 14, 9, 5, 7);
        let f: CString = CString::new("%Y").unwrap_or_else(|_| CString::default());
        let mut buf: vec::Vec<u8> = vec![0u8; 4];
        // "2021" needs 4 bytes plus a null terminator: it cannot fit in 4 bytes.
        let n: c_size_t =
            unsafe { strftime(buf.as_mut_ptr().cast::<i8>(), 4 as c_size_t, f.as_ptr(), &t) };
        assert_eq!(n, 0);
        // A five-byte buffer is exactly enough.
        let mut buf2: vec::Vec<u8> = vec![0u8; 5];
        let n2: c_size_t =
            unsafe { strftime(buf2.as_mut_ptr().cast::<i8>(), 5 as c_size_t, f.as_ptr(), &t) };
        assert_eq!(n2, 4);
    }

    #[test]
    fn zero_capacity_returns_zero() {
        let t: tm = make_tm(2021, 3, 14, 9, 5, 7);
        let f: CString = CString::new("").unwrap_or_else(|_| CString::default());
        let mut buf: [u8; 1] = *b"x";
        let n: c_size_t =
            unsafe { strftime(buf.as_mut_ptr().cast::<i8>(), 0 as c_size_t, f.as_ptr(), &t) };
        assert_eq!(n, 0);
        assert_eq!(buf[0], b'x');
    }
}
