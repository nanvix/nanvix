// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use ::core::ffi::VaList;
use ::sysapi::ffi::{
    c_char,
    c_int,
    c_void,
};

//==================================================================================================
// Constants
//==================================================================================================

/// Return value indicating end-of-input before any conversion.
const EOF: c_int = -1;

//==================================================================================================
// Input cursor
//==================================================================================================

/// A read cursor over a null-terminated input string.
struct Cursor {
    p: *const u8,
    consumed: usize,
}

impl Cursor {
    /// Returns the current byte without advancing (0 at end of input).
    unsafe fn peek(&self) -> u8 {
        unsafe { *self.p }
    }

    /// Returns the current byte and advances the cursor.
    unsafe fn bump(&mut self) -> u8 {
        let c: u8 = unsafe { *self.p };
        if c != 0 {
            self.p = unsafe { self.p.add(1) };
            self.consumed += 1;
        }
        c
    }

    /// Skips ASCII whitespace.
    unsafe fn skip_ws(&mut self) {
        while is_space(unsafe { self.peek() }) {
            unsafe { self.bump() };
        }
    }
}

/// Returns true for ASCII whitespace.
fn is_space(b: u8) -> bool {
    matches!(b, b' ' | b'\t' | b'\n' | b'\r' | 0x0b | 0x0c)
}

/// Returns the numeric value of `b` as a digit in `base`, or `None`.
fn digit_value(b: u8, base: u32) -> Option<u32> {
    let v: u32 = match b {
        b'0'..=b'9' => u32::from(b - b'0'),
        b'a'..=b'z' => u32::from(b - b'a') + 10,
        b'A'..=b'Z' => u32::from(b - b'A') + 10,
        _ => return None,
    };
    if v < base {
        Some(v)
    } else {
        None
    }
}

//==================================================================================================
// Length modifiers
//==================================================================================================

#[derive(Clone, Copy, PartialEq, Eq)]
enum Length {
    Char,
    Short,
    Int,
    Long,
    LongLong,
    Size,
    /// Pointer storage for `%p`, writing through a `void **` argument.
    Pointer,
}

//==================================================================================================
// Integer storage
//==================================================================================================

/// Stores `value` into the variadic integer pointer of the given `length`.
unsafe fn store_int(ap: &mut VaList<'_>, length: Length, value: u64) {
    // SAFETY: caller guarantees a matching pointer argument.
    let ptr: *mut c_void = unsafe { ap.next_arg::<*mut c_void>() };
    if ptr.is_null() {
        return;
    }
    unsafe {
        match length {
            Length::Char => *ptr.cast::<i8>() = value as i8,
            Length::Short => *ptr.cast::<i16>() = value as i16,
            Length::Int => *ptr.cast::<i32>() = value as i32,
            Length::Long => *ptr.cast::<i32>() = value as i32,
            Length::LongLong => *ptr.cast::<i64>() = value as i64,
            Length::Size => *ptr.cast::<usize>() = value as usize,
            // `%p` writes through a `void **`, storing a real pointer value.
            Length::Pointer => *ptr.cast::<*mut c_void>() = (value as usize) as *mut c_void,
        }
    }
}

//==================================================================================================
// Core
//==================================================================================================

