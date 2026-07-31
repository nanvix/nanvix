// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Lint Configuration
//==================================================================================================

// C-interop formatting requires reinterpret casts between signed and unsigned integer types
// (e.g., c_char (i8) to u8, negative values to their unsigned bit-pattern).
#![allow(clippy::cast_sign_loss)]

//==================================================================================================
// Imports
//==================================================================================================

use ::sysapi::ffi::{
    c_char,
    c_int,
};

//==================================================================================================
// Types
//==================================================================================================

/// Flags parsed from a printf format specifier.
pub(crate) struct FormatFlags {
    /// Left-align the output within the field width (`-`).
    pub(crate) left_align: bool,
    /// Always print a sign character (`+`).
    pub(crate) force_sign: bool,
    /// Print a space before positive numbers (` `).
    pub(crate) space_sign: bool,
    /// Pad with zeros instead of spaces (`0`).
    pub(crate) zero_pad: bool,
    /// Use alternate form (`#`).
    pub(crate) alternate: bool,
}

/// Length modifier for printf format specifiers.
#[derive(Clone, Copy)]
enum LengthMod {
    /// No length modifier.
    None,
    /// `l` — long.
    Long,
    /// `ll` — long long.
    LongLong,
    /// `z` — size_t.
    SizeT,
}

//==================================================================================================
// Traits
//==================================================================================================

/// A write target for formatted output.
pub(crate) trait WriteTarget {
    /// Writes a single byte to the target. Returns `Ok(())` on success.
    fn write_byte(&mut self, b: u8) -> Result<(), ()>;

    /// Writes a slice of bytes to the target.
    fn write_bytes(&mut self, bytes: &[u8]) -> Result<(), ()> {
        for &b in bytes {
            self.write_byte(b)?;
        }
        Ok(())
    }

    /// Returns the total number of characters written (or that would have been written,
    /// ignoring any buffer bounds).
    fn total(&self) -> usize;
}

/// An argument source that provides typed values for format specifiers.
pub(crate) trait ArgSource {
    /// Fetches the next `c_int` argument.
    fn next_int(&mut self) -> c_int;
    /// Fetches the next `u32` argument.
    fn next_uint(&mut self) -> u32;
    /// Fetches the next `i32` (long) argument.
    fn next_long(&mut self) -> i32;
    /// Fetches the next `u32` (unsigned long) argument.
    fn next_ulong(&mut self) -> u32;
    /// Fetches the next `i64` (long long) argument.
    fn next_longlong(&mut self) -> i64;
    /// Fetches the next `u64` (unsigned long long) argument.
    fn next_ulonglong(&mut self) -> u64;
    /// Fetches the next `usize` (size_t) argument.
    fn next_size(&mut self) -> usize;
    /// Fetches the next pointer argument as `usize`.
    fn next_ptr(&mut self) -> usize;
    /// Fetches the next `*const c_char` string argument.
    fn next_str(&mut self) -> *const c_char;
    /// Fetches the next `f64` (double) argument. A `float` is promoted to `double` when passed
    /// through an ellipsis, so this also serves the `float` case.
    fn next_double(&mut self) -> f64;
}

//==================================================================================================
// Write Targets
//==================================================================================================

/// Writes formatted output into a bounded buffer (for snprintf/vsnprintf).
pub(crate) struct BufWriter {
    buf: *mut u8,
    /// Maximum buffer size (including space for null terminator).
    size: usize,
    /// Current write position in the buffer.
    pos: usize,
    /// Total characters that would be written (ignoring buffer bounds).
    total: usize,
}

impl BufWriter {
    /// Creates a new [`BufWriter`] targeting the given buffer.
    pub(crate) fn new(buf: *mut u8, size: usize) -> Self {
        Self {
            buf,
            size,
            pos: 0,
            total: 0,
        }
    }

    /// Writes a null terminator at the current position (or at end of buffer).
    pub(crate) fn null_terminate(&mut self) {
        if self.size > 0 {
            let idx: usize = if self.pos < self.size {
                self.pos
            } else {
                self.size - 1
            };
            // SAFETY: idx < self.size, so the write is within buffer bounds.
            unsafe {
                *self.buf.add(idx) = 0;
            }
        }
    }
}

impl WriteTarget for BufWriter {
    fn write_byte(&mut self, b: u8) -> Result<(), ()> {
        if self.pos < self.size.saturating_sub(1) {
            // SAFETY: pos < size - 1, so the write is within buffer bounds.
            unsafe {
                *self.buf.add(self.pos) = b;
            }
            self.pos += 1;
        }
        self.total += 1;
        Ok(())
    }

    fn total(&self) -> usize {
        self.total
    }
}

/// Writes formatted output to a file descriptor (for fprintf/vfprintf).
pub(crate) struct FdWriter {
    fd: c_int,
    written: usize,
    error: bool,
}

impl FdWriter {
    /// Creates a new [`FdWriter`] for the given file descriptor.
    pub(crate) fn new(fd: c_int) -> Self {
        Self {
            fd,
            written: 0,
            error: false,
        }
    }

