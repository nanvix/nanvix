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
/// Sends data to a specific address on a socket. The `sendto()` function sends data on a socket and
/// allows the application to specify the destination address. This function is typically used with
/// connectionless sockets (such as UDP) where each message can be sent to a different destination.
/// For connection-oriented sockets, the destination address is usually ignored since the connection
/// determines the peer. The `sendto()` function may block if the socket send buffer is full, unless
/// the socket is configured for non-blocking operation.
///
/// # Parameters
///
/// - `sockfd`: File descriptor of the socket on which to send data.
/// - `buf`: Pointer to the buffer containing the data to be sent.
/// - `len`: Number of bytes to send from the buffer. A length of `0` is valid and results in
///   sending a zero-length message.
/// - `flags`: Flags that modify the behavior of the send operation (e.g., MSG_DONTWAIT,
///   MSG_NOSIGNAL).
/// - `sockaddr`: Pointer to a socket address structure specifying the destination address.  Can be
///   NULL for connected sockets.
/// - `addrlen`: Size of the socket address structure pointed to by `sockaddr`. Ignored if
///   `sockaddr` is NULL.
///
/// # Returns
///
/// The `sendto()` function returns the number of bytes sent on success. On error, it returns `-1`
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
/// - If `sockaddr` is not NULL, it points to a valid socket address structure.
/// - If `sockaddr` is not NULL, the memory pointed to by `sockaddr` must be at least `addrlen`
///   bytes.
/// - `sockaddr` remains valid for the duration of the function call.
/// - Access to `errno` is synchronized with other threads that may modify it.
///
#[unsafe(no_mangle)]
#[trace_syscall]
#[allow(unreachable_code)]
pub unsafe extern "C" fn sendto(
    sockfd: c_int,
    buf: *const c_void,
    len: c_size_t,
    flags: c_int,
    sockaddr: *const sockaddr,
    addrlen: socklen_t,
) -> c_ssize_t {
    #[cfg(feature = "standalone")]
    {
        let _ = (sockfd, buf, len, flags, sockaddr, addrlen);
        *__errno_location() = ::sys::error::ErrorCode::OperationNotSupported.get();
        return -1;
    }

    // TODO: https://github.com/nanvix/nanvix/issues/589
    ::syslog::debug!("sendto(): not implemented");
    unsafe {
        *__errno_location() = ErrorCode::InvalidSysCall.get();
    }
    -1
}
