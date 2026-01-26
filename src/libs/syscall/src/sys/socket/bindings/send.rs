// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::{
    errno::__errno_location,
    sys::socket,
};
use ::core::slice;
use ::sys::error::ErrorCode;
use ::sysapi::{
    ffi::{
        c_int,
        c_void,
    },
    sys_types::{
        c_size_t,
        c_ssize_t,
    },
};
use ::syslog::trace_syscall;

//==================================================================================================
// Standalone Functions
//==================================================================================================

///
/// # Description
///
/// Sends a message on a socket. The `send()` function sends data on a connected socket. This
/// function is normally used with connected sockets because it does not allow the application to
/// specify a destination address. For connectionless sockets, `send()` is equivalent to `sendto()`
/// with the destination address set to the connected peer. The `send()` function may block if the
/// socket send buffer is full, unless the socket is configured for non-blocking operation.
///
/// # Parameters
///
/// - `sockfd`: File descriptor of the socket on which to send data.
/// - `buf`: Pointer to the buffer containing the data to be sent.
/// - `len`: Number of bytes to send from the buffer. A length of `0` is valid and results in
///   sending a zero-length message.
/// - `flags`: Flags that modify the behavior of the send operation. Currently, only `0` is
///   supported (no special flags).
///
/// # Returns
///
/// The `send()` function returns the number of bytes sent on success. On error, it returns `-1`
/// and sets `errno` to indicate the error. The number of bytes sent may be less than the
/// requested amount if the socket send buffer becomes full.
///
/// # Safety
///
/// This function is unsafe because it may dereference raw pointers and modify global state.
///
/// It is safe to call this function if and only if all the following conditions are met:
/// - `buf` points to a valid buffer containing at least `len` bytes of data.
/// - `buf` remains valid for the duration of the function call.
/// - Access to `errno` is synchronized with other threads that may modify it.
///
#[unsafe(no_mangle)]
#[trace_syscall]
pub unsafe extern "C" fn send(
    sockfd: c_int,
    buf: *const c_void,
    len: c_size_t,
    flags: c_int,
) -> c_ssize_t {
    // Check if `buf` is valid.
    if buf.is_null() {
        ::syslog::error!(
            "send(): invalid buffer (sockfd={sockfd:?}, buf={buf:?}, len={len:?}, flags={flags:?})"
        );
        *__errno_location() = ErrorCode::InvalidArgument.get();
        return -1;
    }

    // NOTE: no check on len because POSIX allows it to be zero.

    // Check if `flags` is valid.
    if flags != 0 {
        ::syslog::error!(
            "send(): unsupported flags (sockfd={sockfd:?}, buf={buf:?}, len={len:?}, \
             flags={flags:?})"
        );
        *__errno_location() = ErrorCode::InvalidArgument.get();
        return -1;
    }

    // Attempt to convert `len` to `usize`.
    let len: usize = match len.try_into() {
        Ok(len) => len,
        Err(_error) => {
            ::syslog::error!(
                "send(): failed to convert length (sockfd={sockfd:?}, buf={buf:?}, len={len:?}, \
                 flags={flags:?})"
            );
            *__errno_location() = ErrorCode::InvalidArgument.get();
            return -1;
        },
    };

    // Attempt to convert buffer.
    let buf: &[u8] = unsafe { slice::from_raw_parts(buf as *const u8, len) };

    // Attempt to send data and check for errors.
    match socket::syscall::send(sockfd, buf, flags) {
        Ok(bytes_sent) => match bytes_sent.try_into() {
            Ok(bytes_sent) => bytes_sent,
            Err(_error) => {
                ::syslog::error!(
                    "send(): failed to convert bytes sent (sockfd={sockfd:?}, buf={buf:?}, \
                     len={len:?}, flags={flags:?})"
                );
                *__errno_location() = ErrorCode::ValueOutOfRange.get();
                -1
            },
        },
        Err(error) => {
            ::syslog::error!(
                "send(): {error:?} (sockfd={sockfd:?}, buf={buf:?}, len={len:?}, flags={flags:?})"
            );
            *__errno_location() = error.code.get();
            -1
        },
    }
}
