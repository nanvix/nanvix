// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Lint Configuration
//==================================================================================================

#![forbid(clippy::cast_sign_loss)]

//==================================================================================================
// Imports
//==================================================================================================

use ::sysapi::ffi::{
    c_char,
    c_int,
};

//==================================================================================================
// Standalone Functions
//==================================================================================================

///
/// # Description
///
/// Maps an error number to a message string.
///
/// This function returns a pointer to a string that describes the error code passed in the argument
/// `errnum`.
///
/// # Parameters
///
/// - `errnum`: Error number.
///
/// # Return Value
///
/// Returns a pointer to the appropriate error description string.
///
/// # Safety
///
/// This function is unsafe because it returns a raw pointer.
///
#[cfg_attr(not(feature = "std"), unsafe(no_mangle))]
pub unsafe extern "C" fn strerror(errnum: c_int) -> *mut c_char {
    let msg: &[u8] = match errnum {
        0 => b"Success\0",
        1 => b"Operation not permitted\0",
        2 => b"No such file or directory\0",
        3 => b"No such process\0",
        4 => b"Interrupted system call\0",
        5 => b"Input/output error\0",
        6 => b"No such device or address\0",
        7 => b"Argument list too long\0",
        8 => b"Exec format error\0",
        9 => b"Bad file descriptor\0",
        10 => b"No child processes\0",
        11 => b"Resource temporarily unavailable\0",
        12 => b"Cannot allocate memory\0",
        13 => b"Permission denied\0",
        14 => b"Bad address\0",
        16 => b"Device or resource busy\0",
        17 => b"File exists\0",
        19 => b"No such device\0",
        20 => b"Not a directory\0",
        21 => b"Is a directory\0",
        22 => b"Invalid argument\0",
        23 => b"Too many open files in system\0",
        24 => b"Too many open files\0",
        25 => b"Inappropriate ioctl for device\0",
        27 => b"File too large\0",
        28 => b"No space left on device\0",
        29 => b"Illegal seek\0",
        30 => b"Read-only file system\0",
        32 => b"Broken pipe\0",
        33 => b"Numerical argument out of domain\0",
        34 => b"Numerical result out of range\0",
        88 => b"Function not implemented\0",
        95 => b"Operation not supported\0",
        _ => b"Unknown error\0",
    };
    msg.as_ptr().cast::<c_char>().cast_mut()
}

#[cfg(all(test, feature = "std"))]
mod test {
    use super::strerror;
    use ::std::vec::Vec;
    use ::sysapi::ffi::c_char;

    fn c_str_to_bytes(p: *const c_char) -> Vec<u8> {
        let mut v: Vec<u8> = Vec::new();
        let mut i: usize = 0;
        unsafe {
            while *p.add(i) != 0 {
                v.push(u8::from_ne_bytes((*p.add(i)).to_ne_bytes()));
                i += 1;
            }
        }
        v
    }

    #[test]
    fn test_strerror_known() {
        let ret: *mut c_char = unsafe { strerror(22) };
        assert!(!ret.is_null());
        assert_eq!(c_str_to_bytes(ret), b"Invalid argument");
    }

    #[test]
    fn test_strerror_zero() {
        let ret: *mut c_char = unsafe { strerror(0) };
        assert!(!ret.is_null());
        assert_eq!(c_str_to_bytes(ret), b"Success");
    }

    #[test]
    fn test_strerror_enosys() {
        // ENOSYS is 88 on Nanvix (see sysapi::errno), not 38 as on Linux.
        let ret: *mut c_char = unsafe { strerror(88) };
        assert!(!ret.is_null());
        assert_eq!(c_str_to_bytes(ret), b"Function not implemented");
    }

    #[test]
    fn test_strerror_unknown() {
        let ret: *mut c_char = unsafe { strerror(999) };
        assert!(!ret.is_null());
        assert_eq!(c_str_to_bytes(ret), b"Unknown error");
    }
}
