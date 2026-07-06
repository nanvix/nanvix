// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use ::sys::error::Error;
#[cfg(feature = "standalone")]
use ::sysapi::{
    ffi::c_int,
    sys_types::pid_t,
    sys_wait::{
        wexitstatus,
        wifexited,
    },
};
#[cfg(feature = "standalone")]
use ::syscall::unistd::bindings;
use ::syscall::{
    netinet::in_::{
        Ipv4Addr,
        Protocol,
        SocketAddrV4,
    },
    sys::socket::{
        AddressFamily,
        Shutdown,
        SocketAddr,
        SocketType,
        syscall::{
            accept,
            bind,
            connect,
            getsockname,
            listen,
            recv,
            send,
            shutdown,
            socket,
        },
    },
    unistd::close,
};

//==================================================================================================
// Constants
//==================================================================================================

const LISTEN_BACKLOG: i32 = 5;

/// Payload used by the INET send/recv test.
const SEND_RECV_MESSAGE: &[u8] = b"hello";

/// Port used by the process-exit socket reclaim regression test.
#[cfg(feature = "standalone")]
const SOCKET_RECLAIM_PORT: u16 = 1993;

/// Exit status used by a child that completed the socket reclaim setup.
#[cfg(feature = "standalone")]
const CHILD_EXIT_SUCCESS: c_int = 0;

/// Exit status used by a child that failed the socket reclaim setup.
#[cfg(feature = "standalone")]
const CHILD_EXIT_FAILURE: c_int = 101;

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
    if let Err(error) = bind(sockfd, addr) {
        let _ = close(sockfd);
        return Err(error);
    }
    Ok(sockfd)
}

/// Creates a new IPv4 listening socket bound to the given address.
fn new_listening_socket(addr: &SocketAddr) -> Result<i32, Error> {
    let sockfd: i32 = new_bound_socket(addr)?;
    if let Err(error) = listen(sockfd, LISTEN_BACKLOG) {
        let _ = close(sockfd);
        return Err(error);
    }
    Ok(sockfd)
}

/// Creates a loopback TCP connection and returns `(listener, client, accepted)`.
fn new_connected_sockets() -> Result<(i32, i32, i32), Error> {
    let server_addr: SocketAddr =
        SocketAddr::V4(SocketAddrV4::new(Ipv4Addr::new([127, 0, 0, 1]), 0));
    let server_fd: i32 = new_listening_socket(&server_addr)?;

    let mut actual_addr: SocketAddr = SocketAddr::V4(SocketAddrV4::default());
    if let Err(error) = getsockname(server_fd, &mut actual_addr) {
        let _ = close(server_fd);
        return Err(error);
    }

    let client_fd: i32 = match new_unbound_socket() {
        Ok(sockfd) => sockfd,
        Err(error) => {
            let _ = close(server_fd);
            return Err(error);
        },
    };
    if let Err(error) = connect(client_fd, &actual_addr) {
        let _ = close(client_fd);
        let _ = close(server_fd);
        return Err(error);
    }

    let (accepted_fd, _peer_addr) = match accept(server_fd) {
        Ok((accepted_fd, peer_addr)) => (accepted_fd, peer_addr),
        Err(error) => {
            let _ = close(client_fd);
            let _ = close(server_fd);
            return Err(error);
        },
    };
    Ok((server_fd, client_fd, accepted_fd))
}

