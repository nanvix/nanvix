// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use ::sys::error::Error;
use ::syscall::{
    netinet::in_::{
        Ipv4Addr,
        Protocol,
        SocketAddrV4,
    },
    sys::socket::{
        AddressFamily,
        SocketAddr,
        SocketType,
        syscall::{
            accept,
            bind,
            connect,
            getsockname,
            listen,
            socket,
        },
    },
    unistd::close,
};

//==================================================================================================
// Constants
//==================================================================================================

const LISTEN_BACKLOG: i32 = 5;

//==================================================================================================
// Helper Functions
//==================================================================================================

/// Creates a new IPv4 stream socket.
fn new_unbound_socket() -> Result<i32, Error> {
    socket(AddressFamily::Inet, SocketType::Stream, Protocol::Tcp)
}

/// Creates a new IPv4 stream socket bound to the given address.
fn new_bound_socket(addr: &SocketAddr) -> Result<i32, Error> {
    let sockfd: i32 = new_unbound_socket()?;
    bind(sockfd, addr)?;
    Ok(sockfd)
}

/// Creates a new IPv4 listening socket bound to the given address.
fn new_listening_socket(addr: &SocketAddr) -> Result<i32, Error> {
    let sockfd: i32 = new_bound_socket(addr)?;
    listen(sockfd, LISTEN_BACKLOG)?;
    Ok(sockfd)
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
fn test_bind_socket(addr: &SocketAddr) -> Result<(), Error> {
    let sockfd: i32 = new_bound_socket(addr)?;
    close(sockfd)?;
    Ok(())
}

/// Tests if we succeed to create a listening socket.
fn test_listen_socket(addr: &SocketAddr) -> Result<(), Error> {
    let sockfd: i32 = new_listening_socket(addr)?;
    close(sockfd)?;
    Ok(())
}

/// Tests if we succeed to get the name of a bound socket.
fn test_getsockname_bound_socket(addr: &SocketAddr) -> Result<(), Error> {
    let sockfd: i32 = new_bound_socket(addr)?;

    let mut result_addr: SocketAddr = SocketAddr::V4(SocketAddrV4::default());
    getsockname(sockfd, &mut result_addr)?;

    assert_eq!(*addr, result_addr, "bound socket address mismatch");

    close(sockfd)?;
    Ok(())
}

/// Tests if we succeed to get the name of a listening socket.
fn test_getsockname_listening_socket(addr: &SocketAddr) -> Result<(), Error> {
    let sockfd: i32 = new_listening_socket(addr)?;

    let mut result_addr: SocketAddr = SocketAddr::V4(SocketAddrV4::default());
    getsockname(sockfd, &mut result_addr)?;

    assert_eq!(*addr, result_addr, "listening socket address mismatch");

    close(sockfd)?;
    Ok(())
}

/// Tests if we succeed to accept a connection on a listening socket.
fn test_accept() -> Result<(), Error> {
    // Create a server socket bound to an ephemeral port.
    let server_addr: SocketAddr =
        SocketAddr::V4(SocketAddrV4::new(Ipv4Addr::new([127, 0, 0, 1]), 0));
    let server_fd: i32 = new_listening_socket(&server_addr)?;

    // Retrieve the actual address assigned by the OS.
    let mut actual_addr: SocketAddr = SocketAddr::V4(SocketAddrV4::default());
    getsockname(server_fd, &mut actual_addr)?;

    // Create a client socket and connect to the server.
    let client_fd: i32 = new_unbound_socket()?;
    connect(client_fd, &actual_addr)?;

    // Accept the incoming connection.
    let (accepted_fd, _peer_addr) = accept(server_fd)?;

    // Clean up.
    close(accepted_fd)?;
    close(client_fd)?;
    close(server_fd)?;
    Ok(())
}

//==================================================================================================
// Entry Point
//==================================================================================================

/// Runs all INET socket tests.
pub fn run() -> Result<(), Error> {
    let addr: SocketAddr = SocketAddr::V4(SocketAddrV4::new(Ipv4Addr::new([127, 0, 0, 1]), 1992));

    test_create_socket()?;
    test_bind_socket(&addr)?;
    test_listen_socket(&addr)?;
    test_getsockname_bound_socket(&addr)?;
    test_getsockname_listening_socket(&addr)?;
    test_accept()?;

    Ok(())
}
