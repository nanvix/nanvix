// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::errno::__errno_location;
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
    },
};

//==================================================================================================
// Standalone Functions
//==================================================================================================

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
