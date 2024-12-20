// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::{
    limits,
    sys::{
        types::{
            size_t,
            ssize_t,
        },
        uio::iovec,
    },
    unistd,
};
use ::nvx::sys::error::ErrorCode;

//==================================================================================================
// Standalone Functions
//==================================================================================================

///
/// # Description
///
/// Writes a vector of data to a file.
///
/// # Parameters
///
/// - `fd`: File descriptor.
/// - `iov`: Vector of data to write.
/// - `iovcnt`: Number of elements in the vector.
///
/// # Returns
///
/// Upon successful completion, the number of bytes written is returned. Otherwise, a negative error
/// code is returned.
///
pub fn writev(fd: i32, iov: *const iovec, iovcnt: i32) -> ssize_t {
    // Check if number of elements in the vector is valid.
    if iovcnt < 0 {
        return (-ErrorCode::InvalidArgument.into_errno()) as ssize_t;
    }

    // Check if vector base is invalid.
    if iov.is_null() {
        return (-ErrorCode::InvalidArgument.into_errno()) as ssize_t;
    }

    // Check for zero-length vector.
    if iovcnt == 0 {
        return 0;
    }

    let do_writev = |dry_run: bool| -> ssize_t {
        let mut total: ssize_t = 0;

        // Traverse i/o vector.
        for i in 0..iovcnt {
            let iov: *const iovec = unsafe { iov.offset(i as isize) };

            // Check if iov is invalid.
            if iov.is_null() {
                return (-ErrorCode::InvalidArgument.into_errno()) as ssize_t;
            }

            let iov_base: *mut u8 = unsafe { (*iov).iov_base };
            let iov_len: size_t = unsafe { (*iov).iov_len };

            // Copy data only if not running in dry-run mode.
            total += if !dry_run {
                // Write data.
                let count = unistd::write(fd, iov_base, iov_len);

                // Check for errors.
                if count < 0 {
                    return ErrorCode::try_from(count)
                        .unwrap_or_else(|_| panic!("unknown error code {count}"))
                        .into_errno() as ssize_t;
                }

                count as ssize_t
            } else {
                iov_len as ssize_t
            };
        }

        total
    };

    // Write in dry-mode run first and parse result.
    match do_writev(true) {
        // Dry-mode run was successful, now write for real.
        count if count <= limits::SSIZE_MAX => do_writev(false),
        // Dry-mode run failed because write request is too large.
        count if count > limits::SSIZE_MAX => ErrorCode::InvalidArgument.into_errno() as ssize_t,
        // Dry-mode run failed because some other error.
        err if err < 0 => err,
        // Dry-mode run failed because of an unexpected return value.
        ret => unreachable!("unexpected return value {ret}"),
    }
}
