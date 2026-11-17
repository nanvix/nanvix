// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

mod float;

//==================================================================================================
// Imports
//==================================================================================================

use crate::{
    fgetc::fgetc,
    ungetc::ungetc,
    EOF,
    FILE,
};
use ::core::ffi::VaList;
use ::sysapi::ffi::{
    c_char,
    c_int,
    c_long,
    c_void,
};

//==================================================================================================
// Input cursor
//==================================================================================================

enum Input {
    String(*const u8),
    Stream(*mut FILE),
}

/// One-byte lookahead shared by string and stream conversions.
struct Cursor {
    input: Input,
    lookahead: Option<c_int>,
    consumed: usize,
}

impl Cursor {
    fn new(input: Input) -> Self {
        Self {
            input,
            lookahead: None,
            consumed: 0,
        }
    }

    /// Returns the current byte without consuming it.
    unsafe fn peek(&mut self) -> Option<u8> {
        let c: c_int = match self.lookahead {
            Some(c) => c,
            None => {
                let c: c_int = match &mut self.input {
                    Input::String(p) => {
                        let byte: u8 = unsafe { **p };
                        if byte == 0 {
                            EOF
                        } else {
                            *p = unsafe { p.add(1) };
                            c_int::from(byte)
                        }
                    },
                    Input::Stream(stream) => unsafe { fgetc(*stream) },
                };
                self.lookahead = Some(c);
                c
            },
        };
        if c == EOF {
            None
        } else {
            Some(c as u8)
        }
    }

    /// Returns the current byte and advances the cursor.
    unsafe fn bump(&mut self) -> Option<u8> {
        let c: Option<u8> = unsafe { self.peek() };
        if c.is_some() {
            self.lookahead = None;
            self.consumed += 1;
        }
        c
    }

