// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::{
    errno::__errno_location,
    sys::{
        socket,
        socket::Shutdown,
    },
};
use ::sys::error::ErrorCode;
use ::sysapi::ffi::c_int;
use ::syslog::trace_syscall;

//==================================================================================================
// Standalone Functions
//==================================================================================================

///
/// # Description
///
/// Shuts down part or all of a full-duplex connection on a socket. The `shutdown()` function
/// disables subsequent send and/or receive operations on a socket, depending on the value of the
/// `how` parameter. Unlike `close()`, which immediately terminates the socket, `shutdown()` allows
/// for graceful termination of communication in one or both directions while keeping the socket
/// file descriptor open. This function is particularly useful for protocols that require orderly
/// shutdown procedures, such as TCP, where it can signal the end of data transmission to the peer.
///
/// # Parameters
///
/// - `sockfd`: File descriptor of the socket to shut down. The socket must be a valid, open socket
///   descriptor previously created with `socket()` or `socketpair()`.
/// - `how`: Specifies which direction of the socket communication to shut down. Valid values are:
///   - `SHUT_RD` (0): Disables further receive operations. The socket will not accept any more
///     incoming data.
///   - `SHUT_WR` (1): Disables further send operations. No more data can be sent through the
///     socket.
///   - `SHUT_RDWR` (2): Disables both send and receive operations. This is equivalent to calling
///     `shutdown()` twice with `SHUT_RD` and `SHUT_WR`.
///
/// # Returns
///
/// The `shutdown()` function returns `0` on success. On error, it returns `-1` and sets `errno` to
/// indicate the error. A successful return indicates that the shutdown operation has been
/// initiated, but it does not guarantee that all data has been transmitted or received.
///
/// # Safety
///
/// This function is unsafe because it may modify global state.
///
/// It is safe to call this function if and only if all the following conditions are met:
/// - `sockfd` refers to a valid, open socket file descriptor.
/// - Access to `errno` is synchronized with other threads that may modify it.
///
#[unsafe(no_mangle)]
#[trace_syscall]
pub unsafe extern "C" fn shutdown(sockfd: c_int, how: c_int) -> c_int {
    // Attempt to convert shutdown mode.
    let how: Shutdown = match Shutdown::try_from(how) {
        Ok(how) => how,
        Err(_error) => {
            ::syslog::error!("shutdown(): invalid shutdown mode (sockfd={sockfd:?}, how={how:?})");
            *__errno_location() = ErrorCode::InvalidArgument.get();
            return -1;
        },
    };

    // Attempt to shutdown the socket and check for errors.
    match socket::syscall::shutdown(sockfd, how) {
        Ok(()) => 0,
        Err(error) => {
            ::syslog::error!("shutdown(): {error:?} (sockfd={sockfd:?}, how={how:?})");
            *__errno_location() = error.code.get();
            -1
        },
    }
}