/// Parses one integer conversion from `cur` and optionally stores it.
///
/// Returns true on a successful match.
#[allow(clippy::too_many_arguments)]
unsafe fn convert_int(
    cur: &mut Cursor,
    ap: &mut VaList<'_>,
    base: u32,
    auto_base: bool,
    width: usize,
    length: Length,
    suppress: bool,
) -> bool {
    unsafe { cur.skip_ws() };
    let mut remaining: usize = if width == 0 { usize::MAX } else { width };

    let mut negative: bool = false;
    let c: u8 = unsafe { cur.peek() };
    if (c == b'+' || c == b'-') && remaining > 0 {
        negative = c == b'-';
        unsafe { cur.bump() };
        remaining -= 1;
    }

    let mut base: u32 = base;
    if auto_base {
        base = 10;
        if unsafe { cur.peek() } == b'0' && remaining > 0 {
            unsafe { cur.bump() };
            remaining -= 1;
            // A leading 0 selects octal; a "0x"/"0X" prefix selects hexadecimal, but only
            // when at least one hexadecimal digit follows it (matching strtol with base 0).
            base = 8;
            if (unsafe { cur.peek() } == b'x' || unsafe { cur.peek() } == b'X') && remaining > 0 {
                let save: *const u8 = cur.p;
                let save_consumed: usize = cur.consumed;
                unsafe { cur.bump() };
                if digit_value(unsafe { cur.peek() }, 16).is_some() {
                    remaining -= 1;
                    base = 16;
                } else {
                    // No hex digit after the prefix: keep the leading 0 as an octal value.
                    cur.p = save;
                    cur.consumed = save_consumed;
                }
            }
            // A leading 0 already counts as a digit; fall through to read more.
            return finish_int(cur, ap, base, negative, remaining, length, suppress, true);
        }
    } else if base == 16 && unsafe { cur.peek() } == b'0' && remaining >= 2 {
        // Optional 0x/0X prefix for hex, consumed only when a hex digit follows it.
        let save: *const u8 = cur.p;
        let save_consumed: usize = cur.consumed;
        unsafe { cur.bump() };
        if unsafe { cur.peek() } == b'x' || unsafe { cur.peek() } == b'X' {
            unsafe { cur.bump() };
            if digit_value(unsafe { cur.peek() }, 16).is_some() {
                remaining -= 2;
            } else {
                // No hex digit after the prefix: rewind so the leading 0 is read as a digit.
                cur.p = save;
                cur.consumed = save_consumed;
            }
        } else {
            cur.p = save;
            cur.consumed = save_consumed;
        }
    }

    unsafe { finish_int(cur, ap, base, negative, remaining, length, suppress, false) }
}

/// Reads digits and stores the resulting integer.
#[allow(clippy::too_many_arguments)]
unsafe fn finish_int(
    cur: &mut Cursor,
    ap: &mut VaList<'_>,
    base: u32,
    negative: bool,
    mut remaining: usize,
    length: Length,
    suppress: bool,
    seeded_zero: bool,
) -> bool {
    let mut value: u64 = 0;
    let mut any: bool = seeded_zero;
    while remaining > 0 {
        match digit_value(unsafe { cur.peek() }, base) {
            Some(d) => {
                value = value
                    .wrapping_mul(u64::from(base))
                    .wrapping_add(u64::from(d));
                any = true;
                unsafe { cur.bump() };
                remaining -= 1;
            },
            None => break,
        }
    }
    if !any {
        return false;
    }
    // Apply the sign for both signed and unsigned conversions: scanf accepts an optional
    // leading sign even for %u/%o/%x/%p, and a negative value wraps (matching strtoul).
    let stored: u64 = if negative {
        (value as i64).wrapping_neg() as u64
    } else {
        value
    };
    if !suppress {
        unsafe { store_int(ap, length, stored) };
    }
    true
}

