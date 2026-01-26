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
use ::core::mem;
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
/// Gets the name of the socket. The `getsockname()` function retrieves the local address to which
/// the socket is bound. This function is particularly useful for determining the local address
/// assigned to a socket, especially when the socket was bound with a wildcard address (such as
/// `INADDR_ANY`) or when the port number was automatically assigned by the system. For unbound
/// sockets, the behavior depends on the address family and protocol implementation.
///
/// # Parameters
///
/// - `sockfd`: File descriptor of the socket for which to retrieve the local address.
/// - `sockaddr`: Pointer to a socket address structure where the local address will be stored.
/// - `len`: Pointer to the size of the socket address structure. On input, it specifies the size
///   of the structure pointed to by `sockaddr`. On output, it contains the actual size of the
///   address returned.
///
/// # Returns
///
/// The `getsockname()` function returns `0` on success. On error, it returns `-1` and sets `errno`
/// to indicate the error.
///
/// # Safety
///
/// This function is unsafe because it may dereference raw pointers and modify global state.
///
/// It is safe to call this function if and only if all the following conditions are met:
/// - `sockaddr` points to a valid socket address structure.
/// - `len` points to a valid `socklen_t` value.
/// - `sockaddr` and `len` remain valid for the duration of the function call.
/// - The memory pointed to by `sockaddr` must be at least `*len` bytes in size.
/// - Access to `errno` is synchronized with other threads that may modify it.
///
#[unsafe(no_mangle)]
#[trace_syscall]
pub unsafe extern "C" fn getsockname(
    sockfd: c_int,
    sockaddr: *mut sockaddr,
    len: *mut socklen_t,
) -> c_int {
    // Check if the address is valid.
    if sockaddr.is_null() {
        ::syslog::error!(
            "getsockname(): invalid socket address (sockfd={sockfd:?}, sockaddr={sockaddr:?}, \
             len={len:?})"
        );
        *__errno_location() = ErrorCode::InvalidArgument.get();
        return -1;
    }

    // Check if the length is valid.
    if len.is_null() || *len < mem::size_of::<sockaddr>() as socklen_t {
        ::syslog::error!(
            "getsockname(): invalid socket address length (sockfd={sockfd:?}, \
             sockaddr={sockaddr:?}, len={len:?})"
        );
        *__errno_location() = ErrorCode::InvalidArgument.get();
        return -1;
    }

    let mut sockaddr_: SocketAddr = SocketAddr::V4(Default::default());

    // Attempt to get the socket name and check for errors.
    match socket::syscall::getsockname(sockfd, &mut sockaddr_) {
        Ok(_) => {
            let (sockaddr_, len_): (sockaddr, socklen_t) = sockaddr_.into();
            *sockaddr = sockaddr_;
            *len = len_;
            0
        },
        Err(error) => {
            ::syslog::error!(
                "getsockname(): {error:?} (sockfd={sockfd:?}, sockaddr={sockaddr:?}, len={len:?})"
            );
            *__errno_location() = error.code.get();
            -1
        },
    }
}
