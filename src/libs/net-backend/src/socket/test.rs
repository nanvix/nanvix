// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

use super::*;
use crate::error::NetError;

/// Tests non-blocking socket readiness before and after data arrives.
#[cfg(unix)]
#[test]
fn poll_socket_tracks_read_and_write_readiness() {
    let backend: NetBackend = NetBackend::new().expect("platform initialization should succeed");
    let (left, right): (i32, i32) = backend
        .socketpair(AddressFamily::Unix, SocketType::Stream, Protocol::Ip)
        .expect("socketpair should succeed");

    assert_eq!(
        backend
            .poll_socket(left, POLLIN)
            .expect("poll read interest"),
        0,
        "an empty stream socket should not be readable"
    );
    assert_eq!(
        backend
            .poll_socket(left, POLLOUT)
            .expect("poll write interest"),
        POLLOUT,
        "a connected stream socket should be writable"
    );

    assert_eq!(backend.send(right, &[1], 1, 0).expect("send one byte"), 1);
    assert_eq!(
        backend
            .poll_socket(left, POLLIN)
            .expect("poll readable socket"),
        POLLIN,
        "queued data should make the socket readable"
    );

    backend.close(left).expect("close left socket");
    backend.close(right).expect("close right socket");
}

/// Tests that a stream hangup suppresses write readiness as required by POSIX.
#[cfg(unix)]
#[test]
fn poll_socket_hangup_is_not_writable() {
    let backend: NetBackend = NetBackend::new().expect("platform initialization should succeed");
    let (left, right): (i32, i32) = backend
        .socketpair(AddressFamily::Unix, SocketType::Stream, Protocol::Ip)
        .expect("socketpair should succeed");
    backend.close(right).expect("close peer socket");

    let revents: c_short = backend
        .poll_socket(left, POLLIN | POLLOUT)
        .expect("poll hung-up socket");
    assert_ne!(revents & POLLHUP, 0, "peer closure should report hangup");
    assert_eq!(revents & POLLOUT, 0, "hangup and write readiness are mutually exclusive");

    backend.close(left).expect("close local socket");
}

/// Tests that `connect()` rejects a socklen larger than `size_of::<sockaddr>()`.
#[test]
fn connect_socklen_too_large() {
    let backend: NetBackend = NetBackend::new().expect("platform initialization should succeed");
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
    let backend: NetBackend = NetBackend::new().expect("platform initialization should succeed");
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
    let backend: NetBackend = NetBackend::new().expect("platform initialization should succeed");
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
    let backend: NetBackend = NetBackend::new().expect("platform initialization should succeed");
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
    let backend: NetBackend = NetBackend::new().expect("platform initialization should succeed");
    let result: Result<(), NetError> = backend.close(-1);
    assert!(result.is_err(), "closing an invalid fd should fail");
}

/// Tests that a `recvfrom` on an empty non-blocking socket reports would-block.
#[test]
fn nonblocking_recvfrom_would_block() {
    let backend: NetBackend = NetBackend::new().expect("platform initialization should succeed");
    let sockfd: i32 = backend
        .socket(AddressFamily::Inet, SocketType::Datagram, Protocol::Udp)
        .expect("creating a UDP socket should succeed");
    let addr: sockaddr = sockaddr {
        sa_len: core::mem::size_of::<sockaddr>() as u8,
        sa_family: 2, // AF_INET
        sa_data: [0, 0, 127, 0, 0, 1, 0, 0, 0, 0, 0, 0, 0, 0],
    };
    backend
        .bind(sockfd, &addr)
        .expect("binding UDP socket should succeed");
    backend
        .set_nonblocking(sockfd, true)
        .expect("enabling non-blocking mode should succeed");

    let mut buf: [u8; 16] = [0; 16];
    let buf_len: usize = buf.len();
    let result: Result<(isize, sockaddr), NetError> =
        backend.recvfrom(sockfd, &mut buf, buf_len, 0);
    match result {
        Err(ref e) => assert!(e.is_would_block(), "expected would-block, got {e:?}"),
        Ok(_) => panic!("recvfrom on an empty non-blocking socket should not succeed"),
    }

    backend
        .close(sockfd)
        .expect("closing the socket should succeed");
}

/// Tests that `socket()` rejects `AddressFamily::Unspec`.
#[test]
fn socket_unspec_domain_rejected() {
    let backend: NetBackend = NetBackend::new().expect("platform initialization should succeed");
    let result: Result<i32, NetError> =
        backend.socket(AddressFamily::Unspec, SocketType::Stream, Protocol::Tcp);
    assert!(result.is_err(), "Unspec domain should be rejected");
}

/// Tests bind + getsockname roundtrip on a loopback address.
#[test]
fn bind_and_getsockname() {
    let backend: NetBackend = NetBackend::new().expect("platform initialization should succeed");
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
    let backend: NetBackend = NetBackend::new().expect("platform initialization should succeed");

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
    let backend: NetBackend = NetBackend::new().expect("platform initialization should succeed");

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
    let backend: NetBackend = NetBackend::new().expect("platform initialization should succeed");

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