/// Implementation of `vsscanf`.
unsafe fn scan(s: *const c_char, fmt: *const c_char, mut ap: VaList<'_>) -> c_int {
    if s.is_null() || fmt.is_null() {
        return EOF;
    }

    let mut cur: Cursor = Cursor {
        p: s.cast::<u8>(),
        consumed: 0,
    };
    let mut f: *const u8 = fmt.cast::<u8>();
    let mut count: c_int = 0;
    let mut matched_any: bool = false;

    loop {
        let fc: u8 = unsafe { *f };
        if fc == 0 {
            break;
        }

        if is_space(fc) {
            unsafe { cur.skip_ws() };
            f = unsafe { f.add(1) };
            continue;
        }

        if fc != b'%' {
            // Literal character must match.
            if unsafe { cur.peek() } != fc {
                // End-of-input before any conversion is an input failure (EOF); a
                // differing byte is a matching failure that simply stops scanning.
                if unsafe { cur.peek() } == 0 && !matched_any {
                    return EOF;
                }
                break;
            }
            unsafe { cur.bump() };
            f = unsafe { f.add(1) };
            continue;
        }

        // Parse a conversion specification.
        f = unsafe { f.add(1) };
        if unsafe { *f } == b'%' {
            // A literal `%%` matches a single '%' in the input without skipping leading
            // whitespace (it behaves like an ordinary character, not a conversion).
            if unsafe { cur.peek() } != b'%' {
                // End-of-input before any conversion is an input failure (EOF).
                if unsafe { cur.peek() } == 0 && !matched_any {
                    return EOF;
                }
                break;
            }
            unsafe { cur.bump() };
            f = unsafe { f.add(1) };
            continue;
        }

        let mut suppress: bool = false;
        if unsafe { *f } == b'*' {
            suppress = true;
            f = unsafe { f.add(1) };
        }

        // Field width.
        let mut width: usize = 0;
        while let Some(d) = digit_value(unsafe { *f }, 10) {
            width = width.wrapping_mul(10).wrapping_add(d as usize);
            f = unsafe { f.add(1) };
        }

        // Length modifier.
        let mut length: Length = Length::Int;
        match unsafe { *f } {
            b'h' => {
                f = unsafe { f.add(1) };
                if unsafe { *f } == b'h' {
                    length = Length::Char;
                    f = unsafe { f.add(1) };
                } else {
                    length = Length::Short;
                }
            },
            b'l' => {
                f = unsafe { f.add(1) };
                if unsafe { *f } == b'l' {
                    length = Length::LongLong;
                    f = unsafe { f.add(1) };
                } else {
                    length = Length::Long;
                }
            },
            b'L' | b'q' | b'j' => {
                length = Length::LongLong;
                f = unsafe { f.add(1) };
            },
            b'z' | b't' => {
                length = Length::Size;
                f = unsafe { f.add(1) };
            },
            _ => {},
        }

        let spec: u8 = unsafe { *f };
        if spec == 0 {
            break;
        }
        f = unsafe { f.add(1) };

        let ok: bool = match spec {
            b'd' => unsafe { convert_int(&mut cur, &mut ap, 10, false, width, length, suppress) },
            b'u' => unsafe { convert_int(&mut cur, &mut ap, 10, false, width, length, suppress) },
            b'i' => unsafe { convert_int(&mut cur, &mut ap, 10, true, width, length, suppress) },
            b'o' => unsafe { convert_int(&mut cur, &mut ap, 8, false, width, length, suppress) },
            b'x' | b'X' => unsafe {
                convert_int(&mut cur, &mut ap, 16, false, width, length, suppress)
            },
            b'p' => unsafe {
                convert_int(&mut cur, &mut ap, 16, false, width, Length::Pointer, suppress)
            },
            b'c' => unsafe { convert_char(&mut cur, &mut ap, width, suppress) },
            b's' => unsafe { convert_str(&mut cur, &mut ap, width, suppress) },
            b'n' => {
                if !suppress {
                    // %n stores the number of characters consumed so far, honoring the
                    // active length modifier (e.g. %hhn, %hn, %ln, %lln, %zn).
                    // SAFETY: caller supplied a pointer matching the length modifier.
                    unsafe { store_int(&mut ap, length, cur.consumed as u64) };
                }
                // %n consumes no input and never fails, so it always completes as a
                // conversion. Record that to prevent a later end-of-input from being
                // reported as EOF, but do not count it as an assignment.
                matched_any = true;
                continue;
            },
            _ => break,
        };

        if !ok {
            // On a matching failure, stop. If input is exhausted with no match
            // at all, report EOF.
            if unsafe { cur.peek() } == 0 && !matched_any {
                return EOF;
            }
            break;
        }
        matched_any = true;
        if !suppress {
            count += 1;
        }
    }

    count
}

