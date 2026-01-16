// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::errno::__errno_location;
use ::sysapi::ffi::c_int;
use ::syslog::trace_syscall;

//==================================================================================================
// Standalone Functions
//==================================================================================================

///
/// # Description
///
/// Marks the socket referred to by `sockfd` as a passive socket, that is, as a socket that will be
/// used to accept incoming connection requests using `accept()`. The `listen()` function applies
/// only to sockets of type `SOCK_STREAM` or `SOCK_SEQPACKET`. For connection-oriented protocols,
/// `listen()` causes the socket to enter the listening state, where it can accept incoming
/// connections. The socket must be bound to a local address using `bind()` before calling `listen()`.
///
/// # Parameters
///
/// - `sockfd`: File descriptor of the socket to mark as passive. The socket must be of type
///   `SOCK_STREAM` or `SOCK_SEQPACKET` and must be bound to a local address.
/// - `backlog`: Maximum length to which the queue of pending connections for `sockfd` may grow.
///   If a connection request arrives when the queue is full, the client may receive an error with
///   an indication of `ECONNREFUSED` or, if the underlying protocol supports retransmission, the
///   request may be ignored so that a later reattempt at connection succeeds.
///
/// # Returns
///
/// The `listen()` function returns `0` on success. On error, it returns `-1` and sets `errno` to
/// indicate the error. Common error conditions include invalid socket descriptor, socket not
/// bound to an address, or socket already connected.
///
/// # Safety
///
/// This function is unsafe because it may modify global state.
///
/// It is safe to call this function if and only if all the following conditions are met:
/// - Access to `errno` is synchronized with other threads that may modify it.
///
#[unsafe(no_mangle)]
#[trace_syscall]
pub unsafe extern "C" fn listen(sockfd: c_int, backlog: c_int) -> c_int {
    // Call listen and check for errors.
    match crate::sys::socket::syscall::listen(sockfd, backlog) {
        Ok(()) => 0,
        Err(error) => {
            ::syslog::error!("listen(): {error:?} (sockfd={sockfd:?}, backlog={backlog:?})");
            *__errno_location() = error.code.get();
            -1
        },
    }
}