    /// Returns the total number of bytes successfully written, or `-1` on error.
    pub(crate) fn result(&self) -> c_int {
        if self.error {
            -1
        } else {
            self.written as c_int
        }
    }
}

impl WriteTarget for FdWriter {
    fn write_byte(&mut self, b: u8) -> Result<(), ()> {
        if self.error {
            return Err(());
        }
        let buf: [u8; 1] = [b];
        // SAFETY: buf is a valid 1-byte buffer on the stack.
        let ret: isize = unsafe { write_fd(self.fd, buf.as_ptr(), 1) };
        // write() may legally return 0 (no progress) or a negative value (error); only a
        // return of exactly 1 means the byte was written.
        if ret != 1 {
            self.error = true;
            return Err(());
        }
        self.written += 1;
        Ok(())
    }

    fn write_bytes(&mut self, bytes: &[u8]) -> Result<(), ()> {
        if self.error {
            return Err(());
        }
        if bytes.is_empty() {
            return Ok(());
        }
        // write() may perform a short write, so loop until the whole buffer is flushed. A
        // return of 0 (no progress) or a negative value (error) stops the loop and marks
        // the stream as failed.
        let mut off: usize = 0;
        while off < bytes.len() {
            // SAFETY: bytes[off..] is a valid, non-empty slice.
            let ret: isize =
                unsafe { write_fd(self.fd, bytes.as_ptr().add(off), bytes.len() - off) };
            if ret <= 0 {
                self.error = true;
                return Err(());
            }
            off += ret as usize;
            self.written += ret as usize;
        }
        Ok(())
    }

    fn total(&self) -> usize {
        self.written
    }
}

/// Writes formatted output to an unbounded buffer (for sprintf/vsprintf).
pub(crate) struct UnboundedBufWriter {
    buf: *mut u8,
    /// Current write position in the buffer.
    pos: usize,
}

impl UnboundedBufWriter {
    /// Creates a new [`UnboundedBufWriter`] targeting the given buffer.
    pub(crate) fn new(buf: *mut u8) -> Self {
        Self { buf, pos: 0 }
    }

    /// Writes a null terminator at the current position.
    pub(crate) fn null_terminate(&mut self) {
        // SAFETY: caller guarantees the buffer is large enough.
        unsafe {
            *self.buf.add(self.pos) = 0;
        }
    }
}

impl WriteTarget for UnboundedBufWriter {
    fn write_byte(&mut self, b: u8) -> Result<(), ()> {
        // SAFETY: caller guarantees the buffer is large enough.
        unsafe {
            *self.buf.add(self.pos) = b;
        }
        self.pos += 1;
        Ok(())
    }

    fn total(&self) -> usize {
        self.pos
    }
}

//==================================================================================================
// Private Functions
//==================================================================================================

/// Platform write wrapper.
///
/// # Safety
///
/// `buf` must point to at least `len` valid bytes.
unsafe fn write_fd(fd: c_int, buf: *const u8, len: usize) -> isize {
    extern "C" {
        fn write(
            fd: c_int,
            buf: *const ::sysapi::ffi::c_void,
            count: ::sysapi::sys_types::c_size_t,
        ) -> ::sysapi::sys_types::c_ssize_t;
    }
    // SAFETY: caller guarantees buf/len validity.
    unsafe {
        write(fd, buf.cast::<::sysapi::ffi::c_void>(), len as ::sysapi::sys_types::c_size_t)
            as isize
    }
}

/// Reads a byte from a `*const c_char` pointer at the given offset.
///
/// # Safety
///
/// The caller must ensure that `fmt.add(pos)` points to a valid, readable byte.
unsafe fn read_byte(fmt: *const c_char, pos: usize) -> u8 {
    // SAFETY: caller guarantees pointer validity.
    crate::c_char_to_u8(unsafe { *fmt.add(pos) })
}

/// Parses format flags from the format string starting at `pos`.
/// Returns the parsed flags and the updated position.
fn parse_flags(fmt: *const c_char, start: usize) -> (FormatFlags, usize) {
    let mut flags: FormatFlags = FormatFlags {
        left_align: false,
        force_sign: false,
        space_sign: false,
        zero_pad: false,
        alternate: false,
    };
    let mut pos: usize = start;
    loop {
        // SAFETY: pos is within the format string (we stop at null terminator).
        let b: u8 = unsafe { read_byte(fmt, pos) };
        match b {
            b'-' => flags.left_align = true,
            b'+' => flags.force_sign = true,
            b' ' => flags.space_sign = true,
            b'0' => flags.zero_pad = true,
            b'#' => flags.alternate = true,
            _ => break,
        }
        pos += 1;
    }
    (flags, pos)
}

/// Parses a decimal integer from the format string starting at `pos`.
/// Returns the parsed value and the updated position.
fn parse_decimal(fmt: *const c_char, start: usize) -> (usize, usize) {
    let mut val: usize = 0;
    let mut pos: usize = start;
    loop {
        // SAFETY: pos is within the format string.
        let b: u8 = unsafe { read_byte(fmt, pos) };
        if b.is_ascii_digit() {
            val = val.wrapping_mul(10).wrapping_add((b - b'0') as usize);
            pos += 1;
        } else {
            break;
        }
    }
    (val, pos)
}

