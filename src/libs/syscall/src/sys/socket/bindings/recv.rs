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
/// Receives a message from a socket. The `recv()` function receives data from a connected socket.
/// This function is normally used with connected sockets because it does not permit the application
/// to retrieve the source address of received data. For connectionless sockets, `recv()` is
/// equivalent to `recvfrom()` with a NULL `from` parameter. The `recv()` function blocks until data
/// is available unless the socket is configured for non-blocking operation.
///
/// # Parameters
///
/// - `sockfd`: File descriptor of the socket from which to receive data.
/// - `buf`: Pointer to the buffer where the received data will be stored.
/// - `len`: Maximum number of bytes to receive into the buffer.
/// - `flags`: Flags that modify the behavior of the receive operation. Currently, only `0` is
///   supported (no special flags).
///
/// # Returns
///
/// The `recv()` function returns the number of bytes received on success. On error, it returns `-1`
/// and sets `errno` to indicate the error. A return value of `0` indicates that the peer has
/// performed an orderly shutdown.
///
/// # Safety
///
/// This function is unsafe because it may dereference raw pointers and modify global state.
///
/// It is safe to call this function if and only if all the following conditions are met:
/// - `buf` points to a valid buffer of at least `len` bytes.
/// - `buf` remains valid for the duration of the function call.
/// - `buf` is properly aligned for byte access.
/// - Access to `errno` is synchronized with other threads that may modify it.
///
#[unsafe(no_mangle)]
#[trace_syscall]
pub unsafe extern "C" fn recv(
    sockfd: c_int,
    buf: *mut c_void,
    len: c_size_t,
    flags: c_int,
) -> c_ssize_t {
    // Check if `buf` is valid.
    if buf.is_null() {
        ::syslog::error!(
            "recv(): invalid buffer (sockfd={sockfd:?}, buf={buf:?}, len={len:?}, flags={flags:?})"
        );
        *__errno_location() = ErrorCode::InvalidArgument.get();
        return -1;
    }

    // Check if `len` is valid.
    if len == 0 {
        ::syslog::error!(
            "recv(): invalid buffer length (sockfd={sockfd:?}, buf={buf:?}, len={len:?}, \
             flags={flags:?})"
        );
        *__errno_location() = ErrorCode::InvalidArgument.get();
        return -1;
    }

    // Check if `flags` is valid.
    if flags != 0 {
        ::syslog::error!(
            "recv(): unsupported flags (sockfd={sockfd:?}, buf={buf:?}, len={len:?}, \
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
                "recv(): failed to convert length (sockfd={sockfd:?}, buf={buf:?}, len={len:?}, \
                 flags={flags:?})"
            );
            *__errno_location() = ErrorCode::InvalidArgument.get();
            return -1;
        },
    };

    // Attempt to convert buffer.
    let buf: &mut [u8] = unsafe { slice::from_raw_parts_mut(buf as *mut u8, len) };

    match socket::syscall::recv(sockfd, buf, flags) {
        Ok(bytes_received) => match bytes_received.try_into() {
            Ok(bytes_received) => bytes_received,
            Err(_error) => {
                ::syslog::error!(
                    "recv(): failed to convert bytes received (sockfd={sockfd:?}, buf={buf:?}, \
                     len={len:?}, flags={flags:?})"
                );
                *__errno_location() = ErrorCode::ValueOutOfRange.get();
                -1
            },
        },
        Err(error) => {
            ::syslog::error!(
                "recv(): {error:?} (sockfd={sockfd:?}, buf={buf:?}, len={len:?}, flags={flags:?})"
            );
            *__errno_location() = error.code.get();
            -1
        },
    }
}
