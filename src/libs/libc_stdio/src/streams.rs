// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use ::sysapi::ffi::c_int;

//==================================================================================================
// Types
//==================================================================================================

/// Minimal FILE structure wrapping a file descriptor.
#[repr(C)]
pub struct FILE {
    /// File descriptor.
    pub fd: c_int,
    /// End-of-file indicator (0 = not EOF).
    pub eof: c_int,
    /// Error indicator (0 = no error).
    pub error: c_int,
    /// One-character push-back buffer for [`ungetc`](`crate::ungetc::ungetc`) (-1 = empty).
    pub ungetc_buf: c_int,
}

//==================================================================================================
// Static Data
//==================================================================================================

/// Standard input stream.
static mut STDIN_FILE: FILE = FILE {
    fd: 0,
    eof: 0,
    error: 0,
    ungetc_buf: -1,
};
/// Standard output stream.
static mut STDOUT_FILE: FILE = FILE {
    fd: 1,
    eof: 0,
    error: 0,
    ungetc_buf: -1,
};
/// Standard error stream.
static mut STDERR_FILE: FILE = FILE {
    fd: 2,
    eof: 0,
    error: 0,
    ungetc_buf: -1,
};

//==================================================================================================
// Standalone Functions
//==================================================================================================

///
/// # Description
///
/// Returns a pointer to the standard input stream.
///
/// # Returns
///
/// A mutable pointer to the standard input [`FILE`].
///
/// # Safety
///
/// The caller must ensure that no data races occur when accessing the returned pointer.
///
#[cfg_attr(not(feature = "std"), unsafe(no_mangle))]
pub unsafe extern "C" fn stdin() -> *mut FILE {
    &raw mut STDIN_FILE
}

///
/// # Description
///
/// Returns a pointer to the standard output stream.
///
/// # Returns
///
/// A mutable pointer to the standard output [`FILE`].
///
/// # Safety
///
/// The caller must ensure that no data races occur when accessing the returned pointer.
///
#[cfg_attr(not(feature = "std"), unsafe(no_mangle))]
pub unsafe extern "C" fn stdout() -> *mut FILE {
    &raw mut STDOUT_FILE
}

///
/// # Description
///
/// Returns a pointer to the standard error stream.
///
/// # Returns
///
/// A mutable pointer to the standard error [`FILE`].
///
/// # Safety
///
/// The caller must ensure that no data races occur when accessing the returned pointer.
///
#[cfg_attr(not(feature = "std"), unsafe(no_mangle))]
pub unsafe extern "C" fn stderr() -> *mut FILE {
    &raw mut STDERR_FILE
}

//==================================================================================================
// Tests
//==================================================================================================

#[cfg(all(test, feature = "std"))]
mod test {
    use super::{
        stderr,
        stdin,
        stdout,
    };

    #[test]
    fn standard_streams_have_expected_descriptors() {
        // SAFETY: the accessors return valid pointers to the static standard streams.
        unsafe {
            assert_eq!((*stdin()).fd, 0);
            assert_eq!((*stdout()).fd, 1);
            assert_eq!((*stderr()).fd, 2);
        }
    }

    #[test]
    fn standard_streams_start_without_errors_or_pushback() {
        // SAFETY: the accessors return valid pointers to the static standard streams.
        unsafe {
            let s: *mut super::FILE = stdin();
            assert_eq!((*s).eof, 0);
            assert_eq!((*s).error, 0);
            assert_eq!((*s).ungetc_buf, -1);
        }
    }

    #[test]
    fn standard_streams_are_distinct_and_stable() {
        // SAFETY: the accessors return stable pointers to distinct static streams.
        unsafe {
            assert!(!core::ptr::eq(stdin(), stdout()));
            assert!(!core::ptr::eq(stdout(), stderr()));
            assert!(core::ptr::eq(stdin(), stdin()));
        }
    }
}
