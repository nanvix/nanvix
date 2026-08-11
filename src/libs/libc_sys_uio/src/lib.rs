// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Crate Configuration
//==================================================================================================

#![cfg_attr(not(feature = "std"), no_std)]

//==================================================================================================
// Imports
//==================================================================================================

use ::core::slice;
use ::sys::error::{
    Error,
    ErrorCode,
};
use ::sysapi::ffi::c_int;
use ::syscall::{
    errno::__errno_location,
    unistd,
};
use ::syslog::trace_syscall;
use sysapi::{
    limits::{
        IOV_MAX,
        SSIZE_MAX,
    },
    sys_types::{
        c_size_t,
        c_ssize_t,
        off_t,
        size_t,
    },
    sys_uio::iovec,
};

//==================================================================================================
// Standalone Functions
//==================================================================================================

///
/// # Description
///
/// Performs a positioned *gather* write: writes data from a list of `iovcnt` buffers
/// (an `iovec` array) to the file referred to by `fd`, starting at `offset`, without
/// changing the file offset. The buffers are written in array order.
///
/// # Standard
///
/// `pwritev()` is a BSD/Linux extension and is **not** specified by POSIX. Its semantics
/// combine two POSIX functions: the scatter/gather behaviour of `writev()` and the
/// positioned write of `pwrite()`.
/// - `writev()`: <https://pubs.opengroup.org/onlinepubs/9799919799/functions/writev.html>
/// - `pwrite()`: <https://pubs.opengroup.org/onlinepubs/9799919799/functions/write.html>
///
/// # Parameters
///
/// - `fd`: File descriptor.
/// - `iov`: Pointer to an array of `iovec` structures.
/// - `iovcnt`: Number of elements in the array.
/// - `offset`: Offset to write to.
///
/// # Returns
///
/// Returns a `c_ssize_t`:
/// - On success: the number of bytes written (a non-negative `c_ssize_t`).
/// - On failure: `-1`, with `errno` set to indicate the error. In particular, if the
///   sum of the `iov_len` values overflows `SSIZE_MAX`, `errno` is set to `EINVAL`.
///
/// # Safety
///
/// This function is unsafe because:
/// - It may dereference raw pointers.
/// - It may access global variables.
///
/// It is safe to call this function if and only if the following conditions are met:
/// - The `iov` pointer is valid and points to an array of `iovec` structures.
/// - This function is called from multiple threads at the same time.
///
#[unsafe(no_mangle)]
#[trace_syscall]
pub unsafe extern "C" fn pwritev(
    fd: i32,
    iov: *const iovec,
    iovcnt: c_int,
    offset: off_t,
) -> c_ssize_t {
    // Check if number of elements in the vector is valid.
    if iovcnt < 0 {
        ::syslog::warn!("pwritev(): invalid iovcnt {iovcnt}");
        *__errno_location() = ErrorCode::InvalidArgument.get();
        return -1;
    }

    // Check if vector base is invalid.
    if iov.is_null() {
        ::syslog::warn!("pwritev(): invalid iov {iov:?}");
        *__errno_location() = ErrorCode::InvalidArgument.get();
        return -1;
    }

    // Check for zero-length vector.
    if iovcnt == 0 {
        return 0;
    }

    let do_writev = |dry_run: bool| -> Result<c_size_t, Error> {
        let mut offset: off_t = offset;
        let mut total: c_size_t = 0;

        // Traverse i/o vector.
        for i in 0..iovcnt {
            let iov: *const iovec = unsafe { iov.offset(i as isize) };

            // Check if iov is invalid.
            if iov.is_null() {
                ::syslog::warn!("pwritev(): invalid iov {iov:?}");
                return Err(Error::new(ErrorCode::InvalidArgument, "invalid iov"));
            }

            let iov_base: *mut u8 = unsafe { (*iov).iov_base };
            let iov_len: size_t = unsafe { (*iov).iov_len };

            // Check if `iov_base` is invalid.
            if iov_base.is_null() {
                ::syslog::warn!("pwritev(): invalid iov_base {iov_base:?}");
                return Err(Error::new(ErrorCode::InvalidArgument, "invalid iov_base"));
            }

            // Check if `iov_len` is invalid.
            if iov_len == 0 {
                ::syslog::warn!("pwritev(): invalid iov_len {iov_len}");
                return Err(Error::new(ErrorCode::InvalidArgument, "invalid iov_len"));
            }

            // Copy data only if not running in dry-run mode.
            let count: c_size_t = if !dry_run {
                // Construct buffer from raw parts.
                let buffer: &[u8] = slice::from_raw_parts(iov_base as *const u8, iov_len as usize);
                // Write data and parse result.
                match unistd::syscall::pwrite(fd, buffer, offset) {
                    Ok(count) => {
                        offset += count as off_t;
                        count
                    },
                    Err(error) => {
                        ::syslog::warn!("pwritev(): write failed (errno={:?})", error);
                        *__errno_location() = error.code.get();
                        return Err(error);
                    },
                }
            } else {
                iov_len as c_size_t
            };

            // Guard against ssize_t overflow (POSIX: EINVAL).
            total = match total.checked_add(count) {
                Some(sum) if sum <= SSIZE_MAX as c_size_t => sum,
                _ => {
                    ::syslog::warn!("pwritev(): iovec length overflows SSIZE_MAX");
                    return Err(Error::new(ErrorCode::InvalidArgument, "iovec length overflow"));
                },
            };
        }

        Ok(total)
    };

    // Write in dry-mode run first and parse result.
    match do_writev(true) {
        // Dry-mode run was successful, now write for real.
        Ok(_count) => {
            match do_writev(false) {
                // Real write was successful.
                Ok(count) => count as c_ssize_t,
                // Real write failed.
                Err(error) => {
                    ::syslog::warn!("pwritev(): write failed (errno={:?})", error);
                    *__errno_location() = error.code.get();
                    -1
                },
            }
        },
        // Dry-mode run failed because some other error.
        Err(error) => {
            ::syslog::warn!("pwritev(): dry-run failed (errno={:?})", error);
            *__errno_location() = error.code.get();
            -1
        },
    }
}

