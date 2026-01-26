// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::{
    errno::__errno_location,
    sys::socket::SocketAddr,
    ErrorCode,
};
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
/// Binds a socket to a local address. When a socket is created with `socket()`, it exists in a name
/// space (address family) but has no address assigned to it. The `bind()` function assigns the
/// address specified by `sockaddr` to the socket referred to by the file descriptor `sockfd`.
///
/// # Parameters
///
/// - `sockfd`: File descriptor of the socket to bind.
/// - `sockaddr`: Pointer to a socket address structure that specifies the address to bind to.
/// - `len`: Size of the socket address structure pointed to by `sockaddr`.
///
/// # Returns
///
/// The `bind()` function returns `0` on success. On error, it returns `-1` and sets `errno` to
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
pub unsafe extern "C" fn bind(sockfd: c_int, sockaddr: *const sockaddr, len: socklen_t) -> c_int {
    // Check if sock address is invalid.
    if sockaddr.is_null() {
        ::syslog::error!(
            "bind(): invalid socket address (sockfd={sockfd:?}, sockaddr={sockaddr:?}, \
             len={len:?})"
        );
        *__errno_location() = ErrorCode::InvalidArgument.get();
        return -1;
    }

    // Attempt to convert socket address.
    let sockaddr: SocketAddr = match SocketAddr::try_from(unsafe { &*sockaddr }) {
        Ok(sockaddr) => sockaddr,
        Err(error) => {
            ::syslog::error!(
                "bind(): {error:?} (sockfd={sockfd:?}, sockaddr={sockaddr:?}, len={len:?})"
            );
            *__errno_location() = error.code.get();
            return -1;
        },
    };

    // Attempt to bind the socket and check for errors.
    match crate::sys::socket::syscall::bind(sockfd, &sockaddr) {
        Ok(()) => 0,
        Err(error) => {
            ::syslog::error!(
                "bind(): {error:?} (sockfd={sockfd:?}, sockaddr={sockaddr:?}, len={len:?})"
            );
            *__errno_location() = error.code.get();
            -1
        },
    }
}