/// Closes connected sockets and returns the first close error, if any.
fn close_connected_sockets(server_fd: i32, client_fd: i32, accepted_fd: i32) -> Result<(), Error> {
    let accepted_result = close(accepted_fd);
    let client_result = close(client_fd);
    let server_result = close(server_fd);

    accepted_result?;
    client_result?;
    server_result?;
    Ok(())
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

/// Tests that we can accept a connection on a listening socket.
fn test_accept() -> Result<(), Error> {
    let (server_fd, client_fd, accepted_fd) = new_connected_sockets()?;
    close_connected_sockets(server_fd, client_fd, accepted_fd)
}

/// Tests that we can send and receive data through connected INET sockets.
fn test_send_recv() -> Result<(), Error> {
    let (server_fd, client_fd, accepted_fd) = new_connected_sockets()?;

    let mut sent: usize = 0;
    while sent < SEND_RECV_MESSAGE.len() {
        let count: usize = send(client_fd, &SEND_RECV_MESSAGE[sent..], 0)?;
        assert!(count > 0, "send made no progress");
        sent += count;
    }
    shutdown(client_fd, Shutdown::Write)?;

    let mut buf: [u8; SEND_RECV_MESSAGE.len()] = [0u8; SEND_RECV_MESSAGE.len()];
    let mut received: usize = 0;
    while received < SEND_RECV_MESSAGE.len() {
        let count: usize = recv(accepted_fd, &mut buf[received..], 0)?;
        assert!(count > 0, "recv made no progress before completing message");
        received += count;
    }

    assert_eq!(&buf[..], SEND_RECV_MESSAGE, "received data mismatch");

    close_connected_sockets(server_fd, client_fd, accepted_fd)
}

/// Tests that a socket draws its application-visible descriptor from the flat namespace, exactly
/// like a file or pipe: two live sockets hold distinct numbers, and a number freed by `close` is
/// the lowest free one, so the next socket reuses it. This is the unified lowest-free allocation
/// that replaced the former reserved socket range — a socket is no longer numbered apart from every
/// other object.
#[cfg(feature = "standalone")]
fn test_socket_fd_is_flat() -> Result<(), Error> {
    let first: i32 = new_unbound_socket()?;
    let second: i32 = new_unbound_socket()?;
    assert_ne!(first, second, "two live sockets must hold distinct descriptors");
    assert!(second > first, "the second socket takes the next free flat descriptor");

    // Freeing the lower descriptor makes it the lowest free, so the next socket reuses it rather
    // than advancing to a higher number.
    close(first)?;
    let reused: i32 = new_unbound_socket()?;
    assert_eq!(reused, first, "a freed socket descriptor is reused as the lowest free number");

    close(second)?;
    close(reused)?;
    Ok(())
}

/// Tests that `dup2` of a socket shares the underlying `networkd` endpoint and that the endpoint
/// survives until its last descriptor is closed. The duplicate is a second flat descriptor backed
/// by the same socket slot, so it reports the same bound address; closing the original — not the
/// last reference — must leave the duplicate fully usable, and only closing the duplicate drops the
/// last reference that releases the endpoint on `networkd`.
#[cfg(feature = "standalone")]
fn test_dup2_socket_shares_endpoint() -> Result<(), Error> {
    use ::syscall::unistd::dup2;

    // Bind to an ephemeral port so the OS assigns a concrete address to compare against.
    let listen_addr: SocketAddr =
        SocketAddr::V4(SocketAddrV4::new(Ipv4Addr::new([127, 0, 0, 1]), 0));
    let sockfd: i32 = new_listening_socket(&listen_addr)?;

    let mut bound_addr: SocketAddr = SocketAddr::V4(SocketAddrV4::default());
    getsockname(sockfd, &mut bound_addr)?;

    // Obtain a descriptor number known to be free by allocating one and releasing it, then
    // duplicate the socket onto it. `dup2` is served authoritatively by vfsd, which clones the
    // socket slot, so both descriptors alias the same endpoint.
    let target: i32 = new_unbound_socket()?;
    close(target)?;
    let dupfd: i32 = dup2(sockfd, target)?;
    assert_eq!(dupfd, target, "dup2 returns the requested target descriptor");
    assert_ne!(dupfd, sockfd, "the duplicate is a distinct descriptor");

    let mut dup_addr: SocketAddr = SocketAddr::V4(SocketAddrV4::default());
    getsockname(dupfd, &mut dup_addr)?;
    assert_eq!(bound_addr, dup_addr, "a duplicated socket shares the original endpoint's address");

    // Close the original. It is not the last reference, so the endpoint must stay alive.
    close(sockfd)?;

    // The duplicate still resolves to the live endpoint and reports the same address.
    let mut survivor_addr: SocketAddr = SocketAddr::V4(SocketAddrV4::default());
    getsockname(dupfd, &mut survivor_addr)?;
    assert_eq!(
        bound_addr, survivor_addr,
        "the endpoint survives closing a non-last socket reference"
    );

    // Close the last reference, which releases the endpoint on networkd.
    close(dupfd)?;
    Ok(())
}

/// Creates a duplicated socket endpoint and leaves both descriptors open for process exit.
#[cfg(feature = "standalone")]
fn leave_duplicated_socket_for_exit(addr: &SocketAddr) -> Result<(), Error> {
    use ::syscall::unistd::dup2;

    let sockfd: i32 = new_bound_socket(addr)?;
    let target: i32 = new_unbound_socket()?;
    close(target)?;

    let dupfd: i32 = dup2(sockfd, target)?;
    assert_eq!(dupfd, target, "dup2 returns the requested target descriptor");
    assert_ne!(dupfd, sockfd, "the duplicate is a distinct descriptor");

    Ok(())
}

/// Tests that process exit reclaims a duplicated socket endpoint exactly enough for the address to
/// be immediately reusable by the parent. The child exits with both aliases open; vfsd must treat
/// them as one underlying `networkd` endpoint when forwarding exit-time close requests.
#[cfg(feature = "standalone")]
fn test_process_exit_reclaims_duplicated_socket_endpoint() -> Result<(), Error> {
    let addr: SocketAddr =
        SocketAddr::V4(SocketAddrV4::new(Ipv4Addr::new([127, 0, 0, 1]), SOCKET_RECLAIM_PORT));

    let child_pid: pid_t = bindings::fork::fork();
    if child_pid == 0 {
        let exit_status: c_int = match leave_duplicated_socket_for_exit(&addr) {
            Ok(()) => CHILD_EXIT_SUCCESS,
            Err(_) => CHILD_EXIT_FAILURE,
        };
        unsafe { bindings::_exit::_exit(exit_status) };
    }

    assert!(child_pid > 0, "fork() failed in parent (ret={})", child_pid);

    let mut status: c_int = 0;
    let reaped: pid_t = unsafe { bindings::waitpid::waitpid(child_pid, &raw mut status, 0) };
    assert_eq!(reaped, child_pid, "waitpid() must reap the child that held duplicated sockets");
    assert!(wifexited(status), "child must exit normally (status={})", status);
    assert_eq!(
        wexitstatus(status),
        CHILD_EXIT_SUCCESS,
        "child must complete duplicated socket setup before exit"
    );

    let sockfd: i32 = new_bound_socket(&addr)?;
    close(sockfd)?;
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
    test_send_recv()?;

    // Flat-namespace socket behavior is implemented only for the standalone (networkd) backend.
    #[cfg(feature = "standalone")]
    {
        test_socket_fd_is_flat()?;
        test_dup2_socket_shares_endpoint()?;
        test_process_exit_reclaims_duplicated_socket_endpoint()?;
    }

    Ok(())
}
