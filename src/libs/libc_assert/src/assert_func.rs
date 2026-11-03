// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use ::sysapi::ffi::{
    c_char,
    c_int,
};
#[cfg(not(feature = "std"))]
use ::sysapi::{
    ffi::c_void,
    sys_types::{
        c_size_t,
        c_ssize_t,
    },
};

//==================================================================================================
// Constants
//==================================================================================================

/// Maximum number of decimal digits required to represent any [`u32`] value.
const U32_MAX_DIGITS: usize = 10;

//==================================================================================================
// External Functions
//==================================================================================================

#[cfg(not(feature = "std"))]
extern "C" {
    fn write(fd: c_int, buf: *const c_void, count: c_size_t) -> c_ssize_t;
    fn abort() -> !;
}

//==================================================================================================
// Output Sink
//==================================================================================================

/// A minimal byte sink used to emit assertion diagnostics.
trait Sink {
    /// Writes all bytes of `buf` to the sink on a best-effort basis.
    fn write(&mut self, buf: &[u8]);
}

/// Sink that writes to the guest C runtime's standard error stream (file descriptor 2).
#[cfg(not(feature = "std"))]
struct ErrSink;

#[cfg(not(feature = "std"))]
impl Sink for ErrSink {
    fn write(&mut self, buf: &[u8]) {
        let mut remaining: &[u8] = buf;
        while !remaining.is_empty() {
            let count: c_size_t = c_size_t::try_from(remaining.len()).unwrap_or(c_size_t::MAX);
            // SAFETY: `remaining` points to at least `count` valid bytes and 2 is stderr.
            let written: c_ssize_t =
                unsafe { write(2, remaining.as_ptr().cast::<c_void>(), count) };
            // Stop on error (negative) or a zero-length write to avoid spinning forever.
            let Ok(advanced) = usize::try_from(written) else {
                break;
            };
            if advanced == 0 {
                break;
            }
            match remaining.get(advanced..) {
                Some(rest) => remaining = rest,
                None => break,
            }
        }
    }
}

/// Sink that writes to the host standard error stream. Used for host-side unit tests.
#[cfg(feature = "std")]
struct ErrSink;

#[cfg(feature = "std")]
impl Sink for ErrSink {
    fn write(&mut self, buf: &[u8]) {
        use ::std::io::Write as _;
        // Best-effort: diagnostics are emitted on a failure path, so I/O errors are ignored.
        let _ = ::std::io::stderr().write_all(buf);
    }
}

//==================================================================================================
// Private Functions
//==================================================================================================

/// Returns the sink that assertion diagnostics are written to.
fn err_sink() -> ErrSink {
    ErrSink
}

/// Aborts the current process and never returns.
#[cfg(not(feature = "std"))]
fn do_abort() -> ! {
    // SAFETY: `abort()` is provided by the C runtime, has no preconditions, and never returns.
    unsafe { abort() }
}

/// Aborts the current process and never returns.
#[cfg(feature = "std")]
fn do_abort() -> ! {
    ::std::process::abort()
}

/// Formats `n` as decimal ASCII digits into `buf`, returning the populated trailing slice.
fn format_uint(mut n: u32, buf: &mut [u8; U32_MAX_DIGITS]) -> &[u8] {
    let mut i: usize = buf.len();
    loop {
        i -= 1;
        buf[i] = b'0' + (n % 10) as u8;
        n /= 10;
        if n == 0 {
            break;
        }
    }
    &buf[i..]
}

/// Writes `n` as decimal digits to `sink`.
fn write_uint<S: Sink>(sink: &mut S, n: u32) {
    let mut buf: [u8; U32_MAX_DIGITS] = [0; U32_MAX_DIGITS];
    sink.write(format_uint(n, &mut buf));
}

/// Writes `n` as a signed decimal integer to `sink`, preserving the sign of negative values.
fn write_int<S: Sink>(sink: &mut S, n: c_int) {
    if n < 0 {
        sink.write(b"-");
    }
    write_uint(sink, n.unsigned_abs());
}

