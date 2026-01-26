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
/// Gets the name of the peer socket. The `getpeername()` function retrieves the address of the peer
/// to which a socket is connected. This function is particularly useful for determining the remote
/// address of a connection after it has been established. For connected sockets, it returns the
/// address of the remote peer, while for connectionless sockets, the behavior depends on the
/// protocol implementation.
///
/// # Parameters
///
/// - `sockfd`: File descriptor of the socket for which to retrieve the peer address.
/// - `sockaddr`: Pointer to a socket address structure where the peer address will be stored.
/// - `len`: Pointer to the size of the socket address structure. On input, it specifies the size
///   of the structure pointed to by `sockaddr`. On output, it contains the actual size of the
///   address returned.
///
/// # Returns
///
/// The `getpeername()` function returns `0` on success. On error, it returns `-1` and sets `errno`
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
pub unsafe extern "C" fn getpeername(
    sockfd: c_int,
    sockaddr: *mut sockaddr,
    len: *mut socklen_t,
) -> c_int {
    // Check if the address is valid.
    if sockaddr.is_null() {
        ::syslog::error!(
            "getpeername(): invalid socket address (sockfd={sockfd:?}, sockaddr={sockaddr:?}, \
             len={len:?})"
        );
        *__errno_location() = ErrorCode::InvalidArgument.get();
        return -1;
    }

    // Check if the length is valid.
    if len.is_null() || (*len as usize) < mem::size_of::<sockaddr>() {
        ::syslog::error!(
            "getpeername(): invalid length (sockfd={sockfd:?}, sockaddr={sockaddr:?}, len={len:?})"
        );
        *__errno_location() = ErrorCode::InvalidArgument.get();
        return -1;
    }

    let mut sockaddr_: SocketAddr = SocketAddr::V4(Default::default());

    // Attempt to get the peer name and check for errors.
    match socket::syscall::getpeername(sockfd, &mut sockaddr_) {
        Ok(()) => {
            let (sockaddr_, len_): (sockaddr, socklen_t) = sockaddr_.into();
            *sockaddr = sockaddr_;
            *len = len_;
            0
        },
        Err(error) => {
            ::syslog::error!(
                "getpeername(): {error:?} (sockfd={sockfd:?}, sockaddr={sockaddr:?}, len={len:?})"
            );
            *__errno_location() = error.code.get();
            -1
        },
    }
}
