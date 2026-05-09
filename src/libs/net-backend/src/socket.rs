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
        normalize_errno,
        raw_shutdown,
        raw_socketpair,
        raw_to_i32,
        sa_data_to_u8,
        socket_failed,
        SocklenT,
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

        let result = unsafe { libc::socket(domain.inner() as i32, typ.inner(), protocol.inner()) };

        if socket_failed(result) {
            let errno: i32 = last_socket_error();
            if is_interrupted(errno) {
                return Err(NetError::Interrupted);
            }
            error!("libc::socket(): failed with errno={errno:?}");
            let error: ErrorCode =
                ErrorCode::try_from(normalize_errno(errno)).unwrap_or(ErrorCode::ValueOutOfRange);
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
                let error: ErrorCode = ErrorCode::try_from(normalize_errno(errno))
                    .unwrap_or(ErrorCode::ValueOutOfRange);
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
                let error: ErrorCode = ErrorCode::try_from(normalize_errno(errno))
                    .unwrap_or(ErrorCode::ValueOutOfRange);
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
                let error: ErrorCode = ErrorCode::try_from(normalize_errno(errno))
                    .unwrap_or(ErrorCode::ValueOutOfRange);
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
                let error: ErrorCode = ErrorCode::try_from(normalize_errno(errno))
                    .unwrap_or(ErrorCode::ValueOutOfRange);
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
            let error: ErrorCode =
                ErrorCode::try_from(normalize_errno(errno)).unwrap_or(ErrorCode::ValueOutOfRange);
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
                let error: ErrorCode = ErrorCode::try_from(normalize_errno(errno))
                    .unwrap_or(ErrorCode::ValueOutOfRange);
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
                let error: ErrorCode = ErrorCode::try_from(normalize_errno(errno))
                    .unwrap_or(ErrorCode::ValueOutOfRange);
                Err(NetError::Errno(error))
            },
            ret => unreachable!("libc::close() returned invalid value {ret:?}"),
        }
    }
}

//==================================================================================================
// Tests
//==================================================================================================

#[cfg(test)]
mod test {
    use super::*;
    use crate::error::NetError;

    /// Tests that `connect()` rejects a socklen larger than `size_of::<sockaddr>()`.
    #[test]
    fn connect_socklen_too_large() {
        let backend: NetBackend =
            NetBackend::new().expect("platform initialization should succeed");
        let addr: sockaddr = sockaddr {
            sa_len: 0,
            sa_family: 2, // AF_INET
            sa_data: [0; 14],
        };
        let oversized: socklen_t = (core::mem::size_of::<libc::sockaddr>() + 1) as socklen_t;
        let result: Result<(), NetError> = backend.connect(0, &addr, oversized);
        assert!(result.is_err(), "connect with oversized socklen should fail");
        match result {
            Err(NetError::Errno(code)) => {
                assert_eq!(code, ErrorCode::InvalidArgument, "error should be InvalidArgument");
            },
            other => panic!("expected Errno(InvalidArgument), got {other:?}"),
        }
    }

    /// Tests that `socketpair()` returns `OperationNotSupported` on Windows.
    #[cfg(windows)]
    #[test]
    fn socketpair_unsupported_on_windows() {
        let backend: NetBackend =
            NetBackend::new().expect("platform initialization should succeed");
        let result: Result<(i32, i32), NetError> =
            backend.socketpair(AddressFamily::Unix, SocketType::Stream, Protocol::Ip);
        assert!(result.is_err(), "socketpair should fail on Windows");
        match result {
            Err(NetError::Errno(code)) => {
                assert_eq!(
                    code,
                    ErrorCode::OperationNotSupported,
                    "error should be OperationNotSupported"
                );
            },
            other => panic!("expected Errno(OperationNotSupported), got {other:?}"),
        }
    }

    /// Tests creating and closing a TCP socket.
    #[test]
    fn tcp_socket_lifecycle() {
        let backend: NetBackend =
            NetBackend::new().expect("platform initialization should succeed");
        let sockfd: i32 = backend
            .socket(AddressFamily::Inet, SocketType::Stream, Protocol::Tcp)
            .expect("creating a TCP socket should succeed");
        backend
            .close(sockfd)
            .expect("closing the socket should succeed");
    }

    /// Tests creating and closing a UDP socket.
    #[test]
    fn udp_socket_lifecycle() {
        let backend: NetBackend =
            NetBackend::new().expect("platform initialization should succeed");
        let sockfd: i32 = backend
            .socket(AddressFamily::Inet, SocketType::Datagram, Protocol::Udp)
            .expect("creating a UDP socket should succeed");
        backend
            .close(sockfd)
            .expect("closing the socket should succeed");
    }

