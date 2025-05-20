// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Configuration
//==================================================================================================

#![allow(non_camel_case_types)]

//==================================================================================================
// Imports
//==================================================================================================

use ::syscall::time::time_t;

//==================================================================================================
// Structures
//==================================================================================================

#[derive(Default, Debug, Clone, Copy)]
#[repr(C, packed)]
pub struct utimbuf {
    /// Access time.
    pub actime: time_t,
    /// Modification time.
    pub modtime: time_t,
}

//==================================================================================================
// Standalone Functions
//==================================================================================================

#[cfg(all(feature = "syscall", feature = "staticlib"))]
mod bindings {
    use super::*;
    use crate::errno::__errno_location;
    use ::sys::error::ErrorCode;
    use syscall::{
        fcntl::AT_FDCWD,
        ffi::{
            c_char,
            c_int,
        },
        time::timespec,
    };

    ///
    /// # Description
    ///
    /// Sets file access and modification times.
    ///
    /// # Parameters
    ///
    /// - `pathname`: Pathname of the file.
    /// - `times`: Access and modification times.
    ///
    /// # Returns
    ///
    /// Upon successful completion, zero is returned. Otherwise, it returns -1 and sets `errno` to
    /// indicate the error.
    ///
    /// # Safety
    ///
    /// This function is unsafe because it dereferences raw pointers.
    ///
    /// It is safe to call this function if the following conditions are met:
    /// - `filename` points to a valid null-terminated C string.
    /// - `times` points to a valid `utimbuf` structures.
    ///
    #[unsafe(no_mangle)]
    pub unsafe extern "C" fn utime(filename: *const c_char, times: *const utimbuf) -> c_int {
        ::syslog::trace!("utime(): filename={:?}, times={:?}", filename, times);

        // Check if `times` is invalid.
        if times.is_null() {
            ::syslog::error!("utime(): invalid times (filename={:?}, times={:?})", filename, times);
            *__errno_location() = ErrorCode::InvalidArgument.get();
            return -1;
        }

        // Attempt to convert `times`.
        let times: [timespec; 2] = [
            timespec {
                tv_sec: (*times).actime as time_t,
                tv_nsec: 0,
            },
            timespec {
                tv_sec: (*times).modtime as time_t,
                tv_nsec: 0,
            },
        ];

        crate::sys::stat::utimensat(AT_FDCWD, filename, times.as_ptr(), 0)
    }
}
