// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use crate::{
    error::NetError,
    types::{
        LibcShutdownReason,
        LibcSocketAddress,
        LibcSocketDomain,
        LibcSocketProtocol,
        LibcSocketType,
    },
    NetBackend,
};
use ::log::{
    debug,
    error,
    warn,
};
use ::sys::error::ErrorCode;
use ::sysapi::sys_socket::{
    sockaddr,
    socklen_t,
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

        match unsafe { libc::socket(domain.inner() as i32, typ.inner(), protocol.inner()) } {
            -1 => {
                let errno: i32 = unsafe { *libc::__errno_location() };
                if errno == libc::EINTR {
                    return Err(NetError::Interrupted);
                }
                error!("libc::socket(): failed with errno={errno:?}");
                let error: ErrorCode = ErrorCode::try_from(errno).unwrap_or_else(|_| {
                    warn!("unknown errno value {errno:?}, mapping to ValueOutOfRange");
                    ErrorCode::ValueOutOfRange
                });
                Err(NetError::Errno(error))
            },
            sockfd => {
                debug!("libc::socket(): fd={sockfd:?}");
                Ok(sockfd)
            },
        }
    }

    /// Creates a pair of connected sockets.
    pub fn socketpair(
        &self,
        domain: AddressFamily,
        typ: SocketType,
        protocol: Protocol,
    ) -> Result<(i32, i32), NetError> {
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
            libc::socketpair(domain.inner() as i32, typ.inner(), protocol.inner(), sv.as_mut_ptr())
        } {
            -1 => {
                let errno: i32 = unsafe { *libc::__errno_location() };
                if errno == libc::EINTR {
                    return Err(NetError::Interrupted);
                }
                error!("libc::socketpair(): failed with errno={errno:?}");
                let error: ErrorCode = ErrorCode::try_from(errno).unwrap_or_else(|_| {
                    warn!("unknown errno value {errno:?}, mapping to ValueOutOfRange");
                    ErrorCode::ValueOutOfRange
                });
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
        let socklen: socklen_t = core::mem::size_of::<libc::sockaddr>() as socklen_t;

        debug!(
            "libc::bind(): sockfd={sockfd:?}, sockaddr.sa_family={:?}, sockaddr.sa_data={:?}, \
             socklen={socklen:?}",
            sockaddr.inner().sa_family,
            sockaddr.inner().sa_data,
        );

        match unsafe { libc::bind(sockfd, &sockaddr.inner() as *const libc::sockaddr, socklen) } {
            -1 => {
                let errno: i32 = unsafe { *libc::__errno_location() };
                if errno == libc::EINTR {
                    return Err(NetError::Interrupted);
                }
                error!("libc::bind(): failed with errno={errno:?}");
                let error: ErrorCode = ErrorCode::try_from(errno).unwrap_or_else(|_| {
                    warn!("unknown errno value {errno:?}, mapping to ValueOutOfRange");
                    ErrorCode::ValueOutOfRange
                });
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
            return Err(NetError::Errno(::sys::error::ErrorCode::InvalidArgument));
        }

        let sockaddr: LibcSocketAddress =
            LibcSocketAddress::try_from(*addr).map_err(|e| NetError::Errno(e.code))?;

        debug!(
            "libc::connect(): sockfd={sockfd:?}, sockaddr.sa_family={:?}, sockaddr.sa_data={:?}, \
             socklen={socklen:?}",
            sockaddr.inner().sa_family,
            sockaddr.inner().sa_data,
        );

        match unsafe {
            libc::connect(
                sockfd,
                &sockaddr.inner() as *const libc::sockaddr,
                socklen as libc::socklen_t,
            )
        } {
            -1 => {
                let errno: i32 = unsafe { *libc::__errno_location() };
                if errno == libc::EINTR {
                    return Err(NetError::Interrupted);
                }
                error!("libc::connect(): failed with errno={errno:?}");
                let error: ErrorCode = ErrorCode::try_from(errno).unwrap_or_else(|_| {
                    warn!("unknown errno value {errno:?}, mapping to ValueOutOfRange");
                    ErrorCode::ValueOutOfRange
                });
                Err(NetError::Errno(error))
            },
            _ => Ok(()),
        }
    }

    /// Listens for connections on a socket.
    pub fn listen(&self, sockfd: i32, backlog: i32) -> Result<(), NetError> {
        debug!("libc::listen(): sockfd={sockfd:?}, backlog={backlog:?}");

        match unsafe { libc::listen(sockfd, backlog) } {
            -1 => {
                let errno: i32 = unsafe { *libc::__errno_location() };
                if errno == libc::EINTR {
                    return Err(NetError::Interrupted);
                }
                error!("libc::listen(): failed with errno={errno:?}");
                let error: ErrorCode = ErrorCode::try_from(errno).unwrap_or_else(|_| {
                    warn!("unknown errno value {errno:?}, mapping to ValueOutOfRange");
                    ErrorCode::ValueOutOfRange
                });
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
        let mut address_len: libc::socklen_t =
            core::mem::size_of::<libc::sockaddr>() as libc::socklen_t;

        debug!("libc::accept(): sockfd={sockfd:?}");

        match unsafe { libc::accept(sockfd, &mut address, &mut address_len) } {
            -1 => {
                let errno: libc::c_int = unsafe { *libc::__errno_location() };
                if errno == libc::EINTR {
                    return Err(NetError::Interrupted);
                }
                error!("libc::accept(): failed with errno={errno:?}");
                let error: ErrorCode = ErrorCode::try_from(errno).unwrap_or_else(|_| {
                    warn!("unknown errno value {errno:?}, mapping to ValueOutOfRange");
                    ErrorCode::ValueOutOfRange
                });
                Err(NetError::Errno(error))
            },
            new_sockfd => {
                let mut sa_data: [u8; 14] = [0u8; 14];
                for (i, &b) in address.sa_data.iter().enumerate() {
                    sa_data[i] = b as u8;
                }
                let addr: sockaddr = sockaddr {
                    sa_len: address_len as u8,
                    sa_family: address.sa_family as u8,
                    sa_data,
                };
                Ok((new_sockfd, addr))
            },
        }
    }

    /// Shuts down part of a full-duplex connection.
    pub fn shutdown(&self, sockfd: i32, how: Shutdown) -> Result<(), NetError> {
        let how: LibcShutdownReason = LibcShutdownReason::from(how);

        debug!("libc::shutdown(): sockfd={sockfd:?}, how={:?}", how.inner());

        match unsafe { libc::shutdown(sockfd, how.inner()) } {
            0 => Ok(()),
            -1 => {
                let errno: libc::c_int = unsafe { *libc::__errno_location() };
                if errno == libc::EINTR {
                    return Err(NetError::Interrupted);
                }
                error!("libc::shutdown(): failed with errno={errno:?}");
                let error: ErrorCode = ErrorCode::try_from(errno).unwrap_or_else(|_| {
                    warn!("unknown errno value {errno:?}, mapping to ValueOutOfRange");
                    ErrorCode::ValueOutOfRange
                });
                Err(NetError::Errno(error))
            },
            ret => unreachable!("libc::shutdown() returned invalid value {ret:?}"),
        }
    }
}
