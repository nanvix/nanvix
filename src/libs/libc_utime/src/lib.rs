// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Crate Configuration
//==================================================================================================

#![cfg_attr(not(feature = "std"), no_std)]

//==================================================================================================
// Standalone Functions
//==================================================================================================

mod bindings {
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

    unsafe extern "C" {
        /// Sets file access and modification times relative to a directory file
        /// descriptor.  Defined by `libc_sys_stat` and resolved at link time from
        /// the same archive.
        fn utimensat(
            dirfd: c_int,
            filename: *const c_char,
            times: *const timespec,
            flags: c_int,
        ) -> c_int;
    }

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
        // A NULL `times` means "set both timestamps to the current time" per POSIX.
        // Forward the NULL down to utimensat(), which handles it.
        if times.is_null() {
            return utimensat(AT_FDCWD, filename, ::core::ptr::null(), 0);
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

        utimensat(AT_FDCWD, filename, times.as_ptr(), 0)
    }
}
