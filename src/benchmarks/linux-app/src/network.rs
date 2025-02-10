// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.
//==================================================================================================
// Imports
//==================================================================================================

use ::core::mem;
use ::posix::{
    ffi::c_int,
    netinet::in_::{
        in_addr,
        sockaddr_in,
    },
    sys::{
        self,
        socket::{
            AddressFamily,
            Protocol,
            Shutdown,
            SocketAddr,
            SocketAddrV4,
            SocketType,
        },
    },
    unistd,
};

//==================================================================================================
// Standalone Functions
//==================================================================================================

pub fn test() {
    // Create a socket.
    let domain: AddressFamily = AddressFamily::Inet;
    let typ: SocketType = SocketType::Stream;
    let protocol: Protocol = Protocol::Tcp;
    let sockfd: i32 = match sys::socket::socket(domain, typ, protocol) {
        Ok(sockfd) => {
            ::nvx::log!("created socket with fd {}", sockfd);
            sockfd
        },
        Err(error) => {
            panic!("failed to create socket: {:?}", error);
        },
    };

    // Bind socket to address to 127.0.0.1:8888.
    let sockaddr_in: sockaddr_in = sockaddr_in {
        sin_family: match sys::socket::AF_INET.try_into() {
            Ok(family) => family,
            Err(e) => panic!("{:?}", e),
        },
        sin_port: u16::to_be(8888),
        sin_addr: in_addr {
            s_addr: u32::from_be_bytes([127, 0, 0, 1]).to_be(),
        },
        sin_zero: [0; 8],
    };

    // TODO: test case for connect().

    // TODO: test case for accept().

    let sockaddr: SocketAddr = SocketAddr::V4(sockaddr_in.into());

    match sys::socket::bind(sockfd, &sockaddr) {
        Ok(()) => {
            ::nvx::log!("bound socket to address");
        },
        Err(error) => {
            panic!("failed to bind socket to address (error={:?})", error);
        },
    }

    // Check if socket is bound to expected address.
    let mut sockaddr_: SocketAddr = SocketAddr::V4(SocketAddrV4::default());
    match sys::socket::getsockname(sockfd, &mut sockaddr_) {
        Ok(()) => {
            if sockaddr_ != sockaddr {
                panic!(
                    "socket is not bound to expected address (expected: {:?}, actual: {:?})",
                    sockaddr, sockaddr_
                );
            }
            ::nvx::log!("socket is bound to address {:?}", sockaddr_);
        },
        Err(error) => {
            panic!("failed to get local name of socket: {:?}", error);
        },
    }

    // Listen for connections on socket.
    match sys::socket::listen(sockfd, 0) {
        Ok(()) => {
            ::nvx::log!("listening for connections on socket");
        },
        Err(error) => {
            panic!("failed to listen for connections on socket ({:?})", error);
        },
    }

    // Close socket.
    match unistd::close(sockfd) {
        0 => {
            ::nvx::log!("closed socket");
        },
        errno => {
            panic!("failed to close socket: {:?}", errno);
        },
    }

    // Create a pair of connected sockets.
    let mut socket_fds: [c_int; 2] = [-1; 2];

    match sys::socket::socketpair(
        AddressFamily::Unix,
        SocketType::Stream,
        Protocol::Unspec,
        &mut socket_fds,
    ) {
        Ok(()) => {
            ::nvx::log!(
                "created pair of connected sockets with fds {} and {}",
                socket_fds[0],
                socket_fds[1]
            );
        },
        Err(errno) => {
            panic!("failed to create pair of connected sockets: {:?}", errno);
        },
    }

    // Get name of the local socket.
    let mut sockaddr_self: [SocketAddr; 2] = unsafe { mem::zeroed() };
    for i in 0..2 {
        match sys::socket::getsockname(socket_fds[i], &mut sockaddr_self[i]) {
            Ok(()) => {
                ::nvx::log!("sockfd {:?} is bound to {:?}", socket_fds[i], sockaddr_self[i]);
            },
            errno => {
                panic!("failed to get local name of connection: {:?}", errno);
            },
        }
    }

    // Get name of the peer socket.
    let mut sockaddr_peer: [SocketAddr; 2] = unsafe { mem::zeroed() };
    for i in (0..2).rev() {
        match sys::socket::getpeername(socket_fds[i], &mut sockaddr_peer[i]) {
            Ok(()) => {
                ::nvx::log!(
                    "sockfd {:?} is connected to peer {:?}",
                    socket_fds[i],
                    sockaddr_peer[i]
                );
            },
            errno => {
                panic!("failed to get peer name of connection: {:?}", errno);
            },
        }
    }

    // Check if local and peer names are the same.
    for i in 0..2 {
        if sockaddr_self[i] != sockaddr_peer[i] {
            panic!("local and peer names are not the same");
        }
    }

    let mut buffer: [u8; 32] = [1; 32];

    // Send message.
    match sys::socket::send(socket_fds[0], &buffer, 0) {
        Ok(len) => {
            ::nvx::log!("sent {} bytes to connection", len);
        },
        Err(error) => {
            panic!("failed to send message to connection (error={:?})", error);
        },
    }

    // Receive message from connection.
    match sys::socket::recv(socket_fds[1], &mut buffer, 0) {
        Ok(len) => {
            ::nvx::log!("received {} bytes from connection", len);
        },
        Err(error) => {
            panic!("failed to receive message from connection (error={:?})", error);
        },
    }

    // Sanity check message contents.
    (0..32).for_each(|i| {
        if buffer[i] != 1 {
            panic!("message contents are not correct");
        }
    });

    // Disallow send and receive operations.
    for socketfd in &socket_fds {
        match sys::socket::shutdown(*socketfd, Shutdown::ReadWrite) {
            Ok(()) => {
                ::nvx::log!("disallowed send and receive operations on connection");
            },
            Err(error) => {
                panic!("failed to disallow send and receive operations on connection: {:?}", error);
            },
        }
    }

    // Close sockets.
    match unistd::close(socket_fds[0]) {
        0 => {
            ::nvx::log!("closed socket with fd {}", socket_fds[0]);
        },
        errno => {
            panic!("failed to close socket with fd {}: {:?}", socket_fds[0], errno);
        },
    }

    match unistd::close(socket_fds[1]) {
        0 => {
            ::nvx::log!("closed socket with fd {}", socket_fds[1]);
        },
        errno => {
            panic!("failed to close socket with fd {}: {:?}", socket_fds[1], errno);
        },
    }
}