    /// Tests that closing an invalid file descriptor returns an error.
    #[test]
    fn close_invalid_fd() {
        let backend: NetBackend =
            NetBackend::new().expect("platform initialization should succeed");
        let result: Result<(), NetError> = backend.close(-1);
        assert!(result.is_err(), "closing an invalid fd should fail");
    }

    /// Tests that `socket()` rejects `AddressFamily::Unspec`.
    #[test]
    fn socket_unspec_domain_rejected() {
        let backend: NetBackend =
            NetBackend::new().expect("platform initialization should succeed");
        let result: Result<i32, NetError> =
            backend.socket(AddressFamily::Unspec, SocketType::Stream, Protocol::Tcp);
        assert!(result.is_err(), "Unspec domain should be rejected");
    }

    /// Tests bind + getsockname roundtrip on a loopback address.
    #[test]
    fn bind_and_getsockname() {
        let backend: NetBackend =
            NetBackend::new().expect("platform initialization should succeed");
        let sockfd: i32 = backend
            .socket(AddressFamily::Inet, SocketType::Stream, Protocol::Tcp)
            .expect("creating socket should succeed");

        // Build sockaddr_in for 127.0.0.1:0 (port 0 = OS-assigned).
        let addr: sockaddr = sockaddr {
            sa_len: core::mem::size_of::<sockaddr>() as u8,
            sa_family: 2, // AF_INET
            // sa_data: [port_hi, port_lo, ip0, ip1, ip2, ip3, 0..]
            sa_data: [0, 0, 127, 0, 0, 1, 0, 0, 0, 0, 0, 0, 0, 0],
        };

        backend
            .bind(sockfd, &addr)
            .expect("bind to loopback should succeed");

        let local: sockaddr = backend
            .getsockname(sockfd)
            .expect("getsockname should succeed");

        // Verify the address family matches.
        assert_eq!(local.sa_family, 2, "returned family should be AF_INET");
        // Verify the IP is 127.0.0.1 (bytes 2..6 of sa_data).
        assert_eq!(&local.sa_data[2..6], &[127, 0, 0, 1], "IP should be 127.0.0.1");

        backend
            .close(sockfd)
            .expect("closing socket should succeed");
    }

    /// Tests a full TCP listen + connect + accept + send/recv cycle.
    #[test]
    fn tcp_listen_connect_accept_send_recv() {
        let backend: NetBackend =
            NetBackend::new().expect("platform initialization should succeed");

        // Create server socket.
        let server_fd: i32 = backend
            .socket(AddressFamily::Inet, SocketType::Stream, Protocol::Tcp)
            .expect("creating server socket should succeed");

        // Bind to loopback port 0.
        let bind_addr: sockaddr = sockaddr {
            sa_len: core::mem::size_of::<sockaddr>() as u8,
            sa_family: 2,
            sa_data: [0, 0, 127, 0, 0, 1, 0, 0, 0, 0, 0, 0, 0, 0],
        };
        backend
            .bind(server_fd, &bind_addr)
            .expect("server bind should succeed");
        backend.listen(server_fd, 1).expect("listen should succeed");

        // Retrieve the OS-assigned port.
        let server_addr: sockaddr = backend
            .getsockname(server_fd)
            .expect("getsockname should succeed");

        // Create client socket and connect.
        let client_fd: i32 = backend
            .socket(AddressFamily::Inet, SocketType::Stream, Protocol::Tcp)
            .expect("creating client socket should succeed");

        let connect_addr: sockaddr = sockaddr {
            sa_len: core::mem::size_of::<sockaddr>() as u8,
            sa_family: server_addr.sa_family,
            sa_data: server_addr.sa_data,
        };
        let socklen: socklen_t = core::mem::size_of::<sockaddr>() as socklen_t;
        backend
            .connect(client_fd, &connect_addr, socklen)
            .expect("connect should succeed");

        // Accept on server side.
        let (accepted_fd, _peer_addr) = backend.accept(server_fd).expect("accept should succeed");

        // Send data from client.
        let msg: &[u8] = b"hello";
        let sent: isize = backend
            .send(client_fd, msg, msg.len(), 0)
            .expect("send should succeed");
        assert_eq!(sent as usize, msg.len(), "should send all bytes");

        // Receive data on accepted socket.
        let mut buf: [u8; 16] = [0u8; 16];
        let buf_len: usize = buf.len();
        let received: isize = backend
            .recv(accepted_fd, &mut buf, buf_len, 0)
            .expect("recv should succeed");
        assert_eq!(&buf[..received as usize], msg, "received data should match sent data");

        // Cleanup.
        backend
            .close(accepted_fd)
            .expect("closing accepted fd should succeed");
        backend
            .close(client_fd)
            .expect("closing client fd should succeed");
        backend
            .close(server_fd)
            .expect("closing server fd should succeed");
    }

