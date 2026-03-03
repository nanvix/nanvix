// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::errno::__errno_location;
use ::sys::error::ErrorCode;
use ::sysapi::{
    ffi::{
        c_int,
        c_void,
    },
    sys_socket::{
        sockaddr,
        socklen_t,
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
/// Receives data from a socket and stores the source address. The `recvfrom()` function receives
/// data from a socket, regardless of whether it is connection-oriented or connectionless. For
/// connectionless sockets, `recvfrom()` allows the application to retrieve the source address of
/// the received data. For connection-oriented sockets, the source address is filled in but is
/// typically not needed since the peer address is already known. This function blocks until data is
/// available unless the socket is configured for non-blocking operation.
///
/// # Parameters
///
/// - `sockfd`: File descriptor of the socket from which to receive data.
/// - `buf`: Pointer to the buffer where the received data will be stored.
/// - `len`: Maximum number of bytes to receive into the buffer.
/// - `flags`: Flags that modify the behavior of the receive operation.
/// - `sockaddr`: Pointer to a socket address structure where the source address will be stored.
///   Can be NULL if the source address is not needed.
/// - `addrlen`: Pointer to the size of the socket address structure. On input, it specifies the
///   size of the structure pointed to by `sockaddr`. On output, it contains the actual size of
///   the address returned. Can be NULL if `sockaddr` is NULL.
///
/// # Returns
///
/// The `recvfrom()` function returns the number of bytes received on success. On error, it returns
/// `-1` and sets `errno` to indicate the error. A return value of `0` indicates that the peer has
/// performed an orderly shutdown.
///
/// # Safety
///
/// This function is unsafe because it may dereference raw pointers and modify global state.
///
/// It is safe to call this function if and only if all the following conditions are met:
/// - `buf` points to a valid buffer of at least `len` bytes.
/// - `buf` remains valid for the duration of the function call.
/// - If `sockaddr` is not NULL, it points to a valid socket address structure.
/// - If `sockaddr` is not NULL, `addrlen` must not be NULL and must point to a valid `socklen_t`.
/// - If `sockaddr` is not NULL, the memory pointed to by `sockaddr` must be at least `*addrlen` bytes.
/// - `sockaddr` and `addrlen` remain valid for the duration of the function call.
/// - Access to `errno` is synchronized with other threads that may modify it.
///
#[unsafe(no_mangle)]
#[trace_syscall]
#[allow(unreachable_code)]
pub unsafe extern "C" fn recvfrom(
    sockfd: c_int,
    buf: *mut c_void,
    len: c_size_t,
    flags: c_int,
    sockaddr: *mut sockaddr,
    addrlen: *mut socklen_t,
) -> c_ssize_t {
    #[cfg(feature = "standalone")]
    {
        let _ = (sockfd, buf, len, flags, sockaddr, addrlen);
        *__errno_location() = ::sys::error::ErrorCode::OperationNotSupported.get();
        return -1;
    }

    // TODO: https://github.com/nanvix/nanvix/issues/590
    ::syslog::debug!("recvfrom(): not implemented");
    unsafe {
        *__errno_location() = ErrorCode::InvalidSysCall.get();
    }
    -1
}