/// Parses a `%c` conversion.
unsafe fn convert_char(
    cur: &mut Cursor,
    ap: &mut VaList<'_>,
    width: usize,
    suppress: bool,
) -> bool {
    let n: usize = if width == 0 { 1 } else { width };
    if unsafe { cur.peek() } == 0 {
        return false;
    }
    let dst: *mut c_char = if suppress {
        core::ptr::null_mut()
    } else {
        // SAFETY: caller supplied a matching char pointer.
        unsafe { ap.next_arg::<*mut c_void>() }.cast::<c_char>()
    };
    let mut i: usize = 0;
    while i < n {
        let c: u8 = unsafe { cur.peek() };
        if c == 0 {
            break;
        }
        unsafe { cur.bump() };
        if !dst.is_null() {
            unsafe { *dst.add(i) = c as c_char };
        }
        i += 1;
    }
    // A field width requires reading exactly that many characters; a short read because
    // the input ended early is a failure for the conversion.
    i == n
}

/// Parses a `%s` conversion.
unsafe fn convert_str(cur: &mut Cursor, ap: &mut VaList<'_>, width: usize, suppress: bool) -> bool {
    unsafe { cur.skip_ws() };
    if unsafe { cur.peek() } == 0 || is_space(unsafe { cur.peek() }) {
        return false;
    }
    let limit: usize = if width == 0 { usize::MAX } else { width };
    let dst: *mut c_char = if suppress {
        core::ptr::null_mut()
    } else {
        // SAFETY: caller supplied a matching char pointer.
        unsafe { ap.next_arg::<*mut c_void>() }.cast::<c_char>()
    };
    let mut i: usize = 0;
    while i < limit {
        let c: u8 = unsafe { cur.peek() };
        if c == 0 || is_space(c) {
            break;
        }
        unsafe { cur.bump() };
        if !dst.is_null() {
            unsafe { *dst.add(i) = c as c_char };
        }
        i += 1;
    }
    if !dst.is_null() {
        unsafe { *dst.add(i) = 0 };
    }
    i > 0
}

//==================================================================================================
// Public Functions
//==================================================================================================

///
/// # Description
///
/// Reads formatted input from the string `s` according to `fmt`, using the variadic argument list
/// `ap` for the storage locations.
///
/// # Returns
///
/// The number of input items successfully matched and assigned, or `EOF` if input ends before the
/// first conversion.
///
/// # Safety
///
/// This function is unsafe because it dereferences raw pointers. `s` and `fmt` must be valid,
/// null-terminated strings and `ap` must provide pointers matching the conversions in `fmt`.
///
#[cfg_attr(not(feature = "std"), unsafe(no_mangle))]
pub unsafe extern "C" fn vsscanf(s: *const c_char, fmt: *const c_char, ap: VaList<'_>) -> c_int {
    unsafe { scan(s, fmt, ap) }
}

//==================================================================================================
// Tests
//==================================================================================================

#[cfg(all(test, feature = "std"))]
mod test {
    use super::{
        digit_value,
        is_space,
        vsscanf,
        Cursor,
    };
    use ::std::vec::Vec;
    use ::sysapi::ffi::{
        c_char,
        c_int,
    };

    /// Variadic helper that forwards to [`vsscanf`], used to construct a real `VaList`.
    ///
    /// # Safety
    ///
    /// `s` and `fmt` must be valid null-terminated strings and the variadic pointers must match
    /// the conversions in `fmt`.
    unsafe extern "C" fn run(s: *const c_char, fmt: *const c_char, args: ...) -> c_int {
        unsafe { vsscanf(s, fmt, args) }
    }

    /// Collects the bytes of a null-terminated `c_char` buffer into a `Vec<u8>`.
    fn cstr_bytes(buf: &[c_char]) -> Vec<u8> {
        buf.iter()
            .take_while(|&&c| c != 0)
            .map(|&c| c as u8)
            .collect()
    }

    #[test]
    fn is_space_matches_ascii_whitespace() {
        for b in [b' ', b'\t', b'\n', b'\r', 0x0b, 0x0c] {
            assert!(is_space(b));
        }
        for b in [b'a', b'0', b'.', 0] {
            assert!(!is_space(b));
        }
    }

    #[test]
    fn digit_value_respects_base() {
        assert_eq!(digit_value(b'9', 10), Some(9));
        assert_eq!(digit_value(b'a', 16), Some(10));
        assert_eq!(digit_value(b'F', 16), Some(15));
        assert_eq!(digit_value(b'7', 8), Some(7));
        assert_eq!(digit_value(b'8', 8), None);
        assert_eq!(digit_value(b'g', 16), None);
    }

