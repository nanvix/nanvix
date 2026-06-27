// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use ::alloc::format;
use ::sys::error::Error;
use ::syscall::{
    netinet::in_::Protocol,
    sys::{
        socket::{
            AddressFamily,
            Shutdown,
            SocketAddr,
            SocketType,
            syscall::{
                bind,
                getpeername,
                getsockname,
                listen,
                recv,
                send,
                shutdown,
                socket,
                socketpair,
            },
        },
        un::SocketAddrUnix,
    },
    unistd::{
        close,
        getpid,
        unlink,
    },
};

//==================================================================================================
// Constants
//==================================================================================================

const LISTEN_BACKLOG: i32 = 5;

//==================================================================================================
// Helper Functions
//==================================================================================================

/// Creates a new Unix stream socket.
fn new_unbound_socket() -> Result<i32, Error> {
    socket(AddressFamily::Unix, SocketType::Stream, Protocol::Ip)
}

/// Creates a new Unix stream socket bound to the given address.
fn new_bound_socket(addr: &SocketAddr) -> Result<i32, Error> {
    let sockfd: i32 = new_unbound_socket()?;
    bind(sockfd, addr)?;
    Ok(sockfd)
}

/// Creates a new Unix listening socket bound to the given address.
fn new_listening_socket(addr: &SocketAddr) -> Result<i32, Error> {
    let sockfd: i32 = new_bound_socket(addr)?;
    listen(sockfd, LISTEN_BACKLOG)?;
    Ok(sockfd)
}

/// Creates a pair of connected Unix stream sockets.
fn new_socket_pair() -> Result<[i32; 2], Error> {
    let mut fds: [i32; 2] = [-1, -1];
    socketpair(AddressFamily::Unix, SocketType::Stream, Protocol::Ip, &mut fds)?;
    Ok(fds)
}

//==================================================================================================
// Tests
//==================================================================================================

/// Tests if we succeed to create a socket.
fn test_create_socket() -> Result<(), Error> {
    let sockfd: i32 = new_unbound_socket()?;
    close(sockfd)?;
    Ok(())
}

/// Tests if we succeed to bind a socket.
fn test_bind_socket(addr: &SocketAddr, path: &str) -> Result<(), Error> {
    let sockfd: i32 = new_bound_socket(addr)?;
    close(sockfd)?;
    unlink(path)?;
    Ok(())
}

/// Tests if we succeed to create a listening socket.
fn test_listen_socket(addr: &SocketAddr, path: &str) -> Result<(), Error> {
    let sockfd: i32 = new_listening_socket(addr)?;
    close(sockfd)?;
    unlink(path)?;
    Ok(())
}

/// Tests if we succeed to get the name of a bound socket.
fn test_getsockname_bound_socket(addr: &SocketAddr, path: &str) -> Result<(), Error> {
    let sockfd: i32 = new_bound_socket(addr)?;

    let mut result_addr: SocketAddr = SocketAddr::Unix(SocketAddrUnix::new(""));
    getsockname(sockfd, &mut result_addr)?;

    assert_eq!(*addr, result_addr, "bound socket address mismatch");

    close(sockfd)?;
    unlink(path)?;
    Ok(())
}

/// Tests if we succeed to get the name of a listening socket.
fn test_getsockname_listening_socket(addr: &SocketAddr, path: &str) -> Result<(), Error> {
    let sockfd: i32 = new_listening_socket(addr)?;

    let mut result_addr: SocketAddr = SocketAddr::Unix(SocketAddrUnix::new(""));
    getsockname(sockfd, &mut result_addr)?;

    assert_eq!(*addr, result_addr, "listening socket address mismatch");

    close(sockfd)?;
    unlink(path)?;
    Ok(())
}

/// Tests if we succeed to create a pair of connected sockets.
fn test_create_socket_pair() -> Result<(), Error> {
    let fds: [i32; 2] = new_socket_pair()?;
    close(fds[0])?;
    close(fds[1])?;
    Ok(())
}

/// Tests if we succeed to get the peer name of a connected socket.
fn test_getpeername_socket_pair() -> Result<(), Error> {
    let fds: [i32; 2] = new_socket_pair()?;

    let mut self_addr: SocketAddr = SocketAddr::Unix(SocketAddrUnix::new(""));
    getsockname(fds[0], &mut self_addr)?;

    let mut peer_addr: SocketAddr = SocketAddr::Unix(SocketAddrUnix::new(""));
    getpeername(fds[0], &mut peer_addr)?;

    assert_eq!(self_addr, peer_addr, "peer address mismatch");

    close(fds[0])?;
    close(fds[1])?;
    Ok(())
}

/// Tests if we succeed to send and receive data through a socket pair.
fn test_send_recv() -> Result<(), Error> {
    let fds: [i32; 2] = new_socket_pair()?;

    let msg: &[u8] = b"hello";
    let sent: usize = send(fds[0], msg, 0)?;
    assert_eq!(sent, msg.len(), "unexpected send count");

    let mut buf: [u8; 6] = [0u8; 6];
    let received: usize = recv(fds[1], &mut buf, 0)?;
    assert_eq!(received, msg.len(), "unexpected recv count");
    assert_eq!(&buf[..msg.len()], msg, "received data mismatch");

    close(fds[0])?;
    close(fds[1])?;
    Ok(())
}

/// Tests if we succeed to shutdown a pair of connected sockets.
fn test_shutdown_socket_pair() -> Result<(), Error> {
    let fds: [i32; 2] = new_socket_pair()?;

    shutdown(fds[0], Shutdown::ReadWrite)?;
    shutdown(fds[1], Shutdown::ReadWrite)?;

    close(fds[0])?;
    close(fds[1])?;
    Ok(())
}

//==================================================================================================
// Entry Point
//==================================================================================================

/// Runs all Unix domain socket tests.
pub fn run() -> Result<(), Error> {
    let pid: i32 = getpid()?.into();
    let socket_name: alloc::string::String = format!("rusttest{}", pid);
    let addr: SocketAddr = SocketAddr::Unix(SocketAddrUnix::new(&socket_name));

    test_create_socket()?;
    test_bind_socket(&addr, &socket_name)?;
    test_listen_socket(&addr, &socket_name)?;
    test_getsockname_bound_socket(&addr, &socket_name)?;
    test_getsockname_listening_socket(&addr, &socket_name)?;
    test_create_socket_pair()?;
    test_getpeername_socket_pair()?;
    test_send_recv()?;
    test_shutdown_socket_pair()?;

    Ok(())
}
