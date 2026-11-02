// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::{
    error::NetError,
    platform::{
        close_socket,
        i32_to_raw,
        is_interrupted,
        last_socket_error,
        raw_poll,
        raw_set_nonblocking,
        raw_shutdown,
        raw_socketpair,
        raw_to_i32,
        sa_data_to_u8,
        socket_failed,
        SocklenT,
        PLATFORM_POLLERR,
        PLATFORM_POLLHUP,
        PLATFORM_POLLIN,
        PLATFORM_POLLNVAL,
        PLATFORM_POLLOUT,
        PLATFORM_POLLPRI,
        PLATFORM_POLLRDBAND,
        PLATFORM_POLLRDNORM,
        PLATFORM_POLLWRBAND,
        PLATFORM_POLLWRNORM,
        SOCKETPAIR_SUPPORTED,
    },
    types::{
        LibcShutdownReason,
        LibcSocketAddress,
        LibcSocketDomain,
        LibcSocketProtocol,
        LibcSocketType,
    },
    NetBackend,
};
use ::core::mem;
use ::log::{
    debug,
    error,
};
use ::sys::error::{
    connect_errno_to_error_code,
    errno_to_error_code,
    ErrorCode,
};
use ::sysapi::{
    ffi::c_short,
    poll::{
        poll_errors::{
            POLLERR,
            POLLHUP,
            POLLNVAL,
        },
        poll_flags::{
            POLLIN,
            POLLOUT,
            POLLPRI,
            POLLRDBAND,
            POLLRDNORM,
            POLLWRBAND,
            POLLWRNORM,
        },
    },
    sys_socket::{
        sockaddr,
        socklen_t,
    },
};
use ::syscall::{
    netinet::in_::Protocol,
    sys::socket::{
        AddressFamily,
        Shutdown,
        SocketType,
    },
};

//==================================================================================================
// Socket Operations
//==================================================================================================

impl NetBackend {
    /// Returns the events that can complete immediately on `sockfd`.
    pub fn poll_socket(&self, sockfd: i32, events: c_short) -> Result<c_short, NetError> {
        let mut platform_events: c_short = 0;
        if events & POLLIN != 0 {
            platform_events |= PLATFORM_POLLIN;
        }
        if events & POLLRDNORM != 0 {
            platform_events |= PLATFORM_POLLRDNORM;
        }
        if events & POLLRDBAND != 0 {
            platform_events |= PLATFORM_POLLRDBAND;
        }
        if events & POLLPRI != 0 {
            platform_events |= PLATFORM_POLLPRI;
        }
        if events & POLLOUT != 0 {
            platform_events |= PLATFORM_POLLOUT;
        }
        if events & POLLWRNORM != 0 {
            platform_events |= PLATFORM_POLLWRNORM;
        }
        if let Some(platform_pollwrband) = PLATFORM_POLLWRBAND {
            if events & POLLWRBAND != 0 {
                platform_events |= platform_pollwrband;
            }
        }

        let mut platform_revents: c_short = 0;
        let result: libc::c_int = loop {
            let result: libc::c_int =
                unsafe { raw_poll(i32_to_raw(sockfd), platform_events, &mut platform_revents) };
            if result < 0 && is_interrupted(last_socket_error()) {
                continue;
            }
            break result;
        };
        if result < 0 {
            let errno: i32 = last_socket_error();
            return Err(NetError::Errno(errno_to_error_code(errno)));
        }

        let mut revents: c_short = 0;
        if platform_revents & (PLATFORM_POLLIN | PLATFORM_POLLRDNORM) != 0 {
            revents |= events & (POLLIN | POLLRDNORM);
        }
        if platform_revents & PLATFORM_POLLRDBAND != 0 {
            revents |= events & POLLRDBAND;
        }
        if platform_revents & PLATFORM_POLLPRI != 0 {
            revents |= events & POLLPRI;
        }
        if platform_revents & (PLATFORM_POLLOUT | PLATFORM_POLLWRNORM) != 0 {
            revents |= events & (POLLOUT | POLLWRNORM);
        }
        if let Some(platform_pollwrband) = PLATFORM_POLLWRBAND {
            if platform_revents & platform_pollwrband != 0 {
                revents |= events & POLLWRBAND;
            }
        }
        if platform_revents & PLATFORM_POLLERR != 0 {
            revents |= POLLERR;
        }
        if platform_revents & PLATFORM_POLLHUP != 0 {
            revents |= POLLHUP;
            revents &= !(POLLOUT | POLLWRNORM);
        }
        if platform_revents & PLATFORM_POLLNVAL != 0 {
            revents |= POLLNVAL;
        }
        Ok(revents)
    }