/// Parses the width field from the format string. Supports `*` (from args).
fn parse_width<A: ArgSource>(
    fmt: *const c_char,
    start: usize,
    args: &mut A,
) -> (usize, bool, usize) {
    // SAFETY: start is within the format string.
    let b: u8 = unsafe { read_byte(fmt, start) };
    if b == b'*' {
        let w: c_int = args.next_int();
        if w < 0 {
            // Negative width means left-align with absolute width.
            ((-w) as usize, true, start + 1)
        } else {
            (w as usize, false, start + 1)
        }
    } else if b.is_ascii_digit() {
        let (val, pos) = parse_decimal(fmt, start);
        (val, false, pos)
    } else {
        (0, false, start)
    }
}

/// Parses the precision field from the format string. Supports `*` (from args).
/// Returns (has_precision, precision_value, new_position).
fn parse_precision<A: ArgSource>(
    fmt: *const c_char,
    start: usize,
    args: &mut A,
) -> (bool, usize, usize) {
    // SAFETY: start is within the format string.
    let b: u8 = unsafe { read_byte(fmt, start) };
    if b != b'.' {
        return (false, 0, start);
    }
    let pos: usize = start + 1;
    // SAFETY: pos is within the format string.
    let b2: u8 = unsafe { read_byte(fmt, pos) };
    if b2 == b'*' {
        let p: c_int = args.next_int();
        if p < 0 {
            (false, 0, pos + 1)
        } else {
            (true, p as usize, pos + 1)
        }
    } else {
        let (val, new_pos) = parse_decimal(fmt, pos);
        (true, val, new_pos)
    }
}

/// Parses the length modifier from the format string.
fn parse_length(fmt: *const c_char, start: usize) -> (LengthMod, usize) {
    // SAFETY: start is within the format string.
    let b: u8 = unsafe { read_byte(fmt, start) };
    match b {
        b'l' => {
            // SAFETY: start + 1 is within the format string.
            let b2: u8 = unsafe { read_byte(fmt, start + 1) };
            if b2 == b'l' {
                (LengthMod::LongLong, start + 2)
            } else {
                (LengthMod::Long, start + 1)
            }
        },
        b'z' => (LengthMod::SizeT, start + 1),
        b'h' => {
            // Skip 'h' or 'hh' — treat as default (promoted to int).
            // SAFETY: start + 1 is within the format string.
            let b2: u8 = unsafe { read_byte(fmt, start + 1) };
            if b2 == b'h' {
                (LengthMod::None, start + 2)
            } else {
                (LengthMod::None, start + 1)
            }
        },
        _ => (LengthMod::None, start),
    }
}

/// Writes padding characters to the writer.
pub(crate) fn write_padding<W: WriteTarget>(
    writer: &mut W,
    ch: u8,
    count: usize,
) -> Result<(), ()> {
    // Emit padding in fixed-size chunks so that file-descriptor-backed writers do not incur
    // one syscall per padding byte for large widths/precisions.
    const CHUNK: usize = 64;
    let chunk: [u8; CHUNK] = [ch; CHUNK];
    let mut remaining: usize = count;
    while remaining > 0 {
        let n: usize = if remaining < CHUNK { remaining } else { CHUNK };
        writer.write_bytes(&chunk[..n])?;
        remaining -= n;
    }
    Ok(())
}

/// Formats an unsigned 64-bit integer into a stack buffer.
/// Returns the starting index and the number of digits in `buf`.
fn format_unsigned(value: u64, base: u64, uppercase: bool, buf: &mut [u8; 32]) -> (usize, usize) {
    let digits: &[u8; 16] = if uppercase {
        b"0123456789ABCDEF"
    } else {
        b"0123456789abcdef"
    };
    if value == 0 {
        buf[31] = b'0';
        return (31, 1);
    }
    let mut pos: usize = 32;
    let mut v: u64 = value;
    while v > 0 {
        pos -= 1;
        buf[pos] = digits[(v % base) as usize];
        v /= base;
    }
    (pos, 32 - pos)
}

