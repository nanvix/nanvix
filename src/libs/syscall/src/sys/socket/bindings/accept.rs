// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::{
    errno::__errno_location,
    sys::socket::SocketAddr,
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
/// Accepts a connection on a socket. The `accept()` function extracts the first connection request
/// on the queue of pending connections for the listening socket, `sockfd`, creates a new connected
/// socket, and returns a new file descriptor referring to that socket. The newly created socket is
/// not in the listening state. The original socket `sockfd` is unaffected by this call.
///
/// # Parameters
///
/// - `sockfd`: File descriptor of the listening socket.
/// - `sockaddr`: Pointer to a socket address structure where the address of the connecting entity
///   will be stored. If this is `NULL`, no address information is returned.
/// - `len`: Pointer to the size of the socket address structure. On input, it specifies the size
///   of the structure pointed to by `sockaddr`. On output, it contains the actual size of the
///   address returned. If `sockaddr` is `NULL`, this parameter is ignored and may be `NULL`.
///
/// # Returns
///
/// The `accept()` function returns a non-negative file descriptor for the new socket on success.
/// On error, it returns `-1` and sets `errno` to indicate the error.
///
/// # Safety
///
/// This function is unsafe because it may dereference raw pointers and modify global state.
///
/// It is safe to call this function if and only if all the following conditions are met:
/// - If `sockaddr` is not `NULL`, it must point to a valid socket address structure.
/// - If `sockaddr` is not `NULL`, `len` must not be `NULL` and must point to a valid `socklen_t`.
/// - If `sockaddr` is not `NULL`, the memory pointed to by `sockaddr` must be at least `*len` bytes in size.
/// - `sockaddr` and `len` must remain valid for the duration of the function call.
/// - Access to `errno` is synchronized with other threads that may modify it.
///
#[unsafe(no_mangle)]
#[trace_syscall]
pub unsafe extern "C" fn accept(
    sockfd: c_int,
    sockaddr: *mut sockaddr,
    len: *mut socklen_t,
) -> c_int {
    // Check if sockaddr and len is invalid.
    if !sockaddr.is_null() && len.is_null() {
        ::syslog::error!(
            "accept(): invalid length pointer (sockfd={sockfd:?}, sockaddr={sockaddr:?}, \
             len={len:?})"
        );
    }

    // Accept connection and check for errors.
    match crate::sys::socket::syscall::accept(sockfd) {
        Ok((sockfd, sockaddr_)) => {
            // Store socket address, if requested.
            let (sockaddr_, len_) = From::<&SocketAddr>::from(&sockaddr_);
            if !sockaddr.is_null() {
                *sockaddr = sockaddr_;
            }

            if !len.is_null() {
                *len = len_;
            }

            sockfd
        },
        Err(error) => {
            ::syslog::error!(
                "accept(): {error:?} (sockfd={sockfd:?}, sockaddr={sockaddr:?}, len={len:?})"
            );
            *__errno_location() = error.code.get();
            -1
        },
    }
}