    /// Creates a new socket.
    pub fn socket(
        &self,
        domain: AddressFamily,
        typ: SocketType,
        protocol: Protocol,
    ) -> Result<i32, NetError> {
        let domain: LibcSocketDomain =
            LibcSocketDomain::try_from_nanvix(domain).map_err(|e| NetError::Errno(e.code))?;
        let typ: LibcSocketType = LibcSocketType::from_nanvix(typ);
        let protocol: LibcSocketProtocol = LibcSocketProtocol::from_nanvix(protocol);

        debug!(
            "libc::socket(): domain={:?}, type={:?}, protocol={protocol:?}",
            domain.inner(),
            typ.inner(),
        );

        let result = unsafe { libc::socket(domain.inner() as i32, typ.inner(), protocol.inner()) };

        if socket_failed(result) {
            let errno: i32 = last_socket_error();
            if is_interrupted(errno) {
                return Err(NetError::Interrupted);
            }
            error!("libc::socket(): failed with errno={errno:?}");
            let error: ErrorCode = errno_to_error_code(errno);
            Err(NetError::Errno(error))
        } else {
            let sockfd = raw_to_i32(result);
            debug!("libc::socket(): fd={sockfd:?}");
            Ok(sockfd)
        }
    }

    /// Creates a pair of connected sockets.
    ///
    /// Not supported on Windows — returns `OperationNotSupported`.
    pub fn socketpair(
        &self,
        domain: AddressFamily,
        typ: SocketType,
        protocol: Protocol,
    ) -> Result<(i32, i32), NetError> {
        if !SOCKETPAIR_SUPPORTED {
            error!("socketpair(): not supported on this platform");
            return Err(NetError::Errno(ErrorCode::OperationNotSupported));
        }

        let domain: LibcSocketDomain =
            LibcSocketDomain::try_from_nanvix(domain).map_err(|e| NetError::Errno(e.code))?;
        let typ: LibcSocketType = LibcSocketType::from_nanvix(typ);
        let protocol: LibcSocketProtocol = LibcSocketProtocol::from_nanvix(protocol);

        let mut sv: [libc::c_int; 2] = [0; 2];

        debug!(
            "libc::socketpair(): domain={:?}, type={:?}, protocol={protocol:?}",
            domain.inner(),
            typ.inner(),
        );

        match unsafe {
            raw_socketpair(domain.inner() as i32, typ.inner(), protocol.inner(), &mut sv)
        } {
            -1 => {
                let errno: i32 = last_socket_error();
                if is_interrupted(errno) {
                    return Err(NetError::Interrupted);
                }
                error!("libc::socketpair(): failed with errno={errno:?}");
                let error: ErrorCode = errno_to_error_code(errno);
                Err(NetError::Errno(error))
            },
            _ => {
                debug!("libc::socketpair(): fds={sv:?}");
                Ok((sv[0], sv[1]))
            },
        }
    }

    /// Binds a socket to an address.
    pub fn bind(&self, sockfd: i32, addr: &sockaddr) -> Result<(), NetError> {
        let sockaddr: LibcSocketAddress =
            LibcSocketAddress::try_from(*addr).map_err(|e| NetError::Errno(e.code))?;
        let socklen: socklen_t = mem::size_of_val(&sockaddr) as socklen_t;

        debug!(
            "libc::bind(): sockfd={sockfd:?}, sockaddr.sa_family={:?}, sockaddr.sa_data={:?}, \
             socklen={socklen:?}",
            sockaddr.inner().sa_family,
            sockaddr.inner().sa_data,
        );

        let raw = i32_to_raw(sockfd);
        match unsafe {
            libc::bind(raw, &sockaddr.inner() as *const libc::sockaddr, socklen as SocklenT)
        } {
            -1 => {
                let errno: i32 = last_socket_error();
                if is_interrupted(errno) {
                    return Err(NetError::Interrupted);
                }
                error!("libc::bind(): failed with errno={errno:?}");
                let error: ErrorCode = errno_to_error_code(errno);
                Err(NetError::Errno(error))
            },
            _ => Ok(()),
        }
    }

    /// Connects a socket to an address.
    pub fn connect(
        &self,
        sockfd: i32,
        addr: &sockaddr,
        socklen: socklen_t,
    ) -> Result<(), NetError> {
        // Validate that socklen does not exceed the size of the address structure.
        if (socklen as usize) > core::mem::size_of::<libc::sockaddr>() {
            error!(
                "connect(): socklen ({socklen}) exceeds size of sockaddr ({})",
                core::mem::size_of::<libc::sockaddr>()
            );
            return Err(NetError::Errno(ErrorCode::InvalidArgument));
        }

        let sockaddr: LibcSocketAddress =
            LibcSocketAddress::try_from(*addr).map_err(|e| NetError::Errno(e.code))?;

        debug!(
            "libc::connect(): sockfd={sockfd:?}, sockaddr.sa_family={:?}, sockaddr.sa_data={:?}, \
             socklen={socklen:?}",
            sockaddr.inner().sa_family,
            sockaddr.inner().sa_data,
        );

        let raw = i32_to_raw(sockfd);
        match unsafe {
            libc::connect(raw, &sockaddr.inner() as *const libc::sockaddr, socklen as SocklenT)
        } {
            -1 => {
                let errno: i32 = last_socket_error();
                if is_interrupted(errno) {
                    return Err(NetError::Interrupted);
                }
                error!("libc::connect(): failed with errno={errno:?}");
                let error: ErrorCode = connect_errno_to_error_code(errno);
                Err(NetError::Errno(error))
            },
            _ => Ok(()),
        }
    }

