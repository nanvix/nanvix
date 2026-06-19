// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use ::sysapi::{
    errno::{
        EINVAL,
        ERANGE,
    },
    ffi::{
        c_char,
        c_int,
    },
    sys_types::c_size_t,
};

//==================================================================================================
// Standalone Functions
//==================================================================================================

///
/// # Description
///
/// Stores a textual description of the error number `errnum` in `buf`.
///
/// # Parameters
///
/// - `errnum`: Error number to describe.
/// - `buf`: Buffer that receives the message.
/// - `buflen`: Size of `buf` in bytes.
///
/// # Returns
///
/// Returns zero on success. Otherwise, returns an error number to indicate the error.
///
/// # Safety
///
/// This function is unsafe because it dereferences raw pointers. `buf` must point to at least
/// `buflen` writable bytes.
///
/// # References
///
/// - <https://pubs.opengroup.org/onlinepubs/9799919799/functions/strerror_r.html>
///
#[cfg_attr(not(feature = "std"), unsafe(no_mangle))]
pub unsafe extern "C" fn strerror_r(errnum: c_int, buf: *mut c_char, buflen: c_size_t) -> c_int {
    if buf.is_null() {
        return EINVAL;
    }

    // SAFETY: strerror returns a valid, null-terminated static string.
    let src: *const c_char = unsafe { crate::strerror::strerror(errnum) };

    // SAFETY: src is a valid, null-terminated string.
    let len: usize = unsafe { crate::strlen::strlen(src) } as usize;
    if buflen == 0 {
        return ERANGE;
    }

    let cap: usize = buflen as usize - 1;
    let copy_len: usize = if len < cap { len } else { cap };
    // SAFETY: src has at least copy_len bytes; buf has room for copy_len + 1.
    unsafe {
        core::ptr::copy_nonoverlapping(src.cast::<u8>(), buf.cast::<u8>(), copy_len);
        *buf.add(copy_len) = 0;
    }

    if len >= buflen as usize {
        ERANGE
    } else {
        0
    }
}

#[cfg(all(test, feature = "std"))]
mod test {
    use super::strerror_r;
    use ::std::vec::Vec;
    use ::sysapi::{
        errno::{
            EINVAL,
            ERANGE,
        },
        ffi::{
            c_char,
            c_int,
        },
    };

    /// Computes the length of a null-terminated string.
    fn c_strlen(p: *const c_char) -> usize {
        let mut i: usize = 0;
        unsafe {
            while *p.add(i) != 0 {
                i += 1;
            }
        }
        i
    }

    #[test]
    fn test_strerror_r_writes_buf_and_returns_zero() {
        let mut buf: Vec<c_char> = vec![0x7F as c_char; 64];
        let ret: c_int = unsafe { strerror_r(22 as c_int, buf.as_mut_ptr(), 64) };
        assert_eq!(ret, 0, "strerror_r must return zero on success");
        // The message must be null-terminated within the buffer.
        assert!(c_strlen(buf.as_ptr()) < 64, "message must be null-terminated within buf");
    }

    #[test]
    fn test_strerror_r_zero() {
        let mut buf: Vec<c_char> = vec![0x7F as c_char; 64];
        let ret: c_int = unsafe { strerror_r(0 as c_int, buf.as_mut_ptr(), 64) };
        assert_eq!(ret, 0, "strerror_r(0) must succeed with a no-error message");
        assert!(c_strlen(buf.as_ptr()) < 64, "message must be null-terminated within buf");
    }

    #[test]
    fn test_strerror_r_truncates_and_returns_erange() {
        let mut buf: Vec<c_char> = vec![0x7F as c_char; 4];
        let ret: c_int = unsafe { strerror_r(22 as c_int, buf.as_mut_ptr(), 4) };
        assert_eq!(ret, ERANGE, "strerror_r must fail with ERANGE when buf is too small");
        // The buffer must always be null-terminated at size - 1 or earlier.
        assert_eq!(buf[3], 0 as c_char, "buf must be null-terminated at size - 1");
        assert!(c_strlen(buf.as_ptr()) <= 3, "truncated message fits within buf");
    }

    #[test]
    fn test_strerror_r_zero_buflen_returns_erange() {
        let mut buf: Vec<c_char> = vec![0x7F as c_char; 4];
        let ret: c_int = unsafe { strerror_r(22 as c_int, buf.as_mut_ptr(), 0) };
        assert_eq!(ret, ERANGE, "strerror_r must fail with ERANGE when buflen is zero");
        assert_eq!(buf[0], 0x7F as c_char, "buf must be untouched when buflen is 0");
    }

    #[test]
    fn test_strerror_r_null_buf_returns_einval() {
        let ret: c_int = unsafe { strerror_r(22 as c_int, core::ptr::null_mut(), 16) };
        assert_eq!(ret, EINVAL, "strerror_r returns EINVAL for a null buffer");
    }
}
