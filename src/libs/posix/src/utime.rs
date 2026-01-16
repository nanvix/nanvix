// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Standalone Functions
//==================================================================================================

#[cfg(all(feature = "syscall", feature = "staticlib"))]
mod bindings {
    use crate::errno::__errno_location;
    use ::sys::error::ErrorCode;
    use ::sysapi::{
        fcntl::atflags::AT_FDCWD,
        ffi::{
            c_char,
            c_int,
        },
        sys_types::time_t,
        time::timespec,
        utime::utimbuf,
    };
    use ::syslog::trace_syscall;

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
    #[trace_syscall]
    pub unsafe extern "C" fn utime(filename: *const c_char, times: *const utimbuf) -> c_int {
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