/// Formats a signed integer specifier (%d / %i) and writes to the target.
fn format_signed<W: WriteTarget>(
    writer: &mut W,
    value: i64,
    flags: &FormatFlags,
    width: usize,
    has_precision: bool,
    precision: usize,
) -> Result<(), ()> {
    let negative: bool = value < 0;
    let abs_val: u64 = if negative {
        // Handle i64::MIN correctly: -(i64::MIN) overflows, use wrapping.
        (value.wrapping_neg()) as u64
    } else {
        value as u64
    };

    let mut buf: [u8; 32] = [0u8; 32];
    let (digit_start, digit_len) = format_unsigned(abs_val, 10, false, &mut buf);

    // Determine sign character.
    let sign: Option<u8> = if negative {
        Some(b'-')
    } else if flags.force_sign {
        Some(b'+')
    } else if flags.space_sign {
        Some(b' ')
    } else {
        Option::None
    };
    let sign_len: usize = if sign.is_some() { 1 } else { 0 };

    // Apply precision: minimum number of digits.
    let effective_digits: usize = if has_precision && precision > digit_len {
        precision
    } else {
        digit_len
    };
    let zero_prefix: usize = effective_digits - digit_len;

    let content_len: usize = sign_len + effective_digits;

    // Determine padding.
    let pad_len: usize = width.saturating_sub(content_len);

    // Zero-pad only when no precision is given and not left-aligned.
    let use_zero_pad: bool = flags.zero_pad && !flags.left_align && !has_precision;

    if flags.left_align {
        // Sign, zero-prefix, digits, then space padding.
        if let Some(s) = sign {
            writer.write_byte(s)?;
        }
        write_padding(writer, b'0', zero_prefix)?;
        writer.write_bytes(&buf[digit_start..digit_start + digit_len])?;
        write_padding(writer, b' ', pad_len)?;
    } else if use_zero_pad {
        // Sign, then zero padding, then digits.
        if let Some(s) = sign {
            writer.write_byte(s)?;
        }
        write_padding(writer, b'0', pad_len + zero_prefix)?;
        writer.write_bytes(&buf[digit_start..digit_start + digit_len])?;
    } else {
        // Space padding, sign, zero-prefix, digits.
        write_padding(writer, b' ', pad_len)?;
        if let Some(s) = sign {
            writer.write_byte(s)?;
        }
        write_padding(writer, b'0', zero_prefix)?;
        writer.write_bytes(&buf[digit_start..digit_start + digit_len])?;
    }

    Ok(())
}

/// Parameters for formatting an unsigned integer specifier.
struct UnsignedFmtParams<'a> {
    base: u64,
    uppercase: bool,
    flags: &'a FormatFlags,
    width: usize,
    has_precision: bool,
    precision: usize,
}

/// Formats an unsigned integer specifier (%u / %x / %X / %o) and writes to the target.
fn format_unsigned_spec<W: WriteTarget>(
    writer: &mut W,
    value: u64,
    params: &UnsignedFmtParams<'_>,
) -> Result<(), ()> {
    let mut buf: [u8; 32] = [0u8; 32];
    let (digit_start, digit_len) = format_unsigned(value, params.base, params.uppercase, &mut buf);

    // Alternate form prefix.
    let prefix: &[u8] = if params.flags.alternate && value != 0 {
        match params.base {
            16 => {
                if params.uppercase {
                    b"0X"
                } else {
                    b"0x"
                }
            },
            8 => b"0",
            _ => b"",
        }
    } else {
        b""
    };

    // Apply precision.
    let effective_digits: usize = if params.has_precision && params.precision > digit_len {
        params.precision
    } else if params.has_precision && params.precision == 0 && value == 0 {
        0
    } else {
        digit_len
    };
    let zero_prefix: usize = effective_digits.saturating_sub(digit_len);

    let content_len: usize = prefix.len() + effective_digits;
    let pad_len: usize = params.width.saturating_sub(content_len);

    let use_zero_pad: bool =
        params.flags.zero_pad && !params.flags.left_align && !params.has_precision;

    if params.flags.left_align {
        writer.write_bytes(prefix)?;
        write_padding(writer, b'0', zero_prefix)?;
        if effective_digits > 0 {
            writer.write_bytes(&buf[digit_start..digit_start + digit_len])?;
        }
        write_padding(writer, b' ', pad_len)?;
    } else if use_zero_pad {
        writer.write_bytes(prefix)?;
        write_padding(writer, b'0', pad_len + zero_prefix)?;
        if effective_digits > 0 {
            writer.write_bytes(&buf[digit_start..digit_start + digit_len])?;
        }
    } else {
        write_padding(writer, b' ', pad_len)?;
        writer.write_bytes(prefix)?;
        write_padding(writer, b'0', zero_prefix)?;
        if effective_digits > 0 {
            writer.write_bytes(&buf[digit_start..digit_start + digit_len])?;
        }
    }

    Ok(())
}

/// Formats a string specifier (%s) and writes to the target.
fn format_string<W: WriteTarget>(
    writer: &mut W,
    s: *const c_char,
    flags: &FormatFlags,
    width: usize,
    has_precision: bool,
    precision: usize,
) -> Result<(), ()> {
    let null_str: &[u8; 6] = b"(null)";

    // Compute string length.
    let (ptr, slen): (*const u8, usize) = if s.is_null() {
        (null_str.as_ptr(), null_str.len())
    } else {
        let mut len: usize = 0;
        // SAFETY: s is non-null; we walk until the null terminator.
        unsafe {
            while *s.add(len) != 0 {
                len += 1;
            }
        }
        (s.cast::<u8>(), len)
    };

    // Apply precision as maximum length.
    let print_len: usize = if has_precision && precision < slen {
        precision
    } else {
        slen
    };

    let pad_len: usize = width.saturating_sub(print_len);

    if flags.left_align {
        // SAFETY: ptr is valid for print_len bytes.
        let slice: &[u8] = unsafe { core::slice::from_raw_parts(ptr, print_len) };
        writer.write_bytes(slice)?;
        write_padding(writer, b' ', pad_len)?;
    } else {
        write_padding(writer, b' ', pad_len)?;
        // SAFETY: ptr is valid for print_len bytes.
        let slice: &[u8] = unsafe { core::slice::from_raw_parts(ptr, print_len) };
        writer.write_bytes(slice)?;
    }

    Ok(())
}