/// Writes a null-terminated C string to `sink`. Does nothing if `s` is null.
///
/// # Safety
///
/// `s` must either be null or point to a valid null-terminated C string.
unsafe fn write_cstr<S: Sink>(sink: &mut S, s: *const c_char) {
    if s.is_null() {
        return;
    }
    let mut len: usize = 0;
    while *s.add(len) != 0 {
        len += 1;
    }
    if len > 0 {
        // SAFETY: `s` points to `len` valid bytes that precede the null terminator.
        let bytes: &[u8] = ::core::slice::from_raw_parts(s.cast::<u8>(), len);
        sink.write(bytes);
    }
}

/// Emits the diagnostic message for a failed `__assert_func` to `sink`, without aborting.
///
/// # Safety
///
/// `file`, `function`, and `expression` must each be null or point to a valid null-terminated C
/// string.
unsafe fn emit_assert_func<S: Sink>(
    sink: &mut S,
    file: *const c_char,
    line: c_int,
    function: *const c_char,
    expression: *const c_char,
) {
    // Format: "file:line: function: Assertion `expression' failed.\n"
    write_cstr(sink, file);
    sink.write(b":");
    write_int(sink, line);
    sink.write(b": ");
    write_cstr(sink, function);
    sink.write(b": Assertion `");
    write_cstr(sink, expression);
    sink.write(b"' failed.\n");
}

/// Emits the diagnostic message for a failed `__assert` to `sink`, without aborting.
///
/// # Safety
///
/// `file` and `expression` must each be null or point to a valid null-terminated C string.
unsafe fn emit_assert<S: Sink>(
    sink: &mut S,
    file: *const c_char,
    line: c_int,
    expression: *const c_char,
) {
    // Format: "file:line: Assertion `expression' failed.\n"
    write_cstr(sink, file);
    sink.write(b":");
    write_int(sink, line);
    sink.write(b": Assertion `");
    write_cstr(sink, expression);
    sink.write(b"' failed.\n");
}

//==================================================================================================
// Standalone Functions
//==================================================================================================

///
/// # Description
///
/// Called when a C `assert()` macro fails. Prints an assertion failure message to stderr and
/// aborts the process.
///
/// # Parameters
///
/// - `file`: The source file name where the assertion failed.
/// - `line`: The line number where the assertion failed.
/// - `function`: The function name where the assertion failed.
/// - `expression`: The failed assertion expression as a string.
///
/// # Safety
///
/// This function is unsafe because it dereferences raw pointers that must each be null or reference
/// a valid null-terminated C string.
///
#[cfg_attr(not(feature = "std"), unsafe(no_mangle))]
pub unsafe extern "C" fn __assert_func(
    file: *const c_char,
    line: c_int,
    function: *const c_char,
    expression: *const c_char,
) -> ! {
    emit_assert_func(&mut err_sink(), file, line, function, expression);
    do_abort()
}

///
/// # Description
///
/// Simplified assertion failure handler without function name. Prints an assertion failure message
/// to stderr and aborts the process.
///
/// # Parameters
///
/// - `file`: The source file name where the assertion failed.
/// - `line`: The line number where the assertion failed.
/// - `expression`: The failed assertion expression as a string.
///
/// # Safety
///
/// This function is unsafe because it dereferences raw pointers that must each be null or reference
/// a valid null-terminated C string.
///
#[cfg_attr(not(feature = "std"), unsafe(no_mangle))]
pub unsafe extern "C" fn __assert(
    file: *const c_char,
    line: c_int,
    expression: *const c_char,
) -> ! {
    emit_assert(&mut err_sink(), file, line, expression);
    do_abort()
}

//==================================================================================================
// Tests
//==================================================================================================

#[cfg(all(test, feature = "std"))]
mod tests;
