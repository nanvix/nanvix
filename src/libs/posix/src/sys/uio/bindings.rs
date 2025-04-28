// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::{
    errno::errno,
    ffi::c_int,
    limits,
    sys::{
        types::{
            off_t,
            ssize_t,
        },
        uio::iovec,
    },
    time::size_t,
    unistd,
};
use ::core::slice;
use ::nvx::sys::error::{
    Error,
    ErrorCode,
};

//==================================================================================================
// Standalone Functions
//==================================================================================================

///
/// # Description
///
/// Writes data to a file descriptor.
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
/// Upon successful completion, `pwritev()` returns the number of bytes written. Otherwise, it
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
#[no_mangle]
pub unsafe extern "C" fn pwritev(
    fd: i32,
    iov: *const iovec,
    iovcnt: c_int,
    offset: off_t,
) -> ssize_t {
    ::nvx::trace!("pwritev(): fd={fd}, iov={iov:?}, iovcnt={iovcnt}, offset={offset}");

    // Check if number of elements in the vector is valid.
    if iovcnt < 0 {
        ::nvx::error!("pwritev(): invalid iovcnt {iovcnt}");
        errno = ErrorCode::InvalidArgument.get();
        return -1;
    }

    // Check if vector base is invalid.
    if iov.is_null() {
        ::nvx::error!("pwritev(): invalid iov {iov:?}");
        errno = ErrorCode::InvalidArgument.get();
        return -1;
    }

    // Check for zero-length vector.
    if iovcnt == 0 {
        return 0;
    }

    let do_writev = |dry_run: bool| -> Result<size_t, Error> {
        let mut offset: off_t = offset;
        let mut total: size_t = 0;

        // Traverse i/o vector.
        for i in 0..iovcnt {
            let iov: *const iovec = unsafe { iov.offset(i as isize) };

            // Check if iov is invalid.
            if iov.is_null() {
                ::nvx::error!("pwritev(): invalid iov {iov:?}");
                return Err(Error::new(ErrorCode::InvalidArgument, "invalid iov"));
            }

            let iov_base: *mut u8 = unsafe { (*iov).iov_base };
            let iov_len: size_t = unsafe { (*iov).iov_len };

            // Check if `iov_base` is invalid.
            if iov_base.is_null() {
                ::nvx::error!("pwritev(): invalid iov_base {iov_base:?}");
                return Err(Error::new(ErrorCode::InvalidArgument, "invalid iov_base"));
            }

            // Check if `iov_len` is invalid.
            if iov_len == 0 {
                ::nvx::error!("pwritev(): invalid iov_len {iov_len}");
                return Err(Error::new(ErrorCode::InvalidArgument, "invalid iov_len"));
            }

            // Copy data only if not running in dry-run mode.
            total += if !dry_run {
                // Construct buffer from raw parts.
                let buffer: &[u8] = slice::from_raw_parts(iov_base as *const u8, iov_len as usize);
                // Write data and parse result.
                match unistd::syscall::pwrite(fd, buffer, offset) {
                    Ok(count) => {
                        offset += count as off_t;
                        count
                    },
                    Err(error) => {
                        ::nvx::error!("pwritev(): write failed (errno={:?})", error);
                        errno = error.code.get();
                        return Err(error);
                    },
                }
            } else {
                iov_len as size_t
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
                Ok(count) => count as ssize_t,
                // Real write failed.
                Err(error) => {
                    ::nvx::error!("pwritev(): write failed (errno={:?})", error);
                    errno = error.code.get();
                    -1
                },
            }
        },
        // Dry-mode run failed because some other error.
        Err(error) => {
            ::nvx::error!("pwritev(): dry-run failed (errno={:?})", error);
            errno = error.code.get();
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
#[no_mangle]
pub unsafe extern "C" fn preadv(fd: i32, iov: *const iovec, iovcnt: i32, offset: off_t) -> ssize_t {
    ::nvx::trace!("preadv(): fd={fd}, iov={iov:?}, iovcnt={iovcnt}, offset={offset}");

    // Check if number of elements in the vector is valid.
    if (iovcnt < 0) || (iovcnt > limits::IOV_MAX as i32) {
        ::nvx::error!("preadv(): invalid iovcnt {iovcnt}");
        errno = ErrorCode::InvalidArgument.get();
        return -1;
    }

    // Check if vector base is invalid.
    if iov.is_null() {
        ::nvx::error!("preadv(): invalid iov {iov:?}");
        errno = ErrorCode::InvalidArgument.get();
        return -1;
    }

    // Check for zero-length vector.
    if iovcnt == 0 {
        return 0;
    }

    let do_preadv = |dry_run: bool| -> Result<size_t, Error> {
        let mut offset: off_t = offset;
        let mut total: size_t = 0;

        // Traverse i/o vector.
        for i in 0..iovcnt {
            let iov: *const iovec = unsafe { iov.offset(i as isize) };

            // Check if base address is invalid.
            if iov.is_null() {
                ::nvx::error!("preadv(): invalid iov {iov:?}");
                return Err(Error::new(ErrorCode::InvalidArgument, "invalid iov"));
            }

            let iov_base: *mut u8 = unsafe { (*iov).iov_base };
            let iov_len: size_t = unsafe { (*iov).iov_len };

            // Check if base address is invalid.
            if iov_base.is_null() {
                ::nvx::error!("preadv(): invalid iov_base {iov_base:?}");
                return Err(Error::new(ErrorCode::InvalidArgument, "invalid iov_base"));
            }

            total += if !dry_run {
                let buffer: &mut [u8] =
                    unsafe { slice::from_raw_parts_mut(iov_base, iov_len as usize) };

                // Read data and check if read failed.
                match unistd::pread(fd, buffer, offset) {
                    Ok(count) => {
                        offset += count as off_t;
                        count as size_t
                    },
                    Err(error) => {
                        ::nvx::error!("preadv(): read failed (errno={:?})", error);
                        errno = error.code.get();
                        return Err(error);
                    },
                }
            } else {
                iov_len as size_t
            };
        }

        Ok(total)
    };

    // Perform a dry-run first.
    match do_preadv(true) {
        Ok(_count) => {
            // Perform the actual read.
            match do_preadv(false) {
                Ok(count) => count as ssize_t,
                Err(error) => {
                    ::nvx::error!("preadv(): read failed (errno={:?})", error);
                    errno = error.code.get();
                    -1
                },
            }
        },
        // Dry-run failed because some other error.
        Err(error) => {
            ::nvx::error!("preadv(): dry-run failed (errno={:?})", error);
            errno = error.code.get();
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
#[no_mangle]
pub unsafe extern "C" fn readv(fd: i32, iov: *const iovec, iovcnt: i32) -> ssize_t {
    ::nvx::trace!("readv(): fd={fd}, iov={iov:?}, iovcnt={iovcnt}");

    // Check if number of elements in the vector is valid.
    if (iovcnt < 0) || (iovcnt > limits::IOV_MAX as i32) {
        ::nvx::error!("readv(): invalid iovcnt {iovcnt}");
        errno = ErrorCode::InvalidArgument.get();
        return -1;
    }

    // Check if vector base is invalid.
    if iov.is_null() {
        ::nvx::error!("readv(): invalid iov {iov:?}");
        errno = ErrorCode::InvalidArgument.get();
        return -1;
    }

    // Check for zero-length vector.
    if iovcnt == 0 {
        return 0;
    }

    let do_readv = |dry_run: bool| -> Result<size_t, Error> {
        let mut total: size_t = 0;

        // Traverse i/o vector.
        for i in 0..iovcnt {
            let iov: *const iovec = unsafe { iov.offset(i as isize) };

            // Check if base address is invalid.
            if iov.is_null() {
                ::nvx::error!("readv(): invalid iov {iov:?}");
                return Err(Error::new(ErrorCode::InvalidArgument, "invalid iov"));
            }

            let iov_base: *mut u8 = unsafe { (*iov).iov_base };
            let iov_len: size_t = unsafe { (*iov).iov_len };

            // Check if base address is invalid.
            if iov_base.is_null() {
                ::nvx::error!("readv(): invalid iov_base {iov_base:?}");
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
                iov_len as size_t
            }
        }

        Ok(total)
    };

    // Perform a dry-run first.
    match do_readv(true) {
        Ok(_count) => {
            // Perform the actual read.
            match do_readv(false) {
                Ok(count) => count as ssize_t,
                Err(error) => {
                    ::nvx::error!("readv(): read failed (errno={:?})", error);
                    errno = error.code.get();
                    -1
                },
            }
        },
        // Dry-run failed because some other error.
        Err(error) => {
            ::nvx::error!("readv(): dry-run failed (errno={:?})", error);
            errno = error.code.get();
            -1
        },
    }
}

///
/// # Description
///
/// Writes data to a file descriptor.
///
/// # Parameters
///
/// - `fd`: File descriptor.
/// - `iov`: Pointer to an array of `iovec` structures.
/// - `iovcnt`: Number of elements in the array.
///
/// # Returns
///
/// Upon successful completion, `writev()` returns the number of bytes written. Otherwise, it
/// it returns `-1` and sets `errno` to indicate the error.
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
#[no_mangle]
pub unsafe extern "C" fn writev(fd: c_int, iov: *const iovec, iovcnt: c_int) -> ssize_t {
    ::nvx::trace!("writev(): fd={fd}, iov={iov:?}, iovcnt={iovcnt}");

    // Check if number of elements in the vector is valid.
    if iovcnt < 0 {
        ::nvx::error!("writev(): invalid iovcnt {iovcnt}");
        errno = ErrorCode::InvalidArgument.get();
        return -1;
    }

    // Check if vector base is invalid.
    if iov.is_null() {
        ::nvx::error!("writev(): invalid iov {iov:?}");
        errno = ErrorCode::InvalidArgument.get();
        return -1;
    }

    // Check for zero-length vector.
    if iovcnt == 0 {
        return 0;
    }

    let do_writev = |dry_run: bool| -> Result<size_t, Error> {
        let mut total: size_t = 0;

        // Traverse i/o vector.
        for i in 0..iovcnt {
            let iov: *const iovec = unsafe { iov.offset(i as isize) };

            // Check if iov is invalid.
            if iov.is_null() {
                ::nvx::error!("writev(): invalid iov {iov:?}");
                return Err(Error::new(ErrorCode::InvalidArgument, "invalid iov"));
            }

            let iov_base: *mut u8 = unsafe { (*iov).iov_base };
            let iov_len: size_t = unsafe { (*iov).iov_len };

            // Check if `iov_base` is invalid.
            if iov_base.is_null() {
                ::nvx::error!("writev(): invalid iov_base {iov_base:?}");
                return Err(Error::new(ErrorCode::InvalidArgument, "invalid iov_base"));
            }

            // Check if `iov_len` is invalid.
            if iov_len == 0 {
                ::nvx::error!("writev(): invalid iov_len {iov_len}");
                return Err(Error::new(ErrorCode::InvalidArgument, "invalid iov_len"));
            }

            // Copy data only if not running in dry-run mode.
            total += if !dry_run {
                // Construct buffer from raw parts.
                let buffer: &[u8] = slice::from_raw_parts(iov_base as *const u8, iov_len as usize);
                // Write data and parse result.
                match unistd::syscall::write(fd, buffer) {
                    Ok(count) => count,
                    Err(error) => {
                        ::nvx::error!("writev(): write failed (errno={:?})", error);
                        errno = error.code.get();
                        return Err(error);
                    },
                }
            } else {
                iov_len as size_t
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
                Ok(count) => count as ssize_t,
                // Real write failed.
                Err(error) => {
                    ::nvx::error!("writev(): write failed (errno={:?})", error);
                    errno = error.code.get();
                    -1
                },
            }
        },
        // Dry-mode run failed because some other error.
        Err(error) => {
            ::nvx::error!("writev(): dry-run failed (errno={:?})", error);
            errno = error.code.get();
            -1
        },
    }
}