///
/// # Description
///
/// Reads data from a file descriptor.
///
/// # Parameters
///
/// - `fd`: File descriptor.
/// - `iov`: Pointer to an array of `iovec` structures.
/// - `iovcnt`: Number of elements in the array.
/// - `offset`: Offset to read from.
///
/// # Returns
///
/// Upon successful completion, `preadv()` returns the number of bytes read. Otherwise, it
/// returns `-1` and sets `errno` to indicate the error.
///
/// # Safety
///
/// This function is unsafe because:
/// - It may dereference raw pointers.
/// - It may access global variables.
///
/// It is safe to call this function if and only if the following conditions are met:
/// // - The `iov` pointer is valid and points to an array of `iovec` structures.
/// // - This function is called from multiple threads at the same time.
///
#[unsafe(no_mangle)]
#[trace_syscall]
pub unsafe extern "C" fn preadv(
    fd: i32,
    iov: *const iovec,
    iovcnt: i32,
    offset: off_t,
) -> c_ssize_t {
    // Check if number of elements in the vector is valid.
    if (iovcnt < 0) || (iovcnt > IOV_MAX as i32) {
        ::syslog::warn!("preadv(): invalid iovcnt {iovcnt}");
        *__errno_location() = ErrorCode::InvalidArgument.get();
        return -1;
    }

    // Check if vector base is invalid.
    if iov.is_null() {
        ::syslog::warn!("preadv(): invalid iov {iov:?}");
        *__errno_location() = ErrorCode::InvalidArgument.get();
        return -1;
    }

    // Check for zero-length vector.
    if iovcnt == 0 {
        return 0;
    }

    let do_preadv = |dry_run: bool| -> Result<c_size_t, Error> {
        let mut offset: off_t = offset;
        let mut total: c_size_t = 0;

        // Traverse i/o vector.
        for i in 0..iovcnt {
            let iov: *const iovec = unsafe { iov.offset(i as isize) };

            // Check if base address is invalid.
            if iov.is_null() {
                ::syslog::warn!("preadv(): invalid iov {iov:?}");
                return Err(Error::new(ErrorCode::InvalidArgument, "invalid iov"));
            }

            let iov_base: *mut u8 = unsafe { (*iov).iov_base };
            let iov_len: size_t = unsafe { (*iov).iov_len };

            // Check if base address is invalid.
            if iov_base.is_null() {
                ::syslog::warn!("preadv(): invalid iov_base {iov_base:?}");
                return Err(Error::new(ErrorCode::InvalidArgument, "invalid iov_base"));
            }

            total += if !dry_run {
                let buffer: &mut [u8] =
                    unsafe { slice::from_raw_parts_mut(iov_base, iov_len as usize) };

                // Read data and check if read failed.
                match unistd::pread(fd, buffer, offset) {
                    Ok(count) => {
                        offset += count as off_t;
                        count as c_size_t
                    },
                    Err(error) => {
                        ::syslog::warn!("preadv(): read failed (errno={:?})", error);
                        *__errno_location() = error.code.get();
                        return Err(error);
                    },
                }
            } else {
                iov_len as c_size_t
            };
        }

        Ok(total)
    };

    // Perform a dry-run first.
    match do_preadv(true) {
        Ok(_count) => {
            // Perform the actual read.
            match do_preadv(false) {
                Ok(count) => count as c_ssize_t,
                Err(error) => {
                    ::syslog::warn!("preadv(): read failed (errno={:?})", error);
                    *__errno_location() = error.code.get();
                    -1
                },
            }
        },
        // Dry-run failed because some other error.
        Err(error) => {
            ::syslog::warn!("preadv(): dry-run failed (errno={:?})", error);
            *__errno_location() = error.code.get();
            -1
        },
    }
}

