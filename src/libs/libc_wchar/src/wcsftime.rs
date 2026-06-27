// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Lint Configuration
//==================================================================================================

#![forbid(clippy::cast_sign_loss)]

//==================================================================================================
// Imports
//==================================================================================================

use crate::{
    wchar_t::wchar_t,
    wcs_narrow::to_narrow_alloc_full,
};
use ::sysapi::{
    ffi::{
        c_char,
        c_void,
    },
    sys_types::c_size_t,
};

//==================================================================================================
// Types
//==================================================================================================

/// Opaque broken-down time structure declared in `<time.h>`.
///
/// It is used solely as a pointer target that is forwarded verbatim to `strftime()`; the layout is
/// never inspected here, so an opaque placeholder is sufficient and keeps this crate decoupled from
/// the time crate.
#[allow(non_camel_case_types)]
#[repr(C)]
pub struct tm {
    _private: [u8; 0],
}

//==================================================================================================
// External Symbols
//==================================================================================================

extern "C" {
    fn malloc(size: c_size_t) -> *mut c_void;
    fn free(ptr: *mut c_void);
    fn strftime(
        s: *mut c_char,
        max: c_size_t,
        format: *const c_char,
        timeptr: *const tm,
    ) -> c_size_t;
}

//==================================================================================================
// Standalone Functions
//==================================================================================================

///
/// # Description
///
/// Formats the broken-down time `timeptr` into the wide-character array `s` according to the
/// wide-character format string `format`. Nanvix operates in the C locale, so this delegates to
/// [`strftime`]: the single-byte format string is narrowed, formatted, and the resulting bytes are
/// widened back into `s`.
///
/// # Parameters
///
/// - `s`: Destination wide-character buffer.
/// - `maxsize`: Capacity of `s`, in wide characters (including the null terminator).
/// - `format`: Null-terminated wide-character format string.
/// - `timeptr`: Broken-down time to format.
///
/// # Returns
///
/// The number of wide characters written to `s`, excluding the null terminator, or `0` if the
/// result (including the terminator) would not fit in `maxsize` wide characters.
///
/// # Safety
///
/// This function is unsafe because it dereferences raw pointers. The caller must ensure that `s`
/// points to storage for at least `maxsize` wide characters and that `format` and `timeptr` point
/// to valid objects.
///
/// # References
///
/// - <https://pubs.opengroup.org/onlinepubs/9799919799/functions/wcsftime.html>
///
#[cfg_attr(not(feature = "std"), unsafe(no_mangle))]
pub unsafe extern "C" fn wcsftime(
    s: *mut wchar_t,
    maxsize: c_size_t,
    format: *const wchar_t,
    timeptr: *const tm,
) -> c_size_t {
    if s.is_null() || format.is_null() || timeptr.is_null() || maxsize == 0 {
        return 0;
    }

    // Narrow the format string to bytes for strftime().
    let narrow_format: crate::wcs_narrow::NarrowString =
        match unsafe { to_narrow_alloc_full(format) } {
            Some(f) => f,
            None => return 0,
        };

    // Format into a narrow scratch buffer with the same element budget as the wide destination.
    let narrow_buf: *mut c_char = unsafe { malloc(maxsize) }.cast::<c_char>();
    if narrow_buf.is_null() {
        return 0;
    }

    let written: c_size_t =
        unsafe { strftime(narrow_buf, maxsize, narrow_format.as_ptr(), timeptr) };
    if written == 0 {
        // A 0 return is either an overflow or a valid empty result; in both cases the contents of
        // the narrow buffer are unspecified, so deterministically terminate the destination. The
        // earlier `maxsize == 0` guard guarantees there is room for the terminator.
        unsafe { *s = 0 };
        unsafe { free(narrow_buf.cast::<c_void>()) };
        return 0;
    }

    // Widen the formatted bytes back into the destination. strftime() guarantees that
    // `written < maxsize`, so the terminating null at index `written` stays in bounds.
    for i in 0..written as usize {
        let byte: u8 = unsafe { *narrow_buf.add(i) }.to_ne_bytes()[0];
        unsafe { *s.add(i) = wchar_t::from(byte) };
    }
    unsafe { *s.add(written as usize) = 0 };

    unsafe { free(narrow_buf.cast::<c_void>()) };
    written
}

//==================================================================================================
// Unit Tests
//==================================================================================================

#[cfg(all(test, feature = "std"))]
mod test {
    use super::*;
    use ::sysapi::ffi::c_int;

    #[repr(C)]
    struct HostTm {
        tm_sec: c_int,
        tm_min: c_int,
        tm_hour: c_int,
        tm_mday: c_int,
        tm_mon: c_int,
        tm_year: c_int,
        tm_wday: c_int,
        tm_yday: c_int,
        tm_isdst: c_int,
    }

    fn make_time() -> HostTm {
        HostTm {
            tm_sec: 7,
            tm_min: 5,
            tm_hour: 9,
            tm_mday: 14,
            tm_mon: 2,
            tm_year: 121,
            tm_wday: 0,
            tm_yday: 72,
            tm_isdst: 0,
        }
    }

    fn c_size_len(len: usize) -> c_size_t {
        c_size_t::try_from(len).expect("buffer length should fit in c_size_t")
    }

    #[test]
    fn test_wcsftime_formats_time() {
        let mut dest: [wchar_t; 5] = [0; 5];
        let format: [wchar_t; 3] = [0x25, 0x59, 0];
        let time: HostTm = make_time();

        let written: c_size_t = unsafe {
            wcsftime(
                dest.as_mut_ptr(),
                c_size_len(dest.len()),
                format.as_ptr(),
                (&time as *const HostTm).cast::<tm>(),
            )
        };

        assert_eq!(written, 4);
        assert_eq!(dest, [0x32, 0x30, 0x32, 0x31, 0]);
    }

    #[test]
    fn test_wcsftime_empty_format_terminates_destination() {
        let mut dest: [wchar_t; 1] = [0x78];
        let format: [wchar_t; 1] = [0];
        let time: HostTm = make_time();

        let written: c_size_t = unsafe {
            wcsftime(
                dest.as_mut_ptr(),
                c_size_len(dest.len()),
                format.as_ptr(),
                (&time as *const HostTm).cast::<tm>(),
            )
        };

        assert_eq!(written, 0);
        assert_eq!(dest[0], 0);
    }

    #[test]
    fn test_wcsftime_rejects_unrepresentable_format_character() {
        let mut dest: [wchar_t; 4] = [-1; 4];
        let format: [wchar_t; 4] = [0x58, 0x100, 0x59, 0];
        let time: HostTm = make_time();

        let written: c_size_t = unsafe {
            wcsftime(
                dest.as_mut_ptr(),
                c_size_len(dest.len()),
                format.as_ptr(),
                (&time as *const HostTm).cast::<tm>(),
            )
        };

        assert_eq!(written, 0);
        assert_eq!(dest, [-1; 4]);
    }
}
