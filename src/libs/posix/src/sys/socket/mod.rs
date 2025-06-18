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
use ::syscall::{
    netinet::in_::Protocol,
    sys::{
        socket,
        socket::{
            AddressFamily,
            Shutdown,
            SocketAddr,
            SocketType,
        },
    },
};
use sysapi::{
    sys_socket::{
        sockaddr,
        socklen_t,
    },
    sys_types::{
        msghdr,
        size_t,
        ssize_t,
    },
};

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
    ::syslog::trace!("accept(): sockfd={sockfd:?}, sockaddr={sockaddr:?}, len={len:?}");

    match socket::syscall::accept(sockfd) {
        Ok((sockfd, sockaddr_)) => {
            // Store socket address, if requested.
            let (sockaddr_, len_) = From::<&SocketAddr>::from(&sockaddr_);
            if !sockaddr.is_null() {
                *sockaddr = sockaddr_;
            }

            if !len.is_null() {
                *len = len_;
            }

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

    match socket::syscall::bind(sockfd, &sockaddr) {
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
    // Check if `sockaddr` is valid.
    if sockaddr.is_null() {
        let reason: &str = "invalid socket address";
        ::syslog::error!("connect(): {reason}");
        *__errno_location() = ErrorCode::InvalidArgument.get();
        return -1;
    }

    // Check if `len` is valid.
    if len == 0 {
        let reason: &str = "invalid socket address length";
        ::syslog::error!("connect(): {reason}");
        *__errno_location() = ErrorCode::InvalidArgument.get();
        return -1;
    }

    let sockaddr: SocketAddr = match TryFrom::<&sockaddr>::try_from(unsafe { &*sockaddr }) {
        Ok(sockaddr) => sockaddr,
        Err(error) => {
            ::syslog::error!("connect(): failed to convert socket address ({error:?})");
            *__errno_location() = error.code.get();
            return -1;
        },
    };

    match socket::syscall::connect(sockfd, &sockaddr) {
        Ok(()) => 0,
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

    match socket::syscall::getpeername(sockfd, &mut sockaddr_) {
        Ok(()) => {
            let (sockaddr_, len_): (sockaddr, socklen_t) = From::<&SocketAddr>::from(&sockaddr_);
            *sockaddr = sockaddr_;
            *len = len_;
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

    match socket::syscall::getsockname(sockfd, &mut sockaddr_) {
        Ok(_) => {
            let (sockaddr_, len_): (sockaddr, socklen_t) = From::<&SocketAddr>::from(&sockaddr_);
            unsafe {
                *sockaddr = sockaddr_;
                *len = len_;
            }
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
/// Gets options on sockets.
///
/// # Parameters
///
/// - `sockfd`: File descriptor of the socket.
/// - `level`: The protocol level at which the option resides.
/// - `optname`: The name of the option.
/// - `optval`: Pointer to the buffer where the option value will be stored.
/// - `optlen`: Pointer to the length of the option value.
///
/// # Returns
///
/// The `getsockopt()` function returns `0` on success. On error, it returns `-1`
/// and sets `errno` to indicate the error.
///
/// # Safety
///
/// This function is unsafe because it may dereference raw pointers.
///
/// It is safe to call this function if the following conditions are met:
/// - `optval` points to a valid buffer of length `*optlen` (if not null).
/// - `optlen` points to a valid length (if not null).
///
#[unsafe(no_mangle)]
pub unsafe extern "C" fn getsockopt(
    sockfd: c_int,
    level: c_int,
    optname: c_int,
    optval: *mut c_void,
    optlen: *mut socklen_t,
) -> c_int {
    ::syslog::trace!(
        "getsockopt(): sockfd={sockfd:?}, level={level:?}, optname={optname:?}, \
         optval={optval:?}, optlen={optlen:?}"
    );
    // TODO: https://github.com/nanvix/nanvix/issues/591
    ::syslog::error!("getsockopt(): not implemented");
    unsafe {
        *__errno_location() = ErrorCode::InvalidSysCall.get();
    }
    -1
}

#[allow(clippy::missing_safety_doc)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn listen(sockfd: c_int, backlog: c_int) -> c_int {
    ::syslog::trace!("listen(): sockfd={:?}, backlog={:?}", sockfd, backlog);

    match socket::syscall::listen(sockfd, backlog) {
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

    match socket::syscall::recv(sockfd, buf, flags) {
        Ok(bytes_received) => bytes_received as ssize_t,
        Err(e) => {
            ::syslog::error!("recv(): failed to receive data through socket {:?}", e);
            *__errno_location() = e.code.get();
            -1
        },
    }
}

///
/// # Description
///
/// Receives data from a specific address.
///
/// # Parameters
///
/// - `sockfd`: File descriptor of the socket.
/// - `buf`: Pointer to the buffer where the received data will be stored.
/// - `len`: Length of the buffer.
/// - `flags`: Flags for receiving data.
/// - `sockaddr`: Pointer to the socket address structure to store the source address.
/// - `addrlen`: Pointer to the length of the socket address structure.
///
/// # Returns
///
/// The `recvfrom()` function returns the number of bytes received on success. On error, it returns `-1`
/// and sets `errno` to indicate the error.
///
/// # Safety
///
/// This function is unsafe because it may dereference raw pointers.
///
/// It is safe to call this function if the following conditions are met:
/// - `sockaddr` points to a valid socket address structure (if not null).
/// - `addrlen` points to a valid length (if not null).
/// - `buf` points to a valid buffer of length `len`.
///
#[unsafe(no_mangle)]
pub unsafe extern "C" fn recvfrom(
    sockfd: c_int,
    buf: *mut c_void,
    len: size_t,
    flags: c_int,
    sockaddr: *mut sockaddr,
    addrlen: *mut socklen_t,
) -> ssize_t {
    ::syslog::trace!(
        "recvfrom(): sockfd={sockfd:?}, buf={buf:?}, len={len:?}, flags={flags:?}, \
         sockaddr={sockaddr:?}, addrlen={addrlen:?}"
    );
    // TODO: https://github.com/nanvix/nanvix/issues/590
    unsafe {
        *__errno_location() = ErrorCode::InvalidSysCall.get();
    }
    -1
}

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
pub unsafe extern "C" fn recvmsg(sockfd: c_int, msg: *mut msghdr, flags: c_int) -> ssize_t {
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

    match socket::syscall::send(sockfd, buf, flags) {
        Ok(bytes_sent) => bytes_sent as ssize_t,
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
pub unsafe extern "C" fn sendmsg(sockfd: c_int, msg: *const msghdr, flags: c_int) -> ssize_t {
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
    len: size_t,
    flags: c_int,
    sockaddr: *const sockaddr,
    addrlen: socklen_t,
) -> ssize_t {
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

///
/// # Description
///
/// Sets options on sockets.
///
/// # Parameters
///
/// - `sockfd`: File descriptor of the socket.
/// - `level`: The protocol level at which the option resides.
/// - `optname`: The name of the option.
/// - `optval`: Pointer to the option value.
/// - `optlen`: Length of the option value.
///
/// # Returns
///
/// The `setsockopt()` function returns `0` on success. On error, it returns `-1` and sets `errno`
/// to indicate the error.
///
/// # Safety
///
/// This function is unsafe because it may dereference raw pointers.
///
/// It is safe to call this function if the following conditions are met:
/// - `optval` points to a valid buffer of length `optlen` (if not null).
///
#[unsafe(no_mangle)]
pub unsafe extern "C" fn setsockopt(
    sockfd: c_int,
    level: c_int,
    optname: c_int,
    optval: *const c_void,
    optlen: socklen_t,
) -> c_int {
    ::syslog::trace!(
        "setsockopt(): sockfd={sockfd:?}, level={level:?}, optname={optname:?}, \
         optval={optval:?}, optlen={optlen:?}"
    );
    // TODO: https://github.com/nanvix/nanvix/issues/471
    ::syslog::error!("setsockopt(): not implemented");
    unsafe {
        *__errno_location() = ErrorCode::InvalidSysCall.get();
    }
    -1
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
    match socket::syscall::socket(domain, typ, protocol) {
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

    match socket::syscall::shutdown(sockfd, how) {
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

    match socket::syscall::socketpair(domain, typ, protocol, socket_fds) {
        Ok(_) => 0,
        Err(e) => {
            *__errno_location() = e.code.get();
            -1
        },
    }
}