///
/// # Description
///
/// Reads a vector of data from a file.
///
/// # Parameters
///
/// - `fd`: File descriptor.
/// - `iov`: Pointer to an array of `iovec` structures.
/// - `iovcnt`: Number of elements in the array.
///
/// # Returns
///
/// Upon successful completion, `readv()` returns the number of bytes read. Otherwise, it
/// returns `-1` and sets `errno` to indicate the error.
///
/// # Safety
///
/// This function is unsafe because:
/// - It may dereference raw pointers.
/// - It may access global variables.
///
/// It is safe to call this function if and only if the following conditions are met:
/// - The `iov` pointer is valid and points to an array of `iovec` structures.
/// - This function is called from multiple threads at the same time.
///
#[unsafe(no_mangle)]
#[trace_syscall]
pub unsafe extern "C" fn readv(fd: i32, iov: *const iovec, iovcnt: i32) -> c_ssize_t {
    // Check if number of elements in the vector is valid.
    if (iovcnt < 0) || (iovcnt > IOV_MAX as i32) {
        ::syslog::warn!("readv(): invalid iovcnt {iovcnt}");
        *__errno_location() = ErrorCode::InvalidArgument.get();
        return -1;
    }

    // Check if vector base is invalid.
    if iov.is_null() {
        ::syslog::warn!("readv(): invalid iov {iov:?}");
        *__errno_location() = ErrorCode::InvalidArgument.get();
        return -1;
    }

    // Check for zero-length vector.
    if iovcnt == 0 {
        return 0;
    }

    let do_readv = |dry_run: bool| -> Result<c_size_t, Error> {
        let mut total: c_size_t = 0;

        // Traverse i/o vector.
        for i in 0..iovcnt {
            let iov: *const iovec = unsafe { iov.offset(i as isize) };

            // Check if base address is invalid.
            if iov.is_null() {
                ::syslog::warn!("readv(): invalid iov {iov:?}");
                return Err(Error::new(ErrorCode::InvalidArgument, "invalid iov"));
            }

            let iov_base: *mut u8 = unsafe { (*iov).iov_base };
            let iov_len: size_t = unsafe { (*iov).iov_len };

            // Check if base address is invalid.
            if iov_base.is_null() {
                ::syslog::warn!("readv(): invalid iov_base {iov_base:?}");
                return Err(Error::new(ErrorCode::InvalidArgument, "invalid iov_base"));
            }

            total += if !dry_run {
                let buffer: &mut [u8] =
                    unsafe { slice::from_raw_parts_mut(iov_base, iov_len as usize) };

                // Read data and check if read failed.
                match unistd::read(fd, buffer) {
                    Ok(count) => count,
                    Err(error) => {
                        return Err(error);
                    },
                }
            } else {
                iov_len as c_size_t
            }
        }

        Ok(total)
    };

    // Perform a dry-run first.
    match do_readv(true) {
        Ok(_count) => {
            // Perform the actual read.
            match do_readv(false) {
                Ok(count) => count as c_ssize_t,
                Err(error) => {
                    ::syslog::warn!("readv(): read failed (errno={:?})", error);
                    *__errno_location() = error.code.get();
                    -1
                },
            }
        },
        // Dry-run failed because some other error.
        Err(error) => {
            ::syslog::warn!("readv(): dry-run failed (errno={:?})", error);
            *__errno_location() = error.code.get();
            -1
        },
    }
}

