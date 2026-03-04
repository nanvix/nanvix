// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::errno::__errno_location;
use ::sys::error::ErrorCode;
use ::sysapi::{
    ffi::c_int,
    sys_types::{
        c_ssize_t,
        msghdr,
    },
};
use ::syslog::trace_syscall;

//==================================================================================================
// Standalone Functions
//==================================================================================================

///
/// # Description
///
/// Sends a message on a socket using a message header structure. The `sendmsg()` function sends
/// data on a socket and provides the most general interface for sending messages. Unlike `send()`
/// and `sendto()`, `sendmsg()` can send multiple buffers in a single call through scatter-gather
/// I/O, include ancillary data (control messages), and specify detailed destination information.
/// This function is particularly useful for advanced socket programming where additional message
/// metadata needs to be transmitted.
///
/// # Parameters
///
/// - `sockfd`: File descriptor of the socket on which to send the message.
/// - `msg`: Pointer to a `msghdr` structure that describes the message buffers, destination
///   address, and ancillary data. The structure contains scatter-gather buffers, address
///   information, and control message space.
/// - `flags`: Flags that modify the behavior of the send operation (e.g., MSG_DONTWAIT, MSG_NOSIGNAL).
///
/// # Returns
///
/// The `sendmsg()` function returns the number of bytes sent on success. On error, it returns `-1`
/// and sets `errno` to indicate the error. The number of bytes sent may be less than the total
/// requested if the socket send buffer becomes full.
///
/// # Safety
///
/// This function is unsafe because it may dereference raw pointers and modify global state.
///
/// It is safe to call this function if and only if all the following conditions are met:
/// - `msg` points to a valid `msghdr` structure.
/// - All buffers referenced by the `msghdr` structure are valid and remain so for the duration of
///   the call.
/// - The `msg_name` field (if not NULL) points to a valid address buffer of at least `msg_namelen`
///   bytes.
/// - The `msg_iov` field points to a valid array of `msg_iovlen` `iovec` structures.
/// - Each `iovec` structure references a valid buffer of the specified length.
/// - The `msg_control` field (if not NULL) points to a valid control message buffer of at least
///   `msg_controllen` bytes.
/// - Access to `errno` is synchronized with other threads that may modify it.
///
#[unsafe(no_mangle)]
#[trace_syscall]
#[allow(unreachable_code)]
pub unsafe extern "C" fn sendmsg(sockfd: c_int, msg: *const msghdr, flags: c_int) -> c_ssize_t {
    #[cfg(feature = "standalone")]
    {
        let _ = (sockfd, msg, flags);
        *__errno_location() = ::sys::error::ErrorCode::OperationNotSupported.get();
        return -1;
    }

    // TODO: https://github.com/nanvix/nanvix/issues/599.
    ::syslog::debug!("sendmsg(): not implemented");
    unsafe {
        *__errno_location() = ErrorCode::InvalidSysCall.get();
    }
    -1
}
