// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::{
    format_engine::{
        format_core,
        ArgSource,
        FdWriter,
    },
    streams::FILE,
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
        unsafe { self.va.next_arg::<c_int>() }
    }
    fn next_uint(&mut self) -> u32 {
        unsafe { self.va.next_arg::<u32>() }
    }
    fn next_long(&mut self) -> i32 {
        unsafe { self.va.next_arg::<i32>() }
    }
    fn next_ulong(&mut self) -> u32 {
        unsafe { self.va.next_arg::<u32>() }
    }
    fn next_longlong(&mut self) -> i64 {
        unsafe { self.va.next_arg::<i64>() }
    }
    fn next_ulonglong(&mut self) -> u64 {
        unsafe { self.va.next_arg::<u64>() }
    }
    fn next_size(&mut self) -> usize {
        unsafe { self.va.next_arg::<usize>() }
    }
    fn next_ptr(&mut self) -> usize {
        unsafe { self.va.next_arg::<usize>() }
    }
    fn next_str(&mut self) -> *const c_char {
        unsafe { self.va.next_arg::<*const c_char>() }
    }
    fn next_double(&mut self) -> f64 {
        unsafe { self.va.next_arg::<f64>() }
    }
}

//==================================================================================================
// Standalone Functions
//==================================================================================================

///
/// # Description
///
/// Writes formatted output to the given file stream.
///
/// # Parameters
///
/// - `stream`: Pointer to the target [`FILE`] stream.
/// - `fmt`: Pointer to a null-terminated printf format string.
/// - `ap`: Variable argument list matching the format specifiers in `fmt`.
///
/// # Returns
///
/// The number of characters written on success, or `-1` on error.
///
/// # Safety
///
/// This function is unsafe because it dereferences raw pointers. The caller must ensure that:
/// - `stream` points to a valid, open [`FILE`] structure.
/// - `fmt` points to a valid, null-terminated format string.
/// - `ap` provides arguments matching the format specifiers.
///
/// # References
///
/// - <https://pubs.opengroup.org/onlinepubs/9799919799/functions/vfprintf.html>
///
#[cfg_attr(not(feature = "std"), unsafe(no_mangle))]
pub unsafe extern "C" fn vfprintf(stream: *mut FILE, fmt: *const c_char, ap: VaList<'_>) -> c_int {
    if stream.is_null() {
        return -1;
    }
    let fd: c_int = (*stream).fd;
    let mut writer: FdWriter = FdWriter::new(fd);
    let mut args: VaListArgs<'_> = VaListArgs { va: ap };
    let fmt_ret: c_int = format_core(&mut writer, fmt, &mut args);
    let ret: c_int = if fmt_ret < 0 {
        fmt_ret
    } else {
        writer.result()
    };
    if ret < 0 {
        // Reflect the failure (write error or invalid format) in the stream's error indicator
        // so ferror() reports it.
        (*stream).error = 1;
    }
    ret
}