/// Formats a wide-string specifier (`%ls`) and writes to the target.
///
/// The argument is a pointer to an array of wide characters (`wchar_t`, i.e. `i32`). In the
/// single-byte C/POSIX locale each wide character maps to one output byte, so a wide character
/// outside `0..=0xff` is not representable and terminates the conversion.
fn format_wide_string<W: WriteTarget>(
    writer: &mut W,
    ws: *const i32,
    flags: &FormatFlags,
    width: usize,
    has_precision: bool,
    precision: usize,
) -> Result<(), ()> {
    // A null pointer is rendered exactly like the narrow `%s` conversion.
    if ws.is_null() {
        return format_string(writer, core::ptr::null(), flags, width, has_precision, precision);
    }

    // Count the leading representable wide characters, honoring any precision cap.
    let mut slen: usize = 0;
    loop {
        if has_precision && slen >= precision {
            break;
        }
        // SAFETY: ws is non-null and NUL-terminated, so `slen` stays within the string.
        let wc: i32 = unsafe { *ws.add(slen) };
        if wc == 0 || (wc as u32) > 0xff {
            break;
        }
        slen += 1;
    }

    let pad_len: usize = width.saturating_sub(slen);

    if !flags.left_align {
        write_padding(writer, b' ', pad_len)?;
    }
    for i in 0..slen {
        // SAFETY: `i < slen`, so the wide character is within the string and representable.
        let wc: i32 = unsafe { *ws.add(i) };
        writer.write_byte((wc as u32 & 0xff) as u8)?;
    }
    if flags.left_align {
        write_padding(writer, b' ', pad_len)?;
    }

    Ok(())
}

//==================================================================================================
// Standalone Functions
//==================================================================================================

