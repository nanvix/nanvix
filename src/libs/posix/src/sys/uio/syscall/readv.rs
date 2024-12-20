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
/// Reads a vector of data from a file.
pub fn readv(fd: i32, iov: *const iovec, iovcnt: i32) -> ssize_t {
    // Check if number of elements in the vector is valid.
    if (iovcnt < 0) || (iovcnt > limits::IOV_MAX as i32) {
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

    let mut total: ssize_t = 0;

    // Traverse i/o vector.
    for i in 0..iovcnt {
        let iov: *const iovec = unsafe { iov.offset(i as isize) };

        // Check if base address is invalid.
        if iov.is_null() {
            return (-ErrorCode::InvalidArgument.into_errno()) as ssize_t;
        }

        let iov_base: *mut u8 = unsafe { (*iov).iov_base };
        let iov_len: size_t = unsafe { (*iov).iov_len };

        // Check if base address is invalid.
        if iov_base.is_null() {
            return (-ErrorCode::InvalidArgument.into_errno()) as ssize_t;
        }

        // Read data.
        let count: ssize_t = unistd::read(fd, iov_base, iov_len);

        // Check if read failed.
        if count < 0 {
            return count;
        }

        total += count;
    }

    total
}