    /// Tests that `getpeername()` returns a valid address after a TCP connection.
    #[test]
    fn getpeername_after_connect() {
        let backend: NetBackend =
            NetBackend::new().expect("platform initialization should succeed");

        // Server: socket + bind + listen.
        let server_fd: i32 = backend
            .socket(AddressFamily::Inet, SocketType::Stream, Protocol::Tcp)
            .expect("creating server socket should succeed");
        let bind_addr: sockaddr = sockaddr {
            sa_len: core::mem::size_of::<sockaddr>() as u8,
            sa_family: 2,
            sa_data: [0, 0, 127, 0, 0, 1, 0, 0, 0, 0, 0, 0, 0, 0],
        };
        backend
            .bind(server_fd, &bind_addr)
            .expect("bind should succeed");
        backend.listen(server_fd, 1).expect("listen should succeed");

        let server_addr: sockaddr = backend
            .getsockname(server_fd)
            .expect("getsockname should succeed");

        // Client: socket + connect.
        let client_fd: i32 = backend
            .socket(AddressFamily::Inet, SocketType::Stream, Protocol::Tcp)
            .expect("creating client socket should succeed");
        let socklen: socklen_t = core::mem::size_of::<sockaddr>() as socklen_t;
        backend
            .connect(client_fd, &server_addr, socklen)
            .expect("connect should succeed");

        // Accept then query peer.
        let (accepted_fd, _) = backend.accept(server_fd).expect("accept should succeed");

        let peer: sockaddr = backend
            .getpeername(client_fd)
            .expect("getpeername should succeed");
        assert_eq!(peer.sa_family, 2, "peer family should be AF_INET");
        assert_eq!(&peer.sa_data[2..6], &[127, 0, 0, 1], "peer IP should be 127.0.0.1");

        backend
            .close(accepted_fd)
            .expect("closing accepted fd should succeed");
        backend
            .close(client_fd)
            .expect("closing client fd should succeed");
        backend
            .close(server_fd)
            .expect("closing server fd should succeed");
    }

    /// Tests that `shutdown(Write)` on the sender causes `recv()` to return 0 on the reader.
    #[test]
    fn shutdown_write_then_recv_eof() {
        let backend: NetBackend =
            NetBackend::new().expect("platform initialization should succeed");

        // Set up connected pair via listen/connect/accept.
        let server_fd: i32 = backend
            .socket(AddressFamily::Inet, SocketType::Stream, Protocol::Tcp)
            .expect("creating server socket should succeed");
        let bind_addr: sockaddr = sockaddr {
            sa_len: core::mem::size_of::<sockaddr>() as u8,
            sa_family: 2,
            sa_data: [0, 0, 127, 0, 0, 1, 0, 0, 0, 0, 0, 0, 0, 0],
        };
        backend
            .bind(server_fd, &bind_addr)
            .expect("bind should succeed");
        backend.listen(server_fd, 1).expect("listen should succeed");

        let server_addr: sockaddr = backend
            .getsockname(server_fd)
            .expect("getsockname should succeed");
        let client_fd: i32 = backend
            .socket(AddressFamily::Inet, SocketType::Stream, Protocol::Tcp)
            .expect("creating client socket should succeed");
        let socklen: socklen_t = core::mem::size_of::<sockaddr>() as socklen_t;
        backend
            .connect(client_fd, &server_addr, socklen)
            .expect("connect should succeed");
        let (accepted_fd, _) = backend.accept(server_fd).expect("accept should succeed");

        // Shutdown write on the client side.
        backend
            .shutdown(client_fd, Shutdown::Write)
            .expect("shutdown(Write) should succeed");

        // Recv on the accepted socket should return 0 (EOF).
        let mut buf: [u8; 16] = [0u8; 16];
        let buf_len: usize = buf.len();
        let received: isize = backend
            .recv(accepted_fd, &mut buf, buf_len, 0)
            .expect("recv should succeed");
        assert_eq!(received, 0, "recv after shutdown(Write) should return 0 (EOF)");

        backend
            .close(accepted_fd)
            .expect("closing accepted fd should succeed");
        backend
            .close(client_fd)
            .expect("closing client fd should succeed");
        backend
            .close(server_fd)
            .expect("closing server fd should succeed");
    }
}