///
/// # Description
///
/// Core formatting engine for printf-family functions. Parses the format string `fmt`, fetches
/// arguments from `args`, and writes formatted output to `writer`.
///
/// # Parameters
///
/// - `writer`: The output target implementing [`WriteTarget`].
/// - `fmt`: Pointer to a null-terminated printf format string.
/// - `args`: An argument source implementing [`ArgSource`].
///
/// # Returns
///
/// The total number of characters written (or that would have been written) as a [`c_int`].
/// Returns `-1` if `fmt` is null.
///
/// # Safety
///
/// The caller must ensure that `fmt` points to a valid, null-terminated format string and that
/// `args` provides arguments matching the format specifiers in `fmt`.
///
pub(crate) fn format_core<W: WriteTarget, A: ArgSource>(
    writer: &mut W,
    fmt: *const c_char,
    args: &mut A,
) -> c_int {
    if fmt.is_null() {
        return -1;
    }

    let mut pos: usize = 0;

    loop {
        // SAFETY: pos is within the format string.
        let b: u8 = unsafe { read_byte(fmt, pos) };

        if b == 0 {
            break;
        }

        if b != b'%' {
            let _ = writer.write_byte(b);
            pos += 1;
            continue;
        }

        // Skip the '%'.
        pos += 1;

        // Parse flags.
        let (mut flags, new_pos) = parse_flags(fmt, pos);
        pos = new_pos;

        // Parse width.
        let (width, width_neg, new_pos) = parse_width(fmt, pos, args);
        pos = new_pos;
        if width_neg {
            flags.left_align = true;
        }

        // Parse precision.
        let (has_precision, precision, new_pos) = parse_precision(fmt, pos, args);
        pos = new_pos;

        // Parse length modifier.
        let (length, new_pos) = parse_length(fmt, pos);
        pos = new_pos;

        // Parse conversion specifier.
        // SAFETY: pos is within the format string.
        let spec: u8 = unsafe { read_byte(fmt, pos) };
        if spec == 0 {
            break;
        }
        pos += 1;

        match spec {
            b'd' | b'i' => {
                let val: i64 = match length {
                    LengthMod::None => args.next_int() as i64,
                    LengthMod::Long => args.next_long() as i64,
                    LengthMod::LongLong => args.next_longlong(),
                    LengthMod::SizeT => args.next_size() as i64,
                };
                let _ = format_signed(writer, val, &flags, width, has_precision, precision);
            },
            b'u' => {
                let val: u64 = match length {
                    LengthMod::None => args.next_uint() as u64,
                    LengthMod::Long => args.next_ulong() as u64,
                    LengthMod::LongLong => args.next_ulonglong(),
                    LengthMod::SizeT => args.next_size() as u64,
                };
                let params: UnsignedFmtParams<'_> = UnsignedFmtParams {
                    base: 10,
                    uppercase: false,
                    flags: &flags,
                    width,
                    has_precision,
                    precision,
                };
                let _ = format_unsigned_spec(writer, val, &params);
            },
            b'x' | b'X' => {
                let upper: bool = spec == b'X';
                let val: u64 = match length {
                    LengthMod::None => args.next_uint() as u64,
                    LengthMod::Long => args.next_ulong() as u64,
                    LengthMod::LongLong => args.next_ulonglong(),
                    LengthMod::SizeT => args.next_size() as u64,
                };
                let params: UnsignedFmtParams<'_> = UnsignedFmtParams {
                    base: 16,
                    uppercase: upper,
                    flags: &flags,
                    width,
                    has_precision,
                    precision,
                };
                let _ = format_unsigned_spec(writer, val, &params);
            },
            b'o' => {
                let val: u64 = match length {
                    LengthMod::None => args.next_uint() as u64,
                    LengthMod::Long => args.next_ulong() as u64,
                    LengthMod::LongLong => args.next_ulonglong(),
                    LengthMod::SizeT => args.next_size() as u64,
                };
                let params: UnsignedFmtParams<'_> = UnsignedFmtParams {
                    base: 8,
                    uppercase: false,
                    flags: &flags,
                    width,
                    has_precision,
                    precision,
                };
                let _ = format_unsigned_spec(writer, val, &params);
            },
            b's' => {
                if matches!(length, LengthMod::Long) {
                    // `%ls` consumes a `wchar_t *`; render it as bytes for the C/POSIX locale.
                    let ws: *const i32 = args.next_str().cast::<i32>();
                    let _ = format_wide_string(writer, ws, &flags, width, has_precision, precision);
                } else {
                    let s: *const c_char = args.next_str();
                    let _ = format_string(writer, s, &flags, width, has_precision, precision);
                }
            },
            b'c' => {
                // `%lc` consumes a `wint_t`; a wide character outside `0..=0xff` is not
                // representable in the single-byte C/POSIX locale and yields no output byte.
                // Plain `%c` truncates its `int` argument to a byte as usual.
                let raw: c_int = args.next_int();
                let byte: Option<u8> = if matches!(length, LengthMod::Long) && (raw as u32) > 0xff {
                    Option::None
                } else {
                    Some(raw as u8)
                };
                let out_len: usize = if byte.is_some() { 1 } else { 0 };
                let pad_len: usize = width.saturating_sub(out_len);
                if !flags.left_align {
                    let _ = write_padding(writer, b' ', pad_len);
                }
                if let Some(b) = byte {
                    let _ = writer.write_byte(b);
                }
                if flags.left_align {
                    let _ = write_padding(writer, b' ', pad_len);
                }
            },
            b'p' => {
                let ptr: usize = args.next_ptr();
                let _ = writer.write_bytes(b"0x");
                let mut buf: [u8; 32] = [0u8; 32];
                let (start, len) = format_unsigned(ptr as u64, 16, false, &mut buf);
                let _ = writer.write_bytes(&buf[start..start + len]);
            },
            b'f' | b'F' | b'e' | b'E' | b'g' | b'G' => {
                let value: f64 = args.next_double();
                let _ = crate::float_fmt::format_float(
                    writer,
                    value,
                    spec,
                    &flags,
                    width,
                    has_precision,
                    precision,
                );
            },
            b'%' => {
                let _ = writer.write_byte(b'%');
            },
            b'n' => {
                // %n is intentionally not supported (security risk).
            },
            _ => {
                // Unknown specifier — write it literally.
                let _ = writer.write_byte(b'%');
                let _ = writer.write_byte(spec);
            },
        }
    }

    // Return the total number of characters written (or that would have been written).
    writer.total() as c_int
}

//==================================================================================================
// Tests
//==================================================================================================

#[cfg(all(test, feature = "std"))]
mod test {
    use super::*;
    use ::std::vec::Vec;

    /// Test write target that collects output into a Vec.
    struct VecWriter {
        data: Vec<u8>,
    }

    impl VecWriter {
        fn new() -> Self {
            Self { data: Vec::new() }
        }

        fn as_str(&self) -> &str {
            core::str::from_utf8(&self.data).expect("valid utf8")
        }
    }

    impl WriteTarget for VecWriter {
        fn write_byte(&mut self, b: u8) -> Result<(), ()> {
            self.data.push(b);
            Ok(())
        }

        fn total(&self) -> usize {
            self.data.len()
        }
    }

    /// Test argument source backed by slices.
    struct TestArgs {
        ints: Vec<c_int>,
        uints: Vec<u32>,
        longs: Vec<i32>,
        ulongs: Vec<u32>,
        longlongs: Vec<i64>,
        ulonglongs: Vec<u64>,
        sizes: Vec<usize>,
        ptrs: Vec<usize>,
        strs: Vec<*const c_char>,
        doubles: Vec<f64>,
        int_idx: usize,
        uint_idx: usize,
        long_idx: usize,
        ulong_idx: usize,
        longlong_idx: usize,
        ulonglong_idx: usize,
        size_idx: usize,
        ptr_idx: usize,
        str_idx: usize,
        double_idx: usize,
    }