    #[test]
    fn cursor_skips_whitespace_and_bumps() {
        let data = c"  ab";
        let mut cur = Cursor {
            p: data.as_ptr().cast::<u8>(),
            consumed: 0,
        };
        // SAFETY: `cur` points at a valid null-terminated buffer.
        unsafe {
            cur.skip_ws();
            assert_eq!(cur.peek(), b'a');
            assert_eq!(cur.bump(), b'a');
            assert_eq!(cur.bump(), b'b');
            assert_eq!(cur.peek(), 0);
            // A bump at end-of-input does not advance past the terminator.
            assert_eq!(cur.bump(), 0);
        }
        // `consumed` counts every byte consumed, including the two skipped spaces.
        assert_eq!(cur.consumed, 4);
    }

    #[test]
    fn scan_two_decimals() {
        let mut a: c_int = 0;
        let mut b: c_int = 0;
        // SAFETY: both pointers are valid and match the `%d` conversions.
        let n = unsafe { run(c"12 34".as_ptr(), c"%d %d".as_ptr(), &raw mut a, &raw mut b) };
        assert_eq!(n, 2);
        assert_eq!(a, 12);
        assert_eq!(b, 34);
    }

    #[test]
    fn scan_negative_decimal() {
        let mut a: c_int = 0;
        // SAFETY: the pointer is valid and matches the `%d` conversion.
        let n = unsafe { run(c"-42".as_ptr(), c"%d".as_ptr(), &raw mut a) };
        assert_eq!(n, 1);
        assert_eq!(a, -42);
    }

    #[test]
    fn scan_hex_with_prefix() {
        let mut a: c_int = 0;
        // SAFETY: the pointer is valid and matches the `%x` conversion.
        let n = unsafe { run(c"0xff".as_ptr(), c"%x".as_ptr(), &raw mut a) };
        assert_eq!(n, 1);
        assert_eq!(a, 255);
    }

    #[test]
    fn scan_string() {
        let mut buf = [0 as c_char; 8];
        // SAFETY: the buffer is large enough for "hello" plus a null terminator.
        let n = unsafe { run(c"hello world".as_ptr(), c"%s".as_ptr(), buf.as_mut_ptr()) };
        assert_eq!(n, 1);
        assert_eq!(cstr_bytes(&buf), b"hello");
    }

    #[test]
    fn scan_respects_field_width() {
        let mut a: c_int = 0;
        let mut b: c_int = 0;
        // SAFETY: both pointers are valid and match the width-limited `%d` conversions.
        let n = unsafe { run(c"1234".as_ptr(), c"%2d%2d".as_ptr(), &raw mut a, &raw mut b) };
        assert_eq!(n, 2);
        assert_eq!(a, 12);
        assert_eq!(b, 34);
    }

    #[test]
    fn scan_suppresses_assignment() {
        let mut a: c_int = 0;
        // SAFETY: the single pointer matches the non-suppressed `%d` conversion.
        let n = unsafe { run(c"7 8".as_ptr(), c"%*d %d".as_ptr(), &raw mut a) };
        assert_eq!(n, 1);
        assert_eq!(a, 8);
    }

    #[test]
    fn scan_reports_eof_on_empty_input() {
        let mut a: c_int = 0;
        // SAFETY: the pointer is valid; no conversion will be performed.
        let n = unsafe { run(c"".as_ptr(), c"%d".as_ptr(), &raw mut a) };
        assert_eq!(n, -1);
    }

    #[test]
    fn scan_stops_at_matching_failure() {
        let mut a: c_int = 0;
        let mut b: c_int = 0;
        // SAFETY: both pointers are valid; the second conversion fails to match.
        let n = unsafe { run(c"12 xy".as_ptr(), c"%d %d".as_ptr(), &raw mut a, &raw mut b) };
        assert_eq!(n, 1);
        assert_eq!(a, 12);
    }
}
