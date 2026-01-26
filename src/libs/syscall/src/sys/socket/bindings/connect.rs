// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::{
    errno::__errno_location,
    sys::{
        socket,
        socket::SocketAddr,
    },
};
use ::sys::error::ErrorCode;
use ::sysapi::{
    ffi::c_int,
    sys_socket::{
        sockaddr,
        socklen_t,
    },
};
use ::syslog::trace_syscall;

//==================================================================================================
// Standalone Functions
//==================================================================================================

///
/// # Description
///
/// Connects a socket to a remote address. The `connect()` function attempts to make a connection
/// on socket `sockfd` to the address specified by `sockaddr`. If the socket is of type `SOCK_DGRAM`,
/// `connect()` specifies the peer with which the socket is to be associated; this address is that
/// to which datagrams are to be sent and is the only address from which datagrams are received.
/// If the socket is of type `SOCK_STREAM`, `connect()` attempts to make a connection to another
/// socket.
///
/// # Parameters
///
/// - `sockfd`: File descriptor of the socket to connect.
/// - `sockaddr`: Pointer to a socket address structure that specifies the address to connect to.
/// - `len`: Size of the socket address structure pointed to by `sockaddr`.
///
/// # Returns
///
/// The `connect()` function returns `0` on success. On error, it returns `-1` and sets `errno` to
/// indicate the error.
///
/// # Safety
///
/// This function is unsafe because it may dereference raw pointers and modify global state.
///
/// It is safe to call this function if and only if all the following conditions are met:
/// - `sockaddr` points to a valid socket address structure.
/// - `sockaddr` remains valid for the duration of the function call.
/// - The memory pointed to by `sockaddr` must be at least `len` bytes in size.
/// - Access to `errno` is synchronized with other threads that may modify it.
///
#[unsafe(no_mangle)]
#[trace_syscall]
pub unsafe extern "C" fn connect(
    sockfd: c_int,
    sockaddr: *const sockaddr,
    len: socklen_t,
) -> c_int {
    // Check if `sockaddr` is valid.
    if sockaddr.is_null() {
        let reason: &str = "invalid socket address";
        ::syslog::error!(
            "connect(): {reason} (sockfd={sockfd:?}, sockaddr={sockaddr:?}, len={len:?})"
        );
        *__errno_location() = ErrorCode::InvalidArgument.get();
        return -1;
    }

    // Check if `len` is valid.
    if len == 0 {
        let reason: &str = "invalid socket address length";
        ::syslog::error!(
            "connect(): {reason} (sockfd={sockfd:?}, sockaddr={sockaddr:?}, len={len:?})"
        );
        *__errno_location() = ErrorCode::InvalidArgument.get();
        return -1;
    }

    // Attempt to convert socket address.
    let sockaddr: SocketAddr = match TryFrom::<&sockaddr>::try_from(&*sockaddr) {
        Ok(sockaddr) => sockaddr,
        Err(error) => {
            ::syslog::error!(
                "connect(): {error:?} (sockfd={sockfd:?}, sockaddr={sockaddr:?}, len={len:?})"
            );
            *__errno_location() = error.code.get();
            return -1;
        },
    };

    // Attempt to connect the socket and check for errors.
    match socket::syscall::connect(sockfd, &sockaddr) {
        Ok(()) => 0,
        Err(error) => {
            ::syslog::error!(
                "connect(): {error:?} (sockfd={sockfd:?}, sockaddr={sockaddr:?}, len={len:?})"
            );
            *__errno_location() = error.code.get();
            -1
        },
    }
}
