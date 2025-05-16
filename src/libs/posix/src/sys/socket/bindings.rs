// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use core::slice;

use crate::{
    errno::__errno_location,
    ffi::{
        c_int,
        c_void,
    },
    netinet::in_::Protocol,
    sys::{
        socket::{
            sockaddr,
            socklen_t,
            AddressFamily,
            Shutdown,
            SocketAddr,
            SocketType,
        },
        types::{
            size_t,
            ssize_t,
        },
    },
};
use ::nvx::sys::error::ErrorCode;

//==================================================================================================
// Standalone Functions
//==================================================================================================

#[allow(clippy::missing_safety_doc)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn accept(
    sockfd: c_int,
    sockaddr: *mut sockaddr,
    len: *mut socklen_t,
) -> c_int {
    ::syslog::trace!("accept(): sockfd={:?}, sockaddr={:?}, len={:?}", sockfd, sockaddr, len);

    let mut sockaddr_: SocketAddr = SocketAddr::V4(Default::default());

    match crate::sys::socket::accept(sockfd, Some(&mut sockaddr_)) {
        Ok(sockfd) => {
            // Store socket address, if requested.
            match sockaddr_.try_into() {
                // Succeeded to convert socket address.
                Ok((sockaddr_, len_)) => {
                    if !sockaddr.is_null() {
                        *sockaddr = sockaddr_;
                    }

                    if !len.is_null() {
                        *len = len_;
                    }
                },
                // Failed to convert socket address.
                Err(error) => {
                    // Warn and continue, as the socket descriptor was successfully created.
                    ::syslog::warn!(
                        "accept(): failed to convert socket address (error={:?})",
                        error
                    );
                },
            };

            sockfd
        },
        Err(error) => {
            ::syslog::error!("accept(): failed to accept connection (error={:?})", error);
            *__errno_location() = error.code.get();
            -1
        },
    }
}

