// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::format_engine::{
    format_core,
    ArgSource,
    BufWriter,
    WriteTarget,
};
use ::core::ffi::VaList;
use ::sysapi::{
    ffi::{
        c_char,
        c_int,
    },
    sys_types::c_size_t,
};

//==================================================================================================
// VaList Adapter
//==================================================================================================

/// Wraps a [`VaList`] to implement [`ArgSource`].
struct VaListArgs<'a> {
    va: VaList<'a>,
}

impl<'a> ArgSource for VaListArgs<'a> {
    fn next_int(&mut self) -> c_int {
        // SAFETY: caller guarantees matching format specifiers and arguments.
        unsafe { self.va.next_arg::<c_int>() }
    }
    fn next_uint(&mut self) -> u32 {
        // SAFETY: caller guarantees matching format specifiers and arguments.
        unsafe { self.va.next_arg::<u32>() }
    }
    fn next_long(&mut self) -> i32 {
        // SAFETY: caller guarantees matching format specifiers and arguments.
        unsafe { self.va.next_arg::<i32>() }
    }
    fn next_ulong(&mut self) -> u32 {
        // SAFETY: caller guarantees matching format specifiers and arguments.
        unsafe { self.va.next_arg::<u32>() }
    }
    fn next_longlong(&mut self) -> i64 {
        // SAFETY: caller guarantees matching format specifiers and arguments.
        unsafe { self.va.next_arg::<i64>() }
    }
    fn next_ulonglong(&mut self) -> u64 {
        // SAFETY: caller guarantees matching format specifiers and arguments.
        unsafe { self.va.next_arg::<u64>() }
    }
    fn next_size(&mut self) -> usize {
        // SAFETY: caller guarantees matching format specifiers and arguments.
        unsafe { self.va.next_arg::<usize>() }
    }
    fn next_ptr(&mut self) -> usize {
        // SAFETY: caller guarantees matching format specifiers and arguments.
        unsafe { self.va.next_arg::<usize>() }
    }
    fn next_str(&mut self) -> *const c_char {
        // SAFETY: caller guarantees matching format specifiers and arguments.
        unsafe { self.va.next_arg::<*const c_char>() }
    }
    fn next_double(&mut self) -> f64 {
        // SAFETY: caller guarantees matching format specifiers and arguments.
        unsafe { self.va.next_arg::<f64>() }
    }
}

//==================================================================================================
// Standalone Functions
//==================================================================================================

///
/// # Description
///
/// Writes at most `size - 1` characters of formatted output to `buf`, followed by a null
/// terminator. If `size` is zero, nothing is written but the return value still reflects the
/// number of characters that would have been written.
///
/// # Parameters
///
/// - `buf`: Pointer to the destination buffer.
/// - `size`: Size of the destination buffer in bytes.
/// - `fmt`: Pointer to a null-terminated printf format string.
/// - `ap`: Variable argument list matching the format specifiers in `fmt`.
///
/// # Returns
///
/// The number of characters that would have been written (excluding the null terminator) had the
/// buffer been large enough. A return value of `size` or more indicates truncation.
///
/// # Safety
///
/// This function is unsafe because it dereferences raw pointers. The caller must ensure that:
/// - `buf` points to a writable buffer of at least `size` bytes (when `size > 0`).
/// - `fmt` points to a valid, null-terminated format string.
/// - `ap` provides arguments matching the format specifiers.
///
/// # References
///
/// - <https://pubs.opengroup.org/onlinepubs/9799919799/functions/vsnprintf.html>
///
#[cfg_attr(not(feature = "std"), unsafe(no_mangle))]
pub unsafe extern "C" fn vsnprintf(
    buf: *mut c_char,
    size: c_size_t,
    fmt: *const c_char,
    ap: VaList<'_>,
) -> c_int {
    let buf_size: usize = size as usize;
    let mut writer: BufWriter = BufWriter::new(buf.cast::<u8>(), buf_size);
    let mut args: VaListArgs<'_> = VaListArgs { va: ap };
    let ret: c_int = format_core(&mut writer, fmt, &mut args);
    if ret < 0 {
        return ret;
    }
    writer.null_terminate();
    writer.total() as c_int
}
