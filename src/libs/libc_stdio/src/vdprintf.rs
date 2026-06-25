// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::format_engine::{
    format_core,
    ArgSource,
    FdWriter,
};
use ::core::ffi::VaList;
use ::sysapi::ffi::{
    c_char,
    c_int,
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
/// Formats output according to `fmt` and `ap` and writes it directly to the file descriptor `fd`.
///
/// # Parameters
///
/// - `fd`: Destination file descriptor.
/// - `fmt`: Pointer to a null-terminated printf format string.
/// - `ap`: Variable argument list matching the format specifiers in `fmt`.
///
/// # Returns
///
/// The number of bytes written on success, or `-1` on error.
///
/// # Safety
///
/// This function is unsafe because it dereferences raw pointers. The caller must ensure that:
/// - `fd` is a valid, writable file descriptor.
/// - `fmt` points to a valid, null-terminated format string.
/// - `ap` provides arguments matching the format specifiers.
///
/// # References
///
/// - <https://pubs.opengroup.org/onlinepubs/9799919799/functions/vdprintf.html>
///
#[cfg_attr(not(feature = "std"), unsafe(no_mangle))]
pub unsafe extern "C" fn vdprintf(fd: c_int, fmt: *const c_char, ap: VaList<'_>) -> c_int {
    let mut writer: FdWriter = FdWriter::new(fd);
    let mut args: VaListArgs<'_> = VaListArgs { va: ap };
    let fmt_ret: c_int = format_core(&mut writer, fmt, &mut args);
    if fmt_ret < 0 {
        return fmt_ret;
    }
    writer.result()
}