    impl TestArgs {
        fn new() -> Self {
            Self {
                ints: Vec::new(),
                uints: Vec::new(),
                longs: Vec::new(),
                ulongs: Vec::new(),
                longlongs: Vec::new(),
                ulonglongs: Vec::new(),
                sizes: Vec::new(),
                ptrs: Vec::new(),
                strs: Vec::new(),
                doubles: Vec::new(),
                int_idx: 0,
                uint_idx: 0,
                long_idx: 0,
                ulong_idx: 0,
                longlong_idx: 0,
                ulonglong_idx: 0,
                size_idx: 0,
                ptr_idx: 0,
                str_idx: 0,
                double_idx: 0,
            }
        }
    }

    impl ArgSource for TestArgs {
        fn next_int(&mut self) -> c_int {
            let v: c_int = self.ints[self.int_idx];
            self.int_idx += 1;
            v
        }
        fn next_uint(&mut self) -> u32 {
            let v: u32 = self.uints[self.uint_idx];
            self.uint_idx += 1;
            v
        }
        fn next_long(&mut self) -> i32 {
            let v: i32 = self.longs[self.long_idx];
            self.long_idx += 1;
            v
        }
        fn next_ulong(&mut self) -> u32 {
            let v: u32 = self.ulongs[self.ulong_idx];
            self.ulong_idx += 1;
            v
        }
        fn next_longlong(&mut self) -> i64 {
            let v: i64 = self.longlongs[self.longlong_idx];
            self.longlong_idx += 1;
            v
        }
        fn next_ulonglong(&mut self) -> u64 {
            let v: u64 = self.ulonglongs[self.ulonglong_idx];
            self.ulonglong_idx += 1;
            v
        }
        fn next_size(&mut self) -> usize {
            let v: usize = self.sizes[self.size_idx];
            self.size_idx += 1;
            v
        }
        fn next_ptr(&mut self) -> usize {
            let v: usize = self.ptrs[self.ptr_idx];
            self.ptr_idx += 1;
            v
        }
        fn next_str(&mut self) -> *const c_char {
            let v: *const c_char = self.strs[self.str_idx];
            self.str_idx += 1;
            v
        }
        fn next_double(&mut self) -> f64 {
            let v: f64 = self.doubles[self.double_idx];
            self.double_idx += 1;
            v
        }
    }

    /// Helper: format with test args and return the output string.
    fn fmt(format: &[u8], args: &mut TestArgs) -> String {
        let mut writer: VecWriter = VecWriter::new();
        format_core(&mut writer, format.as_ptr().cast::<c_char>(), args);
        writer.as_str().to_string()
    }

    #[test]
    fn test_plain_string() {
        let mut args: TestArgs = TestArgs::new();
        assert_eq!(fmt(b"hello\0", &mut args), "hello");
    }

    #[test]
    fn test_percent_d() {
        let mut args: TestArgs = TestArgs::new();
        args.ints.push(42);
        assert_eq!(fmt(b"%d\0", &mut args), "42");
    }

    #[test]
    fn test_percent_d_negative() {
        let mut args: TestArgs = TestArgs::new();
        args.ints.push(-7);
        assert_eq!(fmt(b"%d\0", &mut args), "-7");
    }

    #[test]
    fn test_percent_u() {
        let mut args: TestArgs = TestArgs::new();
        args.uints.push(123);
        assert_eq!(fmt(b"%u\0", &mut args), "123");
    }

    #[test]
    fn test_percent_x() {
        let mut args: TestArgs = TestArgs::new();
        args.uints.push(255);
        assert_eq!(fmt(b"%x\0", &mut args), "ff");
    }

    #[test]
    fn test_percent_upper_x() {
        let mut args: TestArgs = TestArgs::new();
        args.uints.push(255);
        assert_eq!(fmt(b"%X\0", &mut args), "FF");
    }

    #[test]
    fn test_percent_o() {
        let mut args: TestArgs = TestArgs::new();
        args.uints.push(8);
        assert_eq!(fmt(b"%o\0", &mut args), "10");
    }

    #[test]
    fn test_percent_s() {
        let mut args: TestArgs = TestArgs::new();
        let s: &[u8] = b"world\0";
        args.strs.push(s.as_ptr().cast::<c_char>());
        assert_eq!(fmt(b"hello %s\0", &mut args), "hello world");
    }

    #[test]
    fn test_percent_s_null() {
        let mut args: TestArgs = TestArgs::new();
        args.strs.push(core::ptr::null());
        assert_eq!(fmt(b"%s\0", &mut args), "(null)");
    }

    #[test]
    fn test_percent_c() {
        let mut args: TestArgs = TestArgs::new();
        args.ints.push(b'A' as c_int);
        assert_eq!(fmt(b"%c\0", &mut args), "A");
    }

