// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::tm_struct::tm;
use ::sysapi::{
    ffi::c_char,
    sys_types::time_t,
};

//==================================================================================================
// Standalone Functions
//==================================================================================================

///
/// # Description
///
/// Converts a calendar time to a string representation of the form
/// `"Wed Jun 30 21:49:08 1993\n"` using a caller-provided buffer.
///
/// Equivalent to `asctime_r(localtime_r(timep, &tmp), buf)`.
///
/// # Parameters
///
/// - `timep`: Pointer to the calendar time to convert.
/// - `buf`: Pointer to a character buffer of at least 26 bytes.
///
/// # Returns
///
/// On success, returns `buf`. Returns a null pointer on error.
///
/// # Safety
///
/// This function is unsafe because it dereferences the raw pointers `timep` and `buf`. The caller
/// must ensure `buf` has room for at least 26 bytes.
///
#[cfg_attr(not(feature = "std"), unsafe(no_mangle))]
pub unsafe extern "C" fn ctime_r(timep: *const time_t, buf: *mut c_char) -> *mut c_char {
    if timep.is_null() || buf.is_null() {
        return core::ptr::null_mut();
    }

    let mut tmp: tm = tm::new();
    let ret: *mut tm = crate::localtime_r::localtime_r(timep, &mut tmp);
    if ret.is_null() {
        return core::ptr::null_mut();
    }

    crate::asctime_r::asctime_r(&tmp, buf)
}

//==================================================================================================
// Unit Tests
//==================================================================================================

#[cfg(all(test, feature = "std"))]
mod test {
    use super::ctime_r;
    use ::sysapi::{
        ffi::c_char,
        sys_types::time_t,
    };

    fn buf_to_string(buf: &[c_char; 26]) -> ::std::string::String {
        let bytes: ::std::vec::Vec<u8> = buf
            .iter()
            .take_while(|&&c| c != 0)
            .map(|&c| c as u8)
            .collect();
        ::std::string::String::from_utf8(bytes).expect("valid UTF-8")
    }

    #[test]
    fn test_ctime_r_epoch() {
        let t: time_t = 0;
        let mut buf: [c_char; 26] = [0; 26];
        let ret: *mut c_char = unsafe { ctime_r(&t, buf.as_mut_ptr()) };
        assert!(!ret.is_null());
        assert_eq!(buf_to_string(&buf), "Thu Jan  1 00:00:00 1970\n");
    }

    #[test]
    fn test_ctime_r_known_date() {
        // 1993-06-30 21:49:08 UTC.
        let t: time_t = 741_476_948;
        let mut buf: [c_char; 26] = [0; 26];
        let ret: *mut c_char = unsafe { ctime_r(&t, buf.as_mut_ptr()) };
        assert!(!ret.is_null());
        assert_eq!(buf_to_string(&buf), "Wed Jun 30 21:49:08 1993\n");
    }

    #[test]
    fn test_ctime_r_null_pointers() {
        let t: time_t = 0;
        let mut buf: [c_char; 26] = [0; 26];
        assert!(unsafe { ctime_r(core::ptr::null(), buf.as_mut_ptr()) }.is_null());
        assert!(unsafe { ctime_r(&t, core::ptr::null_mut()) }.is_null());
    }
}
