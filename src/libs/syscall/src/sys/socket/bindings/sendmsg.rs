// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::{
    errno::__errno_location,
    sys::socket::{
        self,
        SocketAddr,
    },
};
use ::alloc::vec::Vec;
use ::core::{
    mem,
    slice,
};
use ::sys::error::ErrorCode;
use ::sysapi::{
    ffi::c_int,
    sys_socket::sockaddr,
    sys_types::{
        c_ssize_t,
        msghdr,
    },
    sys_uio::iovec,
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
pub unsafe extern "C" fn sendmsg(sockfd: c_int, msg: *const msghdr, flags: c_int) -> c_ssize_t {
    {
        // Check if `msg` is valid.
        if msg.is_null() {
            ::syslog::warn!("sendmsg(): invalid message header (sockfd={sockfd:?}, msg={msg:?})");
            *__errno_location() = ErrorCode::InvalidArgument.get();
            return -1;
        }
        let msg: &msghdr = unsafe { &*msg };

        // Check if `flags` is valid.
        if flags != 0 {
            ::syslog::warn!("sendmsg(): unsupported flags (sockfd={sockfd:?}, flags={flags:?})");
            *__errno_location() = ErrorCode::InvalidArgument.get();
            return -1;
        }

        // Convert the number of scatter-gather buffers to a count.
        let msg_iovlen = msg.msg_iovlen;
        let iovlen: usize = match usize::try_from(msg_iovlen) {
            Ok(iovlen) => iovlen,
            Err(_error) => {
                ::syslog::warn!(
                    "sendmsg(): invalid iovec count (sockfd={sockfd:?}, msg_iovlen={msg_iovlen:?})"
                );
                *__errno_location() = ErrorCode::InvalidArgument.get();
                return -1;
            },
        };

        // The number of scatter-gather buffers must not exceed the system limit.
        if iovlen > ::sysapi::limits::IOV_MAX {
            ::syslog::warn!(
                "sendmsg(): iovec count exceeds IOV_MAX (sockfd={sockfd:?}, \
                 msg_iovlen={msg_iovlen:?})"
            );
            *__errno_location() = ErrorCode::InvalidArgument.get();
            return -1;
        }

        // The scatter-gather array must be valid when it is non-empty.
        if iovlen > 0 && msg.msg_iov.is_null() {
            ::syslog::warn!("sendmsg(): invalid iovec array (sockfd={sockfd:?})");
            *__errno_location() = ErrorCode::InvalidArgument.get();
            return -1;
        }

        // Ancillary data is not supported by this implementation.
        let msg_control = msg.msg_control;
        let msg_controllen = msg.msg_controllen;
        if !msg_control.is_null() || msg_controllen != 0 {
            ::syslog::warn!(
                "sendmsg(): ancillary data is not supported (sockfd={sockfd:?}, msg_control={:?}, \
                 msg_controllen={:?})",
                msg_control,
                msg_controllen
            );
            *__errno_location() = ErrorCode::OperationNotSupportedOnSocket.get();
            return -1;
        }

        // Gather the scatter-gather buffers into a single contiguous datagram.
        let mut data: Vec<u8> = Vec::new();
        for index in 0..iovlen {
            let entry: &iovec = unsafe { &*msg.msg_iov.add(index) };
            if entry.iov_len == 0 {
                continue;
            }
            if entry.iov_base.is_null() {
                ::syslog::warn!("sendmsg(): invalid iovec buffer (sockfd={sockfd:?})");
                *__errno_location() = ErrorCode::InvalidArgument.get();
                return -1;
            }
            let chunk: &[u8] =
                unsafe { slice::from_raw_parts(entry.iov_base.cast_const(), entry.iov_len) };
            data.extend_from_slice(chunk);
        }

        // When no destination address is supplied, `sendmsg()` behaves like `send()`.
        let result: Result<usize, ::sys::error::Error> = if msg.msg_name.is_null() {
            socket::syscall::send(sockfd, &data, flags)
        } else {
            // Validate that `msg_namelen` covers a full `sockaddr` before dereferencing the pointer.
            let msg_namelen = msg.msg_namelen;
            if (msg_namelen as usize) < mem::size_of::<sockaddr>() {
                ::syslog::warn!(
                    "sendmsg(): invalid socket address length (sockfd={sockfd:?}, \
                     msg_namelen={msg_namelen:?})"
                );
                *__errno_location() = ErrorCode::InvalidArgument.get();
                return -1;
            }

            // Attempt to convert socket address.
            let sockaddr: &sockaddr = unsafe { &*msg.msg_name.cast::<sockaddr>() };
            let sockaddr: SocketAddr = match TryFrom::<&sockaddr>::try_from(sockaddr) {
                Ok(sockaddr) => sockaddr,
                Err(error) => {
                    ::syslog::warn!("sendmsg(): {error:?} (sockfd={sockfd:?})");
                    *__errno_location() = error.code.get();
                    return -1;
                },
            };

            socket::syscall::sendto(sockfd, &data, flags, &sockaddr)
        };

        // Check for errors.
        match result {
            Ok(bytes_sent) => match bytes_sent.try_into() {
                Ok(bytes_sent) => bytes_sent,
                Err(_error) => {
                    ::syslog::warn!(
                        "sendmsg(): failed to convert bytes sent (sockfd={sockfd:?}, \
                         flags={flags:?})"
                    );
                    *__errno_location() = ErrorCode::ValueOutOfRange.get();
                    -1
                },
            },
            Err(error) => {
                ::syslog::warn!("sendmsg(): {error:?} (sockfd={sockfd:?}, flags={flags:?})");
                *__errno_location() = error.code.get();
                -1
            },
        }
    }
}