    #[test]
    fn test_percent_ls() {
        let mut args: TestArgs = TestArgs::new();
        // A wide string is an array of wchar_t (i32); %ls renders it as bytes in the C locale.
        let ws: [i32; 6] = [
            b'w' as i32,
            b'o' as i32,
            b'r' as i32,
            b'l' as i32,
            b'd' as i32,
            0,
        ];
        args.strs.push(ws.as_ptr().cast::<c_char>());
        assert_eq!(fmt(b"hi %ls\0", &mut args), "hi world");
    }

    #[test]
    fn test_percent_ls_null() {
        let mut args: TestArgs = TestArgs::new();
        args.strs.push(core::ptr::null());
        assert_eq!(fmt(b"%ls\0", &mut args), "(null)");
    }

    #[test]
    fn test_percent_ls_precision() {
        let mut args: TestArgs = TestArgs::new();
        let ws: [i32; 6] = [
            b'w' as i32,
            b'o' as i32,
            b'r' as i32,
            b'l' as i32,
            b'd' as i32,
            0,
        ];
        args.strs.push(ws.as_ptr().cast::<c_char>());
        assert_eq!(fmt(b"%.3ls\0", &mut args), "wor");
    }

    #[test]
    fn test_percent_lc() {
        let mut args: TestArgs = TestArgs::new();
        args.ints.push(b'A' as c_int);
        assert_eq!(fmt(b"%lc\0", &mut args), "A");
    }

    #[test]
    fn test_percent_percent() {
        let mut args: TestArgs = TestArgs::new();
        assert_eq!(fmt(b"100%%\0", &mut args), "100%");
    }

    #[test]
    fn test_width_right_align() {
        let mut args: TestArgs = TestArgs::new();
        args.ints.push(42);
        assert_eq!(fmt(b"%10d\0", &mut args), "        42");
    }

    #[test]
    fn test_width_left_align() {
        let mut args: TestArgs = TestArgs::new();
        args.ints.push(42);
        assert_eq!(fmt(b"%-10d\0", &mut args), "42        ");
    }

    #[test]
    fn test_zero_pad() {
        let mut args: TestArgs = TestArgs::new();
        args.ints.push(42);
        assert_eq!(fmt(b"%05d\0", &mut args), "00042");
    }

    #[test]
    fn test_plus_flag() {
        let mut args: TestArgs = TestArgs::new();
        args.ints.push(42);
        assert_eq!(fmt(b"%+d\0", &mut args), "+42");
    }

    #[test]
    fn test_space_flag() {
        let mut args: TestArgs = TestArgs::new();
        args.ints.push(42);
        assert_eq!(fmt(b"% d\0", &mut args), " 42");
    }

    #[test]
    fn test_alternate_hex() {
        let mut args: TestArgs = TestArgs::new();
        args.uints.push(255);
        assert_eq!(fmt(b"%#x\0", &mut args), "0xff");
    }

    #[test]
    fn test_string_precision() {
        let mut args: TestArgs = TestArgs::new();
        let s: &[u8] = b"hello world\0";
        args.strs.push(s.as_ptr().cast::<c_char>());
        assert_eq!(fmt(b"%.5s\0", &mut args), "hello");
    }

    #[test]
    fn test_string_width_right() {
        let mut args: TestArgs = TestArgs::new();
        let s: &[u8] = b"hi\0";
        args.strs.push(s.as_ptr().cast::<c_char>());
        assert_eq!(fmt(b"%10s\0", &mut args), "        hi");
    }

    #[test]
    fn test_long_long_d() {
        let mut args: TestArgs = TestArgs::new();
        args.longlongs.push(1234567890123_i64);
        assert_eq!(fmt(b"%lld\0", &mut args), "1234567890123");
    }

    #[test]
    fn test_pointer() {
        let mut args: TestArgs = TestArgs::new();
        args.ptrs.push(0xDEAD);
        assert_eq!(fmt(b"%p\0", &mut args), "0xdead");
    }

    #[test]
    fn test_zero_value_d() {
        let mut args: TestArgs = TestArgs::new();
        args.ints.push(0);
        assert_eq!(fmt(b"%d\0", &mut args), "0");
    }

    #[test]
    fn test_null_format() {
        let mut args: TestArgs = TestArgs::new();
        let mut writer: VecWriter = VecWriter::new();
        let ret: c_int = format_core(&mut writer, core::ptr::null(), &mut args);
        assert_eq!(ret, -1);
    }

    #[test]
    fn test_buf_writer_truncation() {
        let mut buf: [u8; 6] = [0xFF; 6];
        let mut writer: BufWriter = BufWriter::new(buf.as_mut_ptr(), 6);
        // Write "hello world" — should truncate to "hello".
        let data: &[u8] = b"hello world";
        for &b in data {
            let _ = writer.write_byte(b);
        }
        writer.null_terminate();
        assert_eq!(writer.total(), 11);
        assert_eq!(&buf, b"hello\0");
    }
}
