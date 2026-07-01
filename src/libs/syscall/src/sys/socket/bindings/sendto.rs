// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::errno::__errno_location;
#[cfg(feature = "standalone")]
use crate::sys::{
    socket,
    socket::SocketAddr,
};
#[cfg(feature = "standalone")]
use ::core::slice;
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
        // Check if `buf` is valid.
        if buf.is_null() {
            ::syslog::warn!(
                "sendto(): invalid buffer (sockfd={sockfd:?}, buf={buf:?}, len={len:?}, \
                 flags={flags:?})"
            );
            *__errno_location() = ErrorCode::InvalidArgument.get();
            return -1;
        }

        // NOTE: `len` is not validated here. POSIX permits zero-length datagrams, but the
        // standalone syscall path (`send`/`sendto`) currently rejects empty buffers with EINVAL.

        // Check if `flags` is valid.
        if flags != 0 {
            ::syslog::warn!(
                "sendto(): unsupported flags (sockfd={sockfd:?}, buf={buf:?}, len={len:?}, \
                 flags={flags:?})"
            );
            *__errno_location() = ErrorCode::InvalidArgument.get();
            return -1;
        }

        // Attempt to convert `len` to `usize`.
        let len: usize = match len.try_into() {
            Ok(len) => len,
            Err(_error) => {
                ::syslog::warn!(
                    "sendto(): failed to convert length (sockfd={sockfd:?}, buf={buf:?}, \
                     len={len:?}, flags={flags:?})"
                );
                *__errno_location() = ErrorCode::InvalidArgument.get();
                return -1;
            },
        };

        // Attempt to convert buffer.
        let buf: &[u8] = unsafe { slice::from_raw_parts(buf as *const u8, len) };

        // When no destination address is supplied, `sendto()` behaves like `send()`.
        let result: Result<usize, ::sys::error::Error> = if sockaddr.is_null() {
            socket::syscall::send(sockfd, buf, flags)
        } else {
            // Validate that `addrlen` covers a full `sockaddr` before dereferencing the pointer.
            // A non-NULL pointer with a too-small `addrlen` would otherwise cause an
            // out-of-bounds read when the address is dereferenced below.
            if (addrlen as usize) < ::core::mem::size_of::<sockaddr>() {
                ::syslog::warn!(
                    "sendto(): invalid socket address length (sockfd={sockfd:?}, \
                     sockaddr={sockaddr:?}, addrlen={addrlen:?})"
                );
                *__errno_location() = ErrorCode::InvalidArgument.get();
                return -1;
            }

            // Attempt to convert socket address.
            let sockaddr: SocketAddr = match TryFrom::<&sockaddr>::try_from(&*sockaddr) {
                Ok(sockaddr) => sockaddr,
                Err(error) => {
                    ::syslog::warn!(
                        "sendto(): {error:?} (sockfd={sockfd:?}, sockaddr={sockaddr:?}, \
                         addrlen={addrlen:?})"
                    );
                    *__errno_location() = error.code.get();
                    return -1;
                },
            };

            socket::syscall::sendto(sockfd, buf, flags, &sockaddr)
        };

        // Check for errors.
        match result {
            Ok(bytes_sent) => match bytes_sent.try_into() {
                Ok(bytes_sent) => bytes_sent,
                Err(_error) => {
                    ::syslog::warn!(
                        "sendto(): failed to convert bytes sent (sockfd={sockfd:?}, buf={buf:?}, \
                         len={len:?}, flags={flags:?})"
                    );
                    *__errno_location() = ErrorCode::ValueOutOfRange.get();
                    -1
                },
            },
            Err(error) => {
                ::syslog::warn!(
                    "sendto(): {error:?} (sockfd={sockfd:?}, len={len:?}, flags={flags:?})"
                );
                *__errno_location() = error.code.get();
                -1
            },
        }
    }

    // TODO: https://github.com/nanvix/nanvix/issues/589
    #[cfg(not(feature = "standalone"))]
    {
        let _ = (sockfd, buf, len, flags, sockaddr, addrlen);
        ::syslog::debug!("sendto(): not implemented");
        unsafe {
            *__errno_location() = ErrorCode::InvalidSysCall.get();
        }
        -1
    }
}
