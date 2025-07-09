// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::errno::__errno_location;
use ::core::slice;
use ::sys::error::ErrorCode;
use ::sysapi::ffi::{
    c_int,
    c_void,
};
use ::syscall::sys::{
    socket,
    socket::Shutdown,
};
use sysapi::{
    sys_socket::{
        sockaddr,
        socklen_t,
    },
    sys_types::{
        c_size_t,
        c_ssize_t,
        msghdr,
    },
};

//==================================================================================================
// Standalone Functions
//==================================================================================================

///
/// # Description
///
/// Receives a message from a socket.
///
/// # Parameters
///
/// - `sockfd`: File descriptor of the socket.
/// - `msg`: Pointer to the msghdr structure describing the message buffer.
/// - `flags`: Flags for receiving the message.
///
/// # Returns
///
/// The `recvmsg()` function returns the number of bytes received on success. On error, it returns `-1`
/// and sets `errno` to indicate the error.
///
/// # Safety
///
/// This function is unsafe because it may dereference raw pointers.
///
/// It is safe to call this function if the following conditions are met:
/// - `msg` points to a valid `msghdr` structure.
///
#[unsafe(no_mangle)]
pub unsafe extern "C" fn recvmsg(sockfd: c_int, msg: *mut msghdr, flags: c_int) -> c_ssize_t {
    ::syslog::trace!("recvmsg(): sockfd={sockfd:?}, msg={msg:?}, flags={flags:?}");
    // TODO: https://github.com/nanvix/nanvix/issues/600
    ::syslog::error!("recvmsg(): not implemented");
    *__errno_location() = ErrorCode::InvalidSysCall.get();
    -1
}

#[allow(clippy::missing_safety_doc)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn send(
    sockfd: c_int,
    buf: *const c_void,
    len: c_size_t,
    flags: c_int,
) -> c_ssize_t {
    // Check if `buf` is valid.
    if buf.is_null() {
        ::syslog::error!("send(): invalid buffer");
        *__errno_location() = ErrorCode::InvalidArgument.get();
        return -1;
    }

    // Check if `len` is valid.
    if len == 0 {
        ::syslog::error!("send(): invalid buffer length");
        *__errno_location() = ErrorCode::InvalidArgument.get();
        return -1;
    }

    // Check if `flags` is valid.
    if flags != 0 {
        ::syslog::error!("send(): unsupported flags (flags={:?})", flags);
        *__errno_location() = ErrorCode::InvalidArgument.get();
        return -1;
    }

    // Attempt to convert `len` to `usize`.
    let len: usize = match len.try_into() {
        Ok(len) => len,
        Err(_error) => {
            ::syslog::error!("send(): failed to convert length to usize");
            *__errno_location() = ErrorCode::InvalidArgument.get();
            return -1;
        },
    };

    // Attempt to convert buffer.
    let buf: &[u8] = unsafe { slice::from_raw_parts(buf as *const u8, len) };

    match socket::syscall::send(sockfd, buf, flags) {
        Ok(bytes_sent) => match bytes_sent.try_into() {
            Ok(bytes_sent) => bytes_sent,
            Err(_error) => {
                ::syslog::error!("send(): failed to convert bytes sent");
                *__errno_location() = ErrorCode::ValueOutOfRange.get();
                -1
            },
        },
        Err(e) => {
            ::syslog::error!("send(): failed to send data through socket {:?}", e);
            *__errno_location() = e.code.get();
            -1
        },
    }
}

///
/// # Description
///
/// Sends a message on a socket.
///
/// # Parameters
///
/// - `sockfd`: File descriptor of the socket.
/// - `msg`: Pointer to the msghdr structure describing the message to send.
/// - `flags`: Flags for sending the message.
///
/// # Returns
///
/// The `sendmsg()` function returns the number of bytes sent on success. On error, it returns `-1`
/// and sets `errno` to indicate the error.
///
/// # Safety
///
/// This function is unsafe because it may dereference raw pointers.
///
/// It is safe to call this function if the following conditions are met:
/// - `msg` points to a valid `msghdr` structure.
///
#[unsafe(no_mangle)]
pub unsafe extern "C" fn sendmsg(sockfd: c_int, msg: *const msghdr, flags: c_int) -> c_ssize_t {
    ::syslog::trace!("sendmsg(): sockfd={sockfd:?}, msg={msg:?}, flags={flags:?}");
    // TODO: https://github.com/nanvix/nanvix/issues/599.
    ::syslog::error!("sendmsg(): not implemented");
    unsafe {
        *__errno_location() = ErrorCode::InvalidSysCall.get();
    }
    -1
}

///
/// # Description
///
/// Sends data to a specific address.
///
/// # Parameters
///
/// - `sockfd`: File descriptor of the socket.
/// - `buf`: Pointer to the buffer containing the data to be sent.
/// - `len`: Length of the data to be sent.
/// - `flags`: Flags for sending data.
/// - `sockaddr`: Pointer to the socket address structure.
/// - `addrlen`: Length of the socket address structure.
///
/// # Returns
///
/// The `sendto()` function returns the number of bytes sent on success. On error, it returns `-1`
/// and sets `errno` to indicate the error.
///
/// # Safety
///
/// This function is unsafe because it may dereference raw pointers.
///
/// It is safe to call this function is the following conditions are met:
/// - `sockaddr` points to a valid socket address structure.
/// - `buf` points to a valid buffer of length `len`.
///
#[unsafe(no_mangle)]
pub unsafe extern "C" fn sendto(
    sockfd: c_int,
    buf: *const c_void,
    len: c_size_t,
    flags: c_int,
    sockaddr: *const sockaddr,
    addrlen: socklen_t,
) -> c_ssize_t {
    ::syslog::trace!(
        "sendto(): sockfd={sockfd:?}, buf={buf:?}, len={len:?}, flags={flags:?}, \
         sockaddr={sockaddr:?}, addrlen={addrlen:?}"
    );
    // TODO: https://github.com/nanvix/nanvix/issues/589
    ::syslog::error!("sendto(): not implemented");
    unsafe {
        *__errno_location() = ErrorCode::InvalidSysCall.get();
    }
    -1
}

#[unsafe(no_mangle)]
pub extern "C" fn shutdown(sockfd: c_int, how: c_int) -> c_int {
    ::syslog::trace!("shutdown(): sockfd={:?}, how={:?}", sockfd, how);

    // Attempt to convert shutdown mode.
    let how: Shutdown = match Shutdown::try_from(how) {
        Ok(how) => how,
        Err(_error) => {
            ::syslog::error!("shutdown(): invalid shutdown mode (how={:?})", how);
            unsafe { *__errno_location() = ErrorCode::InvalidArgument.get() };
            return -1;
        },
    };

    match socket::syscall::shutdown(sockfd, how) {
        Ok(_) => 0,
        Err(e) => {
            ::syslog::error!("shutdown(): failed to shutdown socket {:?}", e);
            unsafe { *__errno_location() = e.code.get() };
            -1
        },
    }
}
