// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use core::slice;

use crate::{
    errno::errno,
    ffi::c_int,
    sys::socket::{
        sockaddr,
        socklen_t,
        AddressFamily,
        Protocol,
        SocketAddr,
        SocketType,
    },
};
use ::nvx::sys::error::ErrorCode;

//==================================================================================================
// Standalone Functions
//==================================================================================================

///
/// # Description
///
/// Connects a socket.
///
/// # Parameters
///
/// - `sockfd`: File descriptor of the socket.
/// - `sockaddr`: Address of the socket.
/// - `len`: Size of the address.
///
/// # Returns
///
/// The `connect()` function returns the file descriptor of the socket on success. On error, it
/// returns `-1` and sets `errno` to indicate the error.
///
/// # Safety
///
/// This function is unsafe because it may deference raw pointers.
///
#[no_mangle]
pub unsafe extern "C" fn connect(
    sockfd: c_int,
    sockaddr: *const sockaddr,
    len: socklen_t,
) -> c_int {
    match crate::sys::socket::connect(sockfd, unsafe { &*sockaddr }, len) {
        Ok(sockfd) => sockfd,
        Err(e) => {
            unsafe { errno = e.code.into_errno() }
            -1
        },
    }
}

///
/// # Description
///
/// Gets the name of the peer socket.
///
/// # Parameters
///
/// - `sockfd`: File descriptor of the socket.
/// - `sockaddr`: Location to store the address of the peer socket.
/// - `len`: Location to store the size of the address.
///
/// # Returns
///
/// Upon successful completion, the `getpeername()` function returns `0`. Otherwise, on failure, it
/// returns `-1` and sets `errno` to indicate the error.
///
/// # Safety
///
/// This function is unsafe because it may deference raw pointers.
///
#[no_mangle]
pub unsafe extern "C" fn getpeername(
    sockfd: c_int,
    sockaddr: *mut sockaddr,
    len: *mut socklen_t,
) -> c_int {
    // Check if the address is valid.
    if sockaddr.is_null() {
        unsafe { errno = ErrorCode::InvalidArgument.into_errno() };
        return -1;
    }

    // Check if the length is valid.
    if len.is_null() {
        unsafe { errno = ErrorCode::InvalidArgument.into_errno() };
        return -1;
    }

    let mut sockaddr_: SocketAddr = SocketAddr::V4(Default::default());

    match crate::sys::socket::getpeername(sockfd, &mut sockaddr_) {
        Ok(_) => {
            let (sockaddr_, len_): (sockaddr, socklen_t) = match sockaddr_.try_into() {
                Ok((sockaddr_, len_)) => (sockaddr_, len_),
                Err(e) => {
                    unsafe { errno = e.code.into_errno() };
                    return -1;
                },
            };
            unsafe { *sockaddr = sockaddr_ };
            unsafe { *len = len_ };
            0
        },
        Err(e) => {
            unsafe { errno = e.code.into_errno() }
            -1
        },
    }
}

///
/// # Description
///
/// Gets the name of the socket.
///
/// # Parameters
///
/// - `sockfd`: File descriptor of the socket.
/// - `sockaddr`: Location to store the address of the socket.
/// - `len`: Location to store the size of the address.
///
/// # Returns
///
/// Upon successful completion, the `getsockname()` function returns `0`. Otherwise, on failure, it
/// returns `-1` and sets `errno` to indicate the error.
///
/// # Safety
///
/// This function is unsafe because it may deference raw pointers.
///
#[no_mangle]
pub unsafe extern "C" fn getsockname(
    sockfd: c_int,
    sockaddr: *mut sockaddr,
    len: *mut socklen_t,
) -> c_int {
    // Check if the address is valid.
    if sockaddr.is_null() {
        unsafe { errno = ErrorCode::InvalidArgument.into_errno() };
        return -1;
    }

    // Check if the length is valid.
    if len.is_null() {
        unsafe { errno = ErrorCode::InvalidArgument.into_errno() };
        return -1;
    }

    let mut sockaddr_: SocketAddr = SocketAddr::V4(Default::default());

    match crate::sys::socket::getsockname(sockfd, &mut sockaddr_) {
        Ok(_) => {
            let (sockaddr_, len_): (sockaddr, socklen_t) = match sockaddr_.try_into() {
                Ok((sockaddr_, len_)) => (sockaddr_, len_),
                Err(e) => {
                    unsafe { errno = e.code.into_errno() };
                    return -1;
                },
            };
            unsafe { *sockaddr = sockaddr_ };
            unsafe { *len = len_ };
            0
        },
        Err(e) => {
            unsafe { errno = e.code.into_errno() }
            -1
        },
    }
}

///
/// # Description
///
/// Creates a pair of connected sockets.
///
/// # Parameters
///
/// - `domain`: Communication domain.
/// - `typ`: Socket type.
/// - `protocol`: Protocol.
/// - `socket_fds`: Array where the file descriptors of the sockets will be stored.
///
/// # Returns
///
/// The `socketpair()` function returns `0` on success. On error, it returns `-1` and sets `errno` to
/// indicate the error.
///
/// # Safety
///
/// This function is unsafe because it may deference raw pointers.
///
#[no_mangle]
pub unsafe extern "C" fn socketpair(
    domain: c_int,
    typ: c_int,
    protocol: c_int,
    socket_fds: *mut c_int,
) -> c_int {
    // Check if socket pair is valid.
    if socket_fds.is_null() {
        unsafe { errno = ErrorCode::InvalidArgument.into_errno() };
        return -1;
    }

    // Reconstruct array.
    let socket_fds: &mut [c_int] = slice::from_raw_parts_mut(socket_fds, 2);

    // Attempt to convert socket address family.
    let domain: AddressFamily = match domain.try_into() {
        Ok(domain) => domain,
        Err(_error) => {
            unsafe { errno = ErrorCode::InvalidArgument.into_errno() };
            return -1;
        },
    };

    // Attempt to convert socket type.
    let typ: SocketType = match typ.try_into() {
        Ok(typ) => typ,
        Err(_error) => {
            unsafe { errno = ErrorCode::InvalidArgument.into_errno() };
            return -1;
        },
    };

    // Attempt to convert socket protocol.
    let protocol: Protocol = match protocol.try_into() {
        Ok(protocol) => protocol,
        Err(_error) => {
            unsafe { errno = ErrorCode::InvalidArgument.into_errno() };
            return -1;
        },
    };

    match crate::sys::socket::socketpair(domain, typ, protocol, socket_fds) {
        Ok(_) => 0,
        Err(e) => {
            unsafe { errno = e.code.into_errno() }
            -1
        },
    }
}
