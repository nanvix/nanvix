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
// Constants
//==================================================================================================

/// File descriptor for standard output.
const STDOUT_FD: c_int = 1;

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
}

//==================================================================================================
// Standalone Functions
//==================================================================================================

///
/// # Description
///
/// Writes formatted output to standard output. This is a variadic wrapper that creates a
/// [`VaList`] and delegates to the formatting engine.
///
/// # Parameters
///
/// - `fmt`: Pointer to a null-terminated printf format string.
/// - `...`: Arguments matching the format specifiers in `fmt`.
///
/// # Returns
///
/// The number of characters written on success, or `-1` on error.
///
/// # Safety
///
/// This function is unsafe because it dereferences a raw pointer. The caller must ensure that:
/// - `fmt` points to a valid, null-terminated format string.
/// - The variadic arguments match the format specifiers.
///
/// # References
///
/// - <https://pubs.opengroup.org/onlinepubs/9799919799/functions/printf.html>
///
#[cfg_attr(not(feature = "std"), unsafe(no_mangle))]
pub unsafe extern "C" fn printf(fmt: *const c_char, args: ...) -> c_int {
    let mut writer: FdWriter = FdWriter::new(STDOUT_FD);
    let ap: VaList<'_> = args;
    let mut va_args: VaListArgs<'_> = VaListArgs { va: ap };
    let ret: c_int = format_core(&mut writer, fmt, &mut va_args);
    if ret < 0 {
        return ret;
    }
    writer.result()
}