    /// Listens for connections on a socket.
    pub fn listen(&self, sockfd: i32, backlog: i32) -> Result<(), NetError> {
        debug!("libc::listen(): sockfd={sockfd:?}, backlog={backlog:?}");

        let raw = i32_to_raw(sockfd);
        match unsafe { libc::listen(raw, backlog) } {
            -1 => {
                let errno: i32 = last_socket_error();
                if is_interrupted(errno) {
                    return Err(NetError::Interrupted);
                }
                error!("libc::listen(): failed with errno={errno:?}");
                let error: ErrorCode = errno_to_error_code(errno);
                Err(NetError::Errno(error))
            },
            _ => Ok(()),
        }
    }

    /// Accepts a connection on a socket.
    ///
    /// Returns the new socket file descriptor and the peer address.
    pub fn accept(&self, sockfd: i32) -> Result<(i32, sockaddr), NetError> {
        let mut address: libc::sockaddr = unsafe { core::mem::zeroed() };
        let mut address_len: SocklenT = core::mem::size_of::<libc::sockaddr>() as SocklenT;

        debug!("libc::accept(): sockfd={sockfd:?}");

        let raw = i32_to_raw(sockfd);
        let result = unsafe { libc::accept(raw, &mut address, &mut address_len) };

        if socket_failed(result) {
            let errno: i32 = last_socket_error();
            if is_interrupted(errno) {
                return Err(NetError::Interrupted);
            }
            error!("libc::accept(): failed with errno={errno:?}");
            let error: ErrorCode = errno_to_error_code(errno);
            Err(NetError::Errno(error))
        } else {
            let new_sockfd = raw_to_i32(result);
            let addr: sockaddr = sockaddr {
                sa_len: address_len as u8,
                sa_family: address.sa_family as u8,
                sa_data: sa_data_to_u8(address.sa_data),
            };
            Ok((new_sockfd, addr))
        }
    }

    /// Shuts down part of a full-duplex connection.
    pub fn shutdown(&self, sockfd: i32, how: Shutdown) -> Result<(), NetError> {
        let how: LibcShutdownReason = LibcShutdownReason::from(how);

        debug!("libc::shutdown(): sockfd={sockfd:?}, how={:?}", how.inner());

        let raw = i32_to_raw(sockfd);
        let result = unsafe { raw_shutdown(raw, how.inner()) };

        match result {
            0 => Ok(()),
            -1 => {
                let errno: i32 = last_socket_error();
                if is_interrupted(errno) {
                    return Err(NetError::Interrupted);
                }
                error!("libc::shutdown(): failed with errno={errno:?}");
                let error: ErrorCode = errno_to_error_code(errno);
                Err(NetError::Errno(error))
            },
            ret => unreachable!("libc::shutdown() returned invalid value {ret:?}"),
        }
    }

    /// Closes a socket file descriptor.
    pub fn close(&self, sockfd: i32) -> Result<(), NetError> {
        debug!("libc::close(): sockfd={sockfd:?}");

        let raw = i32_to_raw(sockfd);
        match unsafe { close_socket(raw) } {
            0 => Ok(()),
            -1 => {
                let errno: i32 = last_socket_error();
                if is_interrupted(errno) {
                    return Err(NetError::Interrupted);
                }
                error!("libc::close(): failed with errno={errno:?}");
                let error: ErrorCode = errno_to_error_code(errno);
                Err(NetError::Errno(error))
            },
            ret => unreachable!("libc::close() returned invalid value {ret:?}"),
        }
    }

    /// Enables or disables non-blocking mode on a socket.
    ///
    /// When non-blocking mode is enabled, I/O operations that cannot complete immediately fail with
    /// an error for which [`NetError::is_would_block`] returns `true`, and a `connect()` that cannot
    /// complete immediately reports [`NetError::is_in_progress`].
    pub fn set_nonblocking(&self, sockfd: i32, nonblocking: bool) -> Result<(), NetError> {
        debug!("set_nonblocking(): sockfd={sockfd:?}, nonblocking={nonblocking:?}");

        let raw = i32_to_raw(sockfd);
        // SAFETY: `raw` is the platform socket handle supplied by the caller. Invalid handles are
        // reported by the OS and converted to `NetError` below.
        match unsafe { raw_set_nonblocking(raw, nonblocking) } {
            0 => Ok(()),
            _ => {
                let errno: i32 = last_socket_error();
                if is_interrupted(errno) {
                    return Err(NetError::Interrupted);
                }
                error!("set_nonblocking(): failed with errno={errno:?}");
                let error: ErrorCode = errno_to_error_code(errno);
                Err(NetError::Errno(error))
            },
        }
    }
}

//==================================================================================================
// Tests
//==================================================================================================

#[cfg(test)]
mod test;
