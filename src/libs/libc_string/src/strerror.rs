// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Lint Configuration
//==================================================================================================

#![forbid(clippy::cast_sign_loss)]

//==================================================================================================
// Imports
//==================================================================================================

use ::sysapi::{
    errno::{
        E2BIG,
        EACCES,
        EAGAIN,
        EBADF,
        EBUSY,
        ECHILD,
        EDOM,
        EEXIST,
        EFAULT,
        EFBIG,
        EINTR,
        EINVAL,
        EIO,
        EISDIR,
        EMFILE,
        ENFILE,
        ENODEV,
        ENOENT,
        ENOEXEC,
        ENOMEM,
        ENOSPC,
        ENOSYS,
        ENOTDIR,
        ENOTTY,
        ENXIO,
        EOPNOTSUPP,
        EPERM,
        EPIPE,
        ERANGE,
        EROFS,
        ESPIPE,
        ESRCH,
    },
    ffi::{
        c_char,
        c_int,
    },
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
        EPERM => b"Operation not permitted\0",
        ENOENT => b"No such file or directory\0",
        ESRCH => b"No such process\0",
        EINTR => b"Interrupted system call\0",
        EIO => b"Input/output error\0",
        ENXIO => b"No such device or address\0",
        E2BIG => b"Argument list too long\0",
        ENOEXEC => b"Exec format error\0",
        EBADF => b"Bad file descriptor\0",
        ECHILD => b"No child processes\0",
        EAGAIN => b"Resource temporarily unavailable\0",
        ENOMEM => b"Cannot allocate memory\0",
        EACCES => b"Permission denied\0",
        EFAULT => b"Bad address\0",
        EBUSY => b"Device or resource busy\0",
        EEXIST => b"File exists\0",
        ENODEV => b"No such device\0",
        ENOTDIR => b"Not a directory\0",
        EISDIR => b"Is a directory\0",
        EINVAL => b"Invalid argument\0",
        ENFILE => b"Too many open files in system\0",
        EMFILE => b"Too many open files\0",
        ENOTTY => b"Inappropriate ioctl for device\0",
        EFBIG => b"File too large\0",
        ENOSPC => b"No space left on device\0",
        ESPIPE => b"Illegal seek\0",
        EROFS => b"Read-only file system\0",
        EPIPE => b"Broken pipe\0",
        EDOM => b"Numerical argument out of domain\0",
        ERANGE => b"Numerical result out of range\0",
        ENOSYS => b"Function not implemented\0",
        EOPNOTSUPP => b"Operation not supported\0",
        _ => b"Unknown error\0",
    };
    msg.as_ptr().cast::<c_char>().cast_mut()
}

#[cfg(all(test, feature = "std"))]
mod test {
    use super::strerror;
    use ::std::vec::Vec;
    use ::sysapi::{
        errno::{
            EINVAL,
            ENOSYS,
        },
        ffi::c_char,
    };

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
        let ret: *mut c_char = unsafe { strerror(EINVAL) };
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
        let ret: *mut c_char = unsafe { strerror(ENOSYS) };
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
