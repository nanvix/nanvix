// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::{
    multibyte::wcstombs,
    wchar_t::wchar_t,
    wcslen::wcslen,
};
use ::core::ffi::VaList;
use ::libc_stdio::{
    stdout,
    vfprintf::vfprintf,
    FILE,
};
use ::sysapi::{
    ffi::{
        c_char,
        c_int,
        c_void,
    },
    sys_types::c_size_t,
};

//==================================================================================================
// External Symbols
//==================================================================================================

extern "C" {
    fn malloc(size: c_size_t) -> *mut c_void;
    fn free(ptr: *mut c_void);
}

//==================================================================================================
// Standalone Functions
//==================================================================================================

///
/// # Description
///
/// Writes formatted wide output to the given file stream, taking the conversion arguments from the
/// variable-argument list `ap`.
///
/// The wide format string is converted to a narrow (multibyte) format string and rendered with the
/// byte-oriented `vfprintf` engine. For every conversion specifier the wide and narrow `printf`
/// families consume identical argument types: plain `%s`/`%c` take a `char *`/`int`, while
/// `%ls`/`%lc` take a `wchar_t *`/`wint_t`. The shared formatting engine performs the wide-to-byte
/// conversion for the `%ls`/`%lc` cases, and in the single-byte C/POSIX locale one output byte
/// equals one wide character, so this produces exactly the wide output and character count while
/// reusing the single, well-tested formatting implementation.
///
/// # Parameters
///
/// - `stream`: Pointer to the target [`FILE`] stream.
/// - `format`: Null-terminated wide format string.
/// - `ap`: Variable-argument list supplying the conversion arguments.
///
/// # Return Value
///
/// The number of wide characters written, or a negative value on error.
///
/// # Safety
///
/// This function is unsafe because it dereferences raw pointers and consumes a variadic argument
/// list. The caller must ensure that `stream` is a valid, open [`FILE`], that `format` is a valid
/// wide format string, and that `ap` matches the conversions in `format`.
///
/// # References
///
/// - <https://pubs.opengroup.org/onlinepubs/9799919799/functions/vfwprintf.html>
///
#[cfg_attr(not(feature = "std"), unsafe(no_mangle))]
pub unsafe extern "C" fn vfwprintf(
    stream: *mut FILE,
    format: *const wchar_t,
    ap: VaList<'_>,
) -> c_int {
    if stream.is_null() || format.is_null() {
        return -1;
    }

    if (*stream).orientation == 0 {
        (*stream).orientation = 1;
    }

    // Convert the wide format string to a narrow (multibyte) format string.
    let flen: c_size_t = unsafe { wcslen(format) };
    let nfmt_size: c_size_t = flen.saturating_add(1);
    let nfmt: *mut c_char = unsafe { malloc(nfmt_size) }.cast::<c_char>();
    if nfmt.is_null() {
        return -1;
    }

    let nmax: usize = usize::try_from(nfmt_size).unwrap_or(0);
    // On an encoding error `wcstombs` returns `(size_t)-1`; fail and release the buffer.
    if unsafe { wcstombs(nfmt, format, nmax) } == usize::MAX {
        unsafe { free(nfmt.cast::<c_void>()) };
        return -1;
    }

    let ret: c_int = unsafe { vfprintf(stream, nfmt, ap) };
    unsafe { free(nfmt.cast::<c_void>()) };
    ret
}

///
/// # Description
///
/// Writes formatted wide output to the given file stream.
///
/// # Parameters
///
/// - `stream`: Pointer to the target [`FILE`] stream.
/// - `format`: Null-terminated wide format string.
///
/// # Return Value
///
/// The number of wide characters written, or a negative value on error.
///
/// # Safety
///
/// See [`vfwprintf`].
///
/// # References
///
/// - <https://pubs.opengroup.org/onlinepubs/9799919799/functions/fwprintf.html>
///
#[cfg_attr(not(feature = "std"), unsafe(no_mangle))]
pub unsafe extern "C" fn fwprintf(stream: *mut FILE, format: *const wchar_t, args: ...) -> c_int {
    unsafe { vfwprintf(stream, format, args) }
}

///
/// # Description
///
/// Writes formatted wide output to the standard output stream, taking the conversion arguments from
/// the variable-argument list `ap`. Equivalent to `vfwprintf(stdout, format, ap)`.
///
/// # Parameters
///
/// - `format`: Null-terminated wide format string.
/// - `ap`: Variable-argument list supplying the conversion arguments.
///
/// # Return Value
///
/// The number of wide characters written, or a negative value on error.
///
/// # Safety
///
/// See [`vfwprintf`].
///
/// # References
///
/// - <https://pubs.opengroup.org/onlinepubs/9799919799/functions/vwprintf.html>
///
#[cfg_attr(not(feature = "std"), unsafe(no_mangle))]
pub unsafe extern "C" fn vwprintf(format: *const wchar_t, ap: VaList<'_>) -> c_int {
    unsafe { vfwprintf(stdout(), format, ap) }
}

///
/// # Description
///
/// Writes formatted wide output to the standard output stream. Equivalent to
/// `fwprintf(stdout, format, ...)`.
///
/// # Parameters
///
/// - `format`: Null-terminated wide format string.
///
/// # Return Value
///
/// The number of wide characters written, or a negative value on error.
///
/// # Safety
///
/// See [`vfwprintf`].
///
/// # References
///
/// - <https://pubs.opengroup.org/onlinepubs/9799919799/functions/wprintf.html>
///
#[cfg_attr(not(feature = "std"), unsafe(no_mangle))]
pub unsafe extern "C" fn wprintf(format: *const wchar_t, args: ...) -> c_int {
    unsafe { vwprintf(format, args) }
}
