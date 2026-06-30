// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::{
    strftime::strftime,
    tm_struct::tm,
};
use ::sysapi::{
    ffi::{
        c_char,
        c_void,
    },
    sys_types::c_size_t,
};

//==================================================================================================
// Standalone Functions
//==================================================================================================

/// # Description
///
/// Formats a broken-down time according to `format` using the C/POSIX locale. Nanvix supports only
/// the C/POSIX locale, so this delegates to `strftime()` and ignores the `locale_t` argument.
///
/// # Parameters
///
/// - `s`: Destination buffer for the formatted string.
/// - `max`: Maximum number of bytes to write to `s`, including the null terminator.
/// - `format`: Pointer to the null-terminated format string.
/// - `timeptr`: Pointer to the broken-down time to format.
/// - `locale`: Locale to use (ignored; only the C/POSIX locale is supported).
///
/// # Returns
///
/// The number of bytes written to `s` (excluding the null terminator), or `0` if the result would
/// not fit.
///
/// # Safety
///
/// This function is unsafe because it dereferences the raw pointers `s`, `format`, and `timeptr`,
/// which must be valid for the requested operation.
#[cfg_attr(not(feature = "std"), unsafe(no_mangle))]
pub unsafe extern "C" fn strftime_l(
    s: *mut c_char,
    max: c_size_t,
    format: *const c_char,
    timeptr: *const tm,
    _locale: *mut c_void,
) -> c_size_t {
    unsafe { strftime(s, max, format, timeptr) }
}

//==================================================================================================
// Unit Tests
//==================================================================================================

#[cfg(all(test, feature = "std"))]
mod test {
    use super::*;
    use ::std::{
        ffi::CString,
        string::String,
        vec,
    };

    #[test]
    fn test_strftime_l_delegates_to_strftime() {
        let mut time: tm = tm::new();
        time.tm_year = 2021 - 1900;
        time.tm_mon = 3 - 1;
        time.tm_mday = 14;

        let format: CString = CString::new("%Y-%m-%d").unwrap_or_else(|_| CString::default());
        let mut buffer: vec::Vec<u8> = vec![0u8; 64];
        let capacity: c_size_t = c_size_t::try_from(buffer.len()).expect("capacity fits");
        let locale: *mut c_void = ::core::ptr::null_mut();

        let written: c_size_t = unsafe {
            strftime_l(
                buffer.as_mut_ptr().cast::<c_char>(),
                capacity,
                format.as_ptr(),
                &time,
                locale,
            )
        };

        assert_eq!(String::from_utf8_lossy(&buffer[..written as usize]), "2021-03-14");
    }
}
