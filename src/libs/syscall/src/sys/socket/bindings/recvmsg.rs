// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::errno::__errno_location;
#[cfg(feature = "standalone")]
use crate::sys::socket::{
    self,
    SocketAddr,
};
#[cfg(feature = "standalone")]
use ::alloc::vec::Vec;
#[cfg(feature = "standalone")]
use ::core::{
    cmp,
    mem,
    slice,
};
use ::sys::error::ErrorCode;
use ::sysapi::{
    ffi::c_int,
    sys_types::{
        c_ssize_t,
        msghdr,
    },
};
#[cfg(feature = "standalone")]
use ::sysapi::{
    sys_socket::{
        sockaddr,
        socklen_t,
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
/// Receives a message from a socket using a message header structure. The `recvmsg()` function
/// receives data from a socket and provides the most general interface for receiving messages.
/// Unlike `recv()` and `recvfrom()`, `recvmsg()` can receive multiple buffers in a single call
/// through scatter-gather I/O, access ancillary data (control messages), and retrieve detailed
/// information about the received message. This function is particularly useful for advanced socket
/// programming where additional message metadata is required.
///
/// # Parameters
///
/// - `sockfd`: File descriptor of the socket from which to receive the message.
/// - `msg`: Pointer to a `msghdr` structure that describes the message buffers, source address, and
///   ancillary data. The structure contains scatter-gather buffers, address information, and control
///   message space.
/// - `flags`: Flags that modify the behavior of the receive operation (e.g., MSG_PEEK,
///   MSG_WAITALL).
///
/// # Returns
///
/// The `recvmsg()` function returns the number of bytes received on success. On error, it returns
/// `-1` and sets `errno` to indicate the error. A return value of `0` indicates that the peer has
/// performed an orderly shutdown.
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
pub unsafe extern "C" fn recvmsg(sockfd: c_int, msg: *mut msghdr, flags: c_int) -> c_ssize_t {
    #[cfg(feature = "standalone")]
    {
        // Check if `msg` is valid.
        if msg.is_null() {
            ::syslog::warn!("recvmsg(): invalid message header (sockfd={sockfd:?}, msg={msg:?})");
            *__errno_location() = ErrorCode::InvalidArgument.get();
            return -1;
        }
        let msg: &mut msghdr = unsafe { &mut *msg };

        // Check if `flags` is valid.
        if flags != 0 {
            ::syslog::warn!("recvmsg(): unsupported flags (sockfd={sockfd:?}, flags={flags:?})");
            *__errno_location() = ErrorCode::InvalidArgument.get();
            return -1;
        }

        // Convert the number of scatter-gather buffers to a count.
        let msg_iovlen = msg.msg_iovlen;
        let iovlen: usize = match usize::try_from(msg_iovlen) {
            Ok(iovlen) => iovlen,
            Err(_error) => {
                ::syslog::warn!(
                    "recvmsg(): invalid iovec count (sockfd={sockfd:?}, msg_iovlen={msg_iovlen:?})"
                );
                *__errno_location() = ErrorCode::InvalidArgument.get();
                return -1;
            },
        };

        // The number of scatter-gather buffers must not exceed the system limit.
        if iovlen > ::sysapi::limits::IOV_MAX {
            ::syslog::warn!(
                "recvmsg(): iovec count exceeds IOV_MAX (sockfd={sockfd:?}, \
                 msg_iovlen={msg_iovlen:?})"
            );
            *__errno_location() = ErrorCode::InvalidArgument.get();
            return -1;
        }

        // The scatter-gather array must be valid when it is non-empty.
        if iovlen > 0 && msg.msg_iov.is_null() {
            ::syslog::warn!("recvmsg(): invalid iovec array (sockfd={sockfd:?})");
            *__errno_location() = ErrorCode::InvalidArgument.get();
            return -1;
        }

        // When a source address is requested, the buffer must fit a full `sockaddr`.
        let want_source: bool = !msg.msg_name.is_null();
        let msg_namelen = msg.msg_namelen;
        if want_source && (msg_namelen as usize) < mem::size_of::<sockaddr>() {
            ::syslog::warn!(
                "recvmsg(): invalid socket address length (sockfd={sockfd:?}, \
                 msg_namelen={msg_namelen:?})"
            );
            *__errno_location() = ErrorCode::InvalidArgument.get();
            return -1;
        }

        // Compute the total capacity of the scatter-gather buffers.
        let mut total: usize = 0;
        for index in 0..iovlen {
            let entry: &iovec = unsafe { &*msg.msg_iov.add(index) };
            if entry.iov_len == 0 {
                continue;
            }
            if entry.iov_base.is_null() {
                ::syslog::warn!("recvmsg(): invalid iovec buffer (sockfd={sockfd:?})");
                *__errno_location() = ErrorCode::InvalidArgument.get();
                return -1;
            }
            total = total.saturating_add(entry.iov_len);
        }

        // The receive buffers must have room for at least one byte.
        if total == 0 {
            ::syslog::warn!("recvmsg(): invalid buffer length (sockfd={sockfd:?})");
            *__errno_location() = ErrorCode::InvalidArgument.get();
            return -1;
        }

        // Receive the datagram into a single contiguous staging buffer.
        let mut buffer: Vec<u8> = ::alloc::vec![0u8; total];
        let (received, source): (usize, Option<SocketAddr>) = if want_source {
            match socket::syscall::recvfrom(sockfd, &mut buffer, flags) {
                Ok((received, source)) => (received, Some(source)),
                Err(error) => {
                    ::syslog::warn!("recvmsg(): {error:?} (sockfd={sockfd:?}, flags={flags:?})");
                    *__errno_location() = error.code.get();
                    return -1;
                },
            }
        } else {
            match socket::syscall::recv(sockfd, &mut buffer, flags) {
                Ok(received) => (received, None),
                Err(error) => {
                    ::syslog::warn!("recvmsg(): {error:?} (sockfd={sockfd:?}, flags={flags:?})");
                    *__errno_location() = error.code.get();
                    return -1;
                },
            }
        };

        // Scatter the received bytes across the caller's buffers.
        let mut offset: usize = 0;
        for index in 0..iovlen {
            if offset >= received {
                break;
            }
            let entry: &iovec = unsafe { &*msg.msg_iov.add(index) };
            if entry.iov_len == 0 {
                continue;
            }
            let chunk_len: usize = cmp::min(entry.iov_len, received - offset);
            let chunk: &mut [u8] = unsafe { slice::from_raw_parts_mut(entry.iov_base, chunk_len) };
            chunk.copy_from_slice(&buffer[offset..offset + chunk_len]);
            offset += chunk_len;
        }

        // Store the source address if the caller requested it.
        if let Some(source) = source {
            let (source_addr, source_len): (sockaddr, socklen_t) = source.into();
            unsafe { *msg.msg_name.cast::<sockaddr>() = source_addr };
            msg.msg_namelen = source_len;
        }

        // No ancillary data is delivered by this implementation.
        msg.msg_controllen = 0;
        msg.msg_flags = 0;

        // Report the number of bytes received.
        match received.try_into() {
            Ok(received) => received,
            Err(_error) => {
                ::syslog::warn!(
                    "recvmsg(): failed to convert bytes received (sockfd={sockfd:?}, \
                     flags={flags:?})"
                );
                *__errno_location() = ErrorCode::ValueOutOfRange.get();
                -1
            },
        }
    }

    // TODO: https://github.com/nanvix/nanvix/issues/600
    #[cfg(not(feature = "standalone"))]
    {
        let _ = (sockfd, msg, flags);
        ::syslog::debug!("recvmsg(): not implemented");
        unsafe {
            *__errno_location() = ErrorCode::InvalidSysCall.get();
        }
        -1
    }
}