    /// Skips ASCII whitespace.
    unsafe fn skip_ws(&mut self) {
        while unsafe { self.peek() }.is_some_and(is_space) {
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
            Length::Long => *ptr.cast::<c_long>() = value as c_long,
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
    let mut remaining: usize = if width == 0 { usize::MAX } else { width };

    let mut negative: bool = false;
    let c: Option<u8> = unsafe { cur.peek() };
    if matches!(c, Some(b'+' | b'-')) {
        negative = c == Some(b'-');
        unsafe { cur.bump() };
        remaining -= 1;
    }

    let mut base: u32 = if auto_base { 10 } else { base };
    let mut seeded_zero: bool = false;
    if (auto_base || base == 16) && remaining > 0 && unsafe { cur.peek() } == Some(b'0') {
        unsafe { cur.bump() };
        remaining -= 1;
        seeded_zero = true;
        if auto_base {
            base = 8;
        }
        if remaining > 0 && matches!(unsafe { cur.peek() }, Some(b'x' | b'X')) {
            unsafe { cur.bump() };
            remaining -= 1;
            base = 16;
            // scanf consumes an incomplete input item rather than rewinding like strtol.
            seeded_zero = false;
        }
    }

    unsafe { finish_int(cur, ap, base, negative, remaining, length, suppress, seeded_zero) }
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
        match unsafe { cur.peek() }.and_then(|c| digit_value(c, base)) {
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

/// Shared formatted-input engine.
unsafe fn scan(cur: &mut Cursor, fmt: *const c_char, mut ap: VaList<'_>) -> c_int {
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
            if unsafe { cur.peek() } != Some(fc) {
                // End-of-input before any conversion is an input failure (EOF); a
                // differing byte is a matching failure that simply stops scanning.
                if unsafe { cur.peek() }.is_none() && !matched_any {
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
            if unsafe { cur.peek() } != Some(b'%') {
                // End-of-input before any conversion is an input failure (EOF).
                if unsafe { cur.peek() }.is_none() && !matched_any {
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
            width = width.saturating_mul(10).saturating_add(d as usize);
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

        if !matches!(spec, b'c' | b'[' | b'n') {
            unsafe { cur.skip_ws() };
        }
        if spec != b'n' && unsafe { cur.peek() }.is_none() {
            return if matched_any { count } else { EOF };
        }

        let ok: bool = match spec {
            b'd' | b'u' => unsafe { convert_int(cur, &mut ap, 10, false, width, length, suppress) },
            b'i' => unsafe { convert_int(cur, &mut ap, 10, true, width, length, suppress) },
            b'o' => unsafe { convert_int(cur, &mut ap, 8, false, width, length, suppress) },
            b'x' | b'X' => unsafe { convert_int(cur, &mut ap, 16, false, width, length, suppress) },
            b'p' => unsafe {
                convert_int(cur, &mut ap, 16, false, width, Length::Pointer, suppress)
            },
            b'c' => unsafe { convert_char(cur, &mut ap, width, suppress) },
            b's' => unsafe { convert_str(cur, &mut ap, width, suppress) },
            b'[' => unsafe { convert_set(cur, &mut ap, &mut f, width, suppress) },
            b'a' | b'A' | b'e' | b'E' | b'f' | b'F' | b'g' | b'G' => {
                match unsafe { float::convert(cur, &mut ap, width, length, suppress) } {
                    Ok(matched) => matched,
                    Err(error) => {
                        unsafe { *::sysapi::errno::__errno_location() = error };
                        return if matched_any { count } else { EOF };
                    },
                }
            },
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
            // A short %c read is an input failure; incomplete numeric items are mismatches.
            if spec == b'c' && !matched_any {
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
    let dst: *mut c_char = if suppress {
        core::ptr::null_mut()
    } else {
        // SAFETY: caller supplied a matching char pointer.
        unsafe { ap.next_arg::<*mut c_void>() }.cast::<c_char>()
    };
    let mut i: usize = 0;
    while i < n {
        let Some(c) = (unsafe { cur.peek() }) else {
            break;
        };
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
    unsafe { convert_bytes(cur, ap, width, suppress, |c| !is_space(c)) }
}

/// Reads a non-empty byte sequence and appends its terminating null byte.
unsafe fn convert_bytes(
    cur: &mut Cursor,
    ap: &mut VaList<'_>,
    width: usize,
    suppress: bool,
    accepts: impl Fn(u8) -> bool,
) -> bool {
    if !unsafe { cur.peek() }.is_some_and(&accepts) {
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
        let Some(c) = (unsafe { cur.peek() }) else {
            break;
        };
        if !accepts(c) {
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

/// Parses a scanset, including negation, a leading ']', and ascending byte ranges.
unsafe fn convert_set(
    cur: &mut Cursor,
    ap: &mut VaList<'_>,
    fmt: &mut *const u8,
    width: usize,
    suppress: bool,
) -> bool {
    let mut f: *const u8 = *fmt;
    let inverted: bool = unsafe { *f } == b'^';
    if inverted {
        f = unsafe { f.add(1) };
    }
    let mut set: [bool; 256] = [false; 256];
    if unsafe { *f } == b']' {
        set[usize::from(b']')] = true;
        f = unsafe { f.add(1) };
    }
    while unsafe { *f } != 0 && unsafe { *f } != b']' {
        let start: u8 = unsafe { *f };
        set[usize::from(start)] = true;
        f = unsafe { f.add(1) };
        if unsafe { *f } == b'-' {
            let end: u8 = unsafe { *f.add(1) };
            if end != 0 && end != b']' && start <= end {
                set[usize::from(start)..=usize::from(end)].fill(true);
                f = unsafe { f.add(2) };
            }
        }
    }
    if unsafe { *f } != b']' {
        return false;
    }
    *fmt = unsafe { f.add(1) };
    unsafe { convert_bytes(cur, ap, width, suppress, |c| set[usize::from(c)] != inverted) }
}

/// Scans a stream and returns its single lookahead byte to the stream on every exit path.
pub(crate) unsafe fn scan_stream(stream: *mut FILE, fmt: *const c_char, ap: VaList<'_>) -> c_int {
    let mut cur: Cursor = Cursor::new(Input::Stream(stream));
    let ret: c_int = unsafe { scan(&mut cur, fmt, ap) };
    if let Some(c) = cur.lookahead {
        if c != EOF && unsafe { ungetc(c, stream) } == EOF {
            unsafe { (*stream).error = 1 };
            return if ret == 0 { EOF } else { ret };
        }
    }
    ret
}

//==================================================================================================
// Public Functions
//==================================================================================================

///
/// # Description
///
/// Reads formatted input from the string `s` according to `fmt`, using the variadic argument list
/// `ap` for the storage locations.
/// Supports integer, floating-point, string, character, scanset, and byte-count conversions.
/// Floating-point items use `strtod` precision, including when stored as C `long double`.
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
    if s.is_null() || fmt.is_null() {
        return EOF;
    }
    let mut cur: Cursor = Cursor::new(Input::String(s.cast::<u8>()));
    unsafe { scan(&mut cur, fmt, ap) }
}

//==================================================================================================
// Tests
//==================================================================================================

#[cfg(all(test, feature = "std"))]
mod test;