#[allow(clippy::missing_safety_doc)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn bind(sockfd: c_int, sockaddr: *const sockaddr, len: socklen_t) -> c_int {
    ::syslog::trace!("bind(): sockfd={:?}, sockaddr={:?}, len={:?}", sockfd, sockaddr, len);

    // Check if sock address is valid.
    if sockaddr.is_null() {
        ::syslog::error!("bind(): invalid socket address");
        *__errno_location() = ErrorCode::InvalidArgument.get();
        return -1;
    }

    ::syslog::trace!("bind(): sockaddr={:?}", unsafe { &*sockaddr });

    // Attempt to convert socket address.
    let sockaddr: SocketAddr = match SocketAddr::try_from(unsafe { &*sockaddr }) {
        Ok(sockaddr) => sockaddr,
        Err(e) => {
            ::syslog::error!("bind(): failed to convert socket address {:?}", e);
            *__errno_location() = e.code.get();
            return -1;
        },
    };

    match crate::sys::socket::bind(sockfd, &sockaddr) {
        Ok(_) => 0,
        Err(e) => {
            *__errno_location() = e.code.get();
            -1
        },
    }
}

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
#[unsafe(no_mangle)]
pub unsafe extern "C" fn connect(
    sockfd: c_int,
    sockaddr: *const sockaddr,
    len: socklen_t,
) -> c_int {
    match crate::sys::socket::connect(sockfd, unsafe { &*sockaddr }, len) {
        Ok(sockfd) => sockfd,
        Err(e) => {
            *__errno_location() = e.code.get();
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
#[unsafe(no_mangle)]
pub unsafe extern "C" fn getpeername(
    sockfd: c_int,
    sockaddr: *mut sockaddr,
    len: *mut socklen_t,
) -> c_int {
    // Check if the address is valid.
    if sockaddr.is_null() {
        *__errno_location() = ErrorCode::InvalidArgument.get();
        return -1;
    }

    // Check if the length is valid.
    if len.is_null() {
        *__errno_location() = ErrorCode::InvalidArgument.get();
        return -1;
    }

    let mut sockaddr_: SocketAddr = SocketAddr::V4(Default::default());

    match crate::sys::socket::getpeername(sockfd, &mut sockaddr_) {
        Ok(_) => {
            match sockaddr_.try_into() {
                Ok((sockaddr_, len_)) => {
                    *sockaddr = sockaddr_;
                    *len = len_;
                },
                Err(error) => {
                    ::syslog::error!(
                        "getpeername(): failed to convert socket address (error={:?})",
                        error
                    );
                    *__errno_location() = error.code.get();
                    return -1;
                },
            };
            0
        },
        Err(e) => {
            *__errno_location() = e.code.get();
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
#[unsafe(no_mangle)]
pub unsafe extern "C" fn getsockname(
    sockfd: c_int,
    sockaddr: *mut sockaddr,
    len: *mut socklen_t,
) -> c_int {
    // Check if the address is valid.
    if sockaddr.is_null() {
        *__errno_location() = ErrorCode::InvalidArgument.get();
        return -1;
    }

    // Check if the length is valid.
    if len.is_null() {
        *__errno_location() = ErrorCode::InvalidArgument.get();
        return -1;
    }

    let mut sockaddr_: SocketAddr = SocketAddr::V4(Default::default());

    match crate::sys::socket::getsockname(sockfd, &mut sockaddr_) {
        Ok(_) => {
            let (sockaddr_, len_): (sockaddr, socklen_t) = match sockaddr_.try_into() {
                Ok((sockaddr_, len_)) => (sockaddr_, len_),
                Err(e) => {
                    *__errno_location() = e.code.get();
                    return -1;
                },
            };
            unsafe { *sockaddr = sockaddr_ };
            unsafe { *len = len_ };
            0
        },
        Err(e) => {
            *__errno_location() = e.code.get();
            -1
        },
    }
}

#[allow(clippy::missing_safety_doc)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn listen(sockfd: c_int, backlog: c_int) -> c_int {
    ::syslog::trace!("listen(): sockfd={:?}, backlog={:?}", sockfd, backlog);
    match crate::sys::socket::listen(sockfd, backlog) {
        Ok(_) => 0,
        Err(e) => {
            ::syslog::error!("listen(): failed to listen on socket {:?}", e);
            *__errno_location() = e.code.get();
            -1
        },
    }
}

#[allow(clippy::missing_safety_doc)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn recv(
    sockfd: c_int,
    buf: *mut c_void,
    len: size_t,
    flags: c_int,
) -> ssize_t {
    // Check if `buf` is valid.
    if buf.is_null() {
        ::syslog::error!("recv(): invalid buffer");
        *__errno_location() = ErrorCode::InvalidArgument.get();
        return -1;
    }

    // Check if `len` is valid.
    if len == 0 {
        ::syslog::error!("recv(): invalid buffer length");
        *__errno_location() = ErrorCode::InvalidArgument.get();
        return -1;
    }

    // Check if `flags` is valid.
    if flags != 0 {
        ::syslog::error!("recv(): unsupported flags (flags={:?})", flags);
        *__errno_location() = ErrorCode::InvalidArgument.get();
        return -1;
    }

    // Attempt to convert buffer.
    let buf: &mut [u8] = unsafe { slice::from_raw_parts_mut(buf as *mut u8, len as usize) };

    match crate::sys::socket::recv(sockfd, buf, flags) {
        Ok(bytes_received) => bytes_received as ssize_t,
        Err(e) => {
            ::syslog::error!("recv(): failed to receive data through socket {:?}", e);
            *__errno_location() = e.code.get();
            -1
        },
    }
}

#[allow(clippy::missing_safety_doc)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn send(
    sockfd: c_int,
    buf: *const c_void,
    len: size_t,
    flags: c_int,
) -> ssize_t {
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

    // Attempt to convert buffer.
    let buf: &[u8] = unsafe { slice::from_raw_parts(buf as *const u8, len as usize) };

    match crate::sys::socket::send(sockfd, buf, flags) {
        Ok(bytes_sent) => bytes_sent as ssize_t,
        Err(e) => {
            ::syslog::error!("send(): failed to send data through socket {:?}", e);
            *__errno_location() = e.code.get();
            -1
        },
    }
}

#[allow(clippy::missing_safety_doc)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn socket(domain: c_int, typ: c_int, protocol: c_int) -> c_int {
    ::syslog::trace!("socket(): domain={:?}, type={:?}, protocol={:?}", domain, typ, protocol);
    // Attempt to convert socket address family.
    let domain: AddressFamily = match AddressFamily::try_from(domain) {
        Ok(domain) => domain,
        Err(_error) => {
            ::syslog::error!("socket(): invalid socket address family (domain={:?})", domain);
            *__errno_location() = ErrorCode::InvalidArgument.get();
            return -1;
        },
    };

    // Attempt to convert socket type.
    let typ: SocketType = match SocketType::try_from(typ) {
        Ok(typ) => typ,
        Err(_error) => {
            ::syslog::error!("socket(): invalid socket type (type={:?})", typ);
            *__errno_location() = ErrorCode::InvalidArgument.get();
            return -1;
        },
    };

    // Attempt to convert socket protocol.
    let protocol: Protocol = match Protocol::try_from(protocol) {
        Ok(protocol) => protocol,
        Err(_error) => {
            ::syslog::error!("socket(): invalid socket protocol (protocol={:?})", protocol);
            *__errno_location() = ErrorCode::InvalidArgument.get();
            return -1;
        },
    };

    // Create socket.
    match crate::sys::socket::socket(domain, typ, protocol) {
        Ok(sockfd) => sockfd,
        Err(error) => {
            ::syslog::error!("socket(): failed to create socket (error={:?})", error);
            *__errno_location() = error.code.get();
            -1
        },
    }
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

    match crate::sys::socket::shutdown(sockfd, how) {
        Ok(_) => 0,
        Err(e) => {
            ::syslog::error!("shutdown(): failed to shutdown socket {:?}", e);
            unsafe { *__errno_location() = e.code.get() };
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
#[unsafe(no_mangle)]
pub unsafe extern "C" fn socketpair(
    domain: c_int,
    typ: c_int,
    protocol: c_int,
    socket_fds: *mut c_int,
) -> c_int {
    // Check if socket pair is valid.
    if socket_fds.is_null() {
        *__errno_location() = ErrorCode::InvalidArgument.get();
        return -1;
    }

    // Reconstruct array.
    let socket_fds: &mut [c_int] = slice::from_raw_parts_mut(socket_fds, 2);

    // Attempt to convert socket address family.
    let domain: AddressFamily = match domain.try_into() {
        Ok(domain) => domain,
        Err(_error) => {
            *__errno_location() = ErrorCode::InvalidArgument.get();
            return -1;
        },
    };

    // Attempt to convert socket type.
    let typ: SocketType = match typ.try_into() {
        Ok(typ) => typ,
        Err(_error) => {
            *__errno_location() = ErrorCode::InvalidArgument.get();
            return -1;
        },
    };

    // Attempt to convert socket protocol.
    let protocol: Protocol = match protocol.try_into() {
        Ok(protocol) => protocol,
        Err(_error) => {
            *__errno_location() = ErrorCode::InvalidArgument.get();
            return -1;
        },
    };

    match crate::sys::socket::socketpair(domain, typ, protocol, socket_fds) {
        Ok(_) => 0,
        Err(e) => {
            *__errno_location() = e.code.get();
            -1
        },
    }
}