///
/// # Description
///
/// Performs a *gather* write: writes data from a list of `iovcnt` buffers (an `iovec`
/// array) to the file referred to by `fd`, advancing the file offset. The buffers are
/// written in array order, as if by a single `write()` of the concatenated data.
///
/// # Standard
///
/// `writev()`: <https://pubs.opengroup.org/onlinepubs/9799919799/functions/writev.html>
///
/// # Parameters
///
/// - `fd`: File descriptor.
/// - `iov`: Pointer to an array of `iovec` structures.
/// - `iovcnt`: Number of elements in the array.
///
/// # Returns
///
/// Returns a `c_ssize_t`:
/// - On success: the number of bytes written (a non-negative `c_ssize_t`).
/// - On failure: `-1`, with `errno` set to indicate the error. In particular, if the
///   sum of the `iov_len` values overflows `SSIZE_MAX`, `errno` is set to `EINVAL`.
///
/// # Safety
///
/// This function is unsafe because:
/// - It may dereference raw pointers.
/// - It may access global variables.
///
/// It is safe to call this function if and only if the following conditions are met:
/// - The `iov` pointer is valid and points to an array of `iovec` structures.
/// - This function is called from multiple threads at the same time.
///
#[unsafe(no_mangle)]
#[trace_syscall]
pub unsafe extern "C" fn writev(fd: c_int, iov: *const iovec, iovcnt: c_int) -> c_ssize_t {
    // Check if number of elements in the vector is valid.
    if iovcnt < 0 {
        ::syslog::warn!("writev(): invalid iovcnt {iovcnt}");
        *__errno_location() = ErrorCode::InvalidArgument.get();
        return -1;
    }

    // Check if vector base is invalid.
    if iov.is_null() {
        ::syslog::warn!("writev(): invalid iov {iov:?}");
        *__errno_location() = ErrorCode::InvalidArgument.get();
        return -1;
    }

    // Check for zero-length vector.
    if iovcnt == 0 {
        return 0;
    }

    let do_writev = |dry_run: bool| -> Result<c_size_t, Error> {
        let mut total: c_size_t = 0;

        // Traverse i/o vector.
        for i in 0..iovcnt {
            let iov: *const iovec = unsafe { iov.offset(i as isize) };

            // Check if iov is invalid.
            if iov.is_null() {
                ::syslog::warn!("writev(): invalid iov {iov:?}");
                return Err(Error::new(ErrorCode::InvalidArgument, "invalid iov"));
            }

            let iov_base: *mut u8 = unsafe { (*iov).iov_base };
            let iov_len: size_t = unsafe { (*iov).iov_len };

            // Check if `iov_base` is invalid.
            if iov_base.is_null() {
                ::syslog::warn!("writev(): invalid iov_base {iov_base:?}");
                return Err(Error::new(ErrorCode::InvalidArgument, "invalid iov_base"));
            }

            // Check if `iov_len` is invalid.
            if iov_len == 0 {
                ::syslog::warn!("writev(): invalid iov_len {iov_len}");
                return Err(Error::new(ErrorCode::InvalidArgument, "invalid iov_len"));
            }

            // Copy data only if not running in dry-run mode.
            let count: c_size_t = if !dry_run {
                // Construct buffer from raw parts.
                let buffer: &[u8] = slice::from_raw_parts(iov_base as *const u8, iov_len as usize);
                // Write data and parse result.
                match unistd::syscall::write(fd, buffer) {
                    Ok(count) => count,
                    Err(error) => {
                        ::syslog::warn!("writev(): write failed (errno={:?})", error);
                        *__errno_location() = error.code.get();
                        return Err(error);
                    },
                }
            } else {
                iov_len as c_size_t
            };

            // Guard against ssize_t overflow (POSIX: EINVAL).
            total = match total.checked_add(count) {
                Some(sum) if sum <= SSIZE_MAX as c_size_t => sum,
                _ => {
                    ::syslog::warn!("writev(): iovec length overflows SSIZE_MAX");
                    return Err(Error::new(ErrorCode::InvalidArgument, "iovec length overflow"));
                },
            };
        }

        Ok(total)
    };

    // Write in dry-mode run first and parse result.
    match do_writev(true) {
        // Dry-mode run was successful, now write for real.
        Ok(_count) => {
            match do_writev(false) {
                // Real write was successful.
                Ok(count) => count as c_ssize_t,
                // Real write failed.
                Err(error) => {
                    ::syslog::warn!("writev(): write failed (errno={:?})", error);
                    *__errno_location() = error.code.get();
                    -1
                },
            }
        },
        // Dry-mode run failed because some other error.
        Err(error) => {
            ::syslog::warn!("writev(): dry-run failed (errno={:?})", error);
            *__errno_location() = error.code.get();
            -1
        },
    }
}
