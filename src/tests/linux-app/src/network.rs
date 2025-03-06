// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.
//==================================================================================================
// Imports
//==================================================================================================

use ::nvx::sys::error::{
    Error,
    ErrorCode,
};
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
        types::ssize_t,
    },
    unistd,
};

//==================================================================================================
// Unbound Socket
//==================================================================================================

struct UnboundSocket {
    sockfd: c_int,
}

impl UnboundSocket {
    pub fn new(domain: AddressFamily, typ: SocketType, protocol: Protocol) -> Result<Self, Error> {
        let sockfd: c_int = sys::socket::socket(domain, typ, protocol)?;

        Ok(UnboundSocket { sockfd })
    }

    pub fn bind(self, sockaddr: &SocketAddr) -> Result<BoundSocket, (UnboundSocket, Error)> {
        match sys::socket::bind(self.sockfd, sockaddr) {
            Ok(()) => Ok(BoundSocket { socket: self }),
            Err(error) => Err((self, error)),
        }
    }
}

impl Drop for UnboundSocket {
    fn drop(&mut self) {
        match unistd::close(self.sockfd) {
            0 => {},
            errno => {
                panic!("failed to close socket with fd {}: {:?}", self.sockfd, errno);
            },
        }
    }
}

//==================================================================================================
// Bound Socket
//==================================================================================================

struct BoundSocket {
    socket: UnboundSocket,
}

impl BoundSocket {
    pub fn getsockname(&self) -> Result<SocketAddr, Error> {
        let mut sockaddr: SocketAddr = SocketAddr::V4(SocketAddrV4::default());
        sys::socket::getsockname(self.socket.sockfd, &mut sockaddr)?;
        Ok(sockaddr)
    }

    pub fn listen(self) -> Result<ListeningSocket, (BoundSocket, Error)> {
        match sys::socket::listen(self.socket.sockfd, 0) {
            Ok(()) => Ok(ListeningSocket { socket: self }),
            Err(error) => Err((self, error)),
        }
    }
}

//==================================================================================================
// Listening Socket
//==================================================================================================

struct ListeningSocket {
    socket: BoundSocket,
}

impl ListeningSocket {
    fn getsockname(&self) -> Result<SocketAddr, Error> {
        self.socket.getsockname()
    }
}

//==================================================================================================
// Connected Socket
//==================================================================================================

struct ConnectedSocket {
    socket: BoundSocket,
}

impl ConnectedSocket {
    fn pair(
        domain: AddressFamily,
        typ: SocketType,
        protocol: Protocol,
    ) -> Result<(Self, Self), Error> {
        let mut socket_fds: [c_int; 2] = [-1; 2];

        match sys::socket::socketpair(domain, typ, protocol, &mut socket_fds) {
            Ok(()) => {},
            Err(errno) => {
                return Err(errno);
            },
        }

        let bound_socket_0: BoundSocket = BoundSocket {
            socket: UnboundSocket {
                sockfd: socket_fds[0],
            },
        };
        let bound_socket_1: BoundSocket = BoundSocket {
            socket: UnboundSocket {
                sockfd: socket_fds[1],
            },
        };

        Ok((
            ConnectedSocket {
                socket: bound_socket_0,
            },
            ConnectedSocket {
                socket: bound_socket_1,
            },
        ))
    }

    fn getsockname(&self) -> Result<SocketAddr, Error> {
        self.socket.getsockname()
    }

    fn getpeername(&self) -> Result<SocketAddr, Error> {
        let mut sockaddr: SocketAddr = SocketAddr::V4(SocketAddrV4::default());
        sys::socket::getpeername(self.socket.socket.sockfd, &mut sockaddr)?;
        Ok(sockaddr)
    }

    fn send(&self, buffer: &[u8], flags: c_int) -> Result<ssize_t, Error> {
        sys::socket::send(self.socket.socket.sockfd, buffer, flags)
    }

    fn recv(&self, buffer: &mut [u8], flags: c_int) -> Result<ssize_t, Error> {
        sys::socket::recv(self.socket.socket.sockfd, buffer, flags)
    }

    fn shutdown(&self, how: Shutdown) -> Result<(), Error> {
        sys::socket::shutdown(self.socket.socket.sockfd, how)
    }
}

//==================================================================================================
// Standalone Functions
//==================================================================================================

fn new_unbound_socket(
    domain: AddressFamily,
    typ: SocketType,
    protocol: Protocol,
) -> Result<UnboundSocket, Error> {
    UnboundSocket::new(domain, typ, protocol)
}

fn new_bound_socket(
    domain: AddressFamily,
    typ: SocketType,
    protocol: Protocol,
    sockaddr: &SocketAddr,
) -> Result<BoundSocket, Error> {
    let unbound_socket: UnboundSocket = new_unbound_socket(domain, typ, protocol)?;
    match unbound_socket.bind(sockaddr) {
        Ok(bound_socket) => Ok(bound_socket),
        Err((_unbound_socket, error)) => Err(error),
    }
}

fn new_listening_socket(
    domain: AddressFamily,
    typ: SocketType,
    protocol: Protocol,
    sockaddr: &SocketAddr,
) -> Result<ListeningSocket, Error> {
    let bound_socket: BoundSocket = new_bound_socket(domain, typ, protocol, sockaddr)?;
    match bound_socket.listen() {
        Ok(listen_socket) => Ok(listen_socket),
        Err((_bound_socket, error)) => Err(error),
    }
}

fn new_socket_pair(
    domain: AddressFamily,
    typ: SocketType,
    protocol: Protocol,
) -> Result<(ConnectedSocket, ConnectedSocket), Error> {
    ConnectedSocket::pair(domain, typ, protocol)
}

fn test_create_socket_pair(
    domain: AddressFamily,
    typ: SocketType,
    protocol: Protocol,
) -> Result<(), Error> {
    ::nvx::info!("test_create_socket_pair");
    let (_socket_0, _socket_1): (ConnectedSocket, ConnectedSocket) =
        new_socket_pair(domain, typ, protocol)?;
    Ok(())
}

fn test_create_socket(
    domain: AddressFamily,
    typ: SocketType,
    protocol: Protocol,
) -> Result<(), Error> {
    ::nvx::info!("test_create_socket");
    let _unbound_socket: UnboundSocket = new_unbound_socket(domain, typ, protocol)?;
    Ok(())
}

fn test_bind_socket(
    domain: AddressFamily,
    typ: SocketType,
    protocol: Protocol,
    sockaddr: &SocketAddr,
) -> Result<(), Error> {
    ::nvx::info!("test_bind_socket");
    let _bound_socket: BoundSocket = new_bound_socket(domain, typ, protocol, sockaddr)?;
    Ok(())
}

fn test_listen_socket(
    domain: AddressFamily,
    typ: SocketType,
    protocol: Protocol,
    sockaddr: &SocketAddr,
) -> Result<(), Error> {
    ::nvx::info!("test_listen_socket");
    let _listen_socket: ListeningSocket = new_listening_socket(domain, typ, protocol, sockaddr)?;
    Ok(())
}

fn test_getsockname_bound_socket(
    domain: AddressFamily,
    typ: SocketType,
    protocol: Protocol,
    sockaddr: &SocketAddr,
) -> Result<(), Error> {
    ::nvx::info!("test_getsockname_bound_socket");
    let bound_socket: BoundSocket = new_bound_socket(domain, typ, protocol, sockaddr)?;
    let sockaddr_: SocketAddr = bound_socket.getsockname()?;
    if sockaddr != &sockaddr_ {
        return Err(Error::new(ErrorCode::RemoteAddressChanged, "remote address changed"));
    }
    Ok(())
}

fn test_getsockname_listening_socket(
    domain: AddressFamily,
    typ: SocketType,
    protocol: Protocol,
    sockaddr: &SocketAddr,
) -> Result<(), Error> {
    ::nvx::info!("test_getsockname_listening_socket");
    let listening_socket: ListeningSocket = new_listening_socket(domain, typ, protocol, sockaddr)?;
    let sockaddr_: SocketAddr = listening_socket.getsockname()?;
    if sockaddr != &sockaddr_ {
        return Err(Error::new(ErrorCode::RemoteAddressChanged, "remote address changed"));
    }
    Ok(())
}

fn test_getsockname(
    domain: AddressFamily,
    typ: SocketType,
    protocol: Protocol,
    sockaddr: &SocketAddr,
) -> Result<(), Error> {
    ::nvx::info!("test_getsockname");
    test_getsockname_bound_socket(domain, typ, protocol, sockaddr)?;
    test_getsockname_listening_socket(domain, typ, protocol, sockaddr)?;
    Ok(())
}

fn test_getpeername(
    domain: AddressFamily,
    typ: SocketType,
    protocol: Protocol,
) -> Result<(), Error> {
    ::nvx::info!("test_getpeername");

    let (socket_0, socket_1): (ConnectedSocket, ConnectedSocket) =
        new_socket_pair(domain, typ, protocol)?;

    let sockaddr_self: [SocketAddr; 2] = [socket_0.getsockname()?, socket_1.getsockname()?];
    let sockaddr_peer: [SocketAddr; 2] = [socket_1.getpeername()?, socket_0.getpeername()?];

    for i in 0..2 {
        if sockaddr_self[i] != sockaddr_peer[i] {
            return Err(Error::new(ErrorCode::RemoteAddressChanged, "remote address changed"));
        }
    }

    Ok(())
}

fn test_send_receive(
    domain: AddressFamily,
    typ: SocketType,
    protocol: Protocol,
) -> Result<(), Error> {
    ::nvx::info!("test_send_receive");

    let (socket_0, socket_1): (ConnectedSocket, ConnectedSocket) =
        new_socket_pair(domain, typ, protocol)?;

    let mut buffer: [u8; 32] = [1; 32];

    // Send message.
    socket_0.send(&buffer, 0)?;

    // Zero out buffer.
    for b in &mut buffer {
        *b = 0;
    }

    // Receive message from connection.
    socket_1.recv(&mut buffer, 0)?;

    // Sanity check message contents.
    for b in &buffer {
        if *b != 1 {
            return Err(Error::new(ErrorCode::InvalidMessage, "message contents are not correct"));
        }
    }

    Ok(())
}

fn test_shutdown(domain: AddressFamily, typ: SocketType, protocol: Protocol) -> Result<(), Error> {
    ::nvx::info!("test_shutdown");

    let (socket_0, socket_1): (ConnectedSocket, ConnectedSocket) =
        new_socket_pair(domain, typ, protocol)?;

    // Disallow send and receive operations.
    for socket in &[socket_0, socket_1] {
        socket.shutdown(Shutdown::ReadWrite)?
    }

    Ok(())
}

pub fn test_network() -> Result<(), Error> {
    ::nvx::info!("test_network");
    let domain: AddressFamily = AddressFamily::Inet;
    let typ: SocketType = SocketType::Stream;
    let protocol: Protocol = Protocol::Tcp;

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

    let sockaddr: SocketAddr = match SocketAddr::try_from(&sockaddr_in) {
        Ok(sockaddr) => sockaddr,
        Err(e) => panic!("{:?}", e),
    };

    test_create_socket(domain, typ, protocol)?;
    test_bind_socket(domain, typ, protocol, &sockaddr)?;
    test_listen_socket(domain, typ, protocol, &sockaddr)?;
    test_getsockname(domain, typ, protocol, &sockaddr)?;

    let domain: AddressFamily = AddressFamily::Unix;
    let typ: SocketType = SocketType::Stream;
    let protocol: Protocol = Protocol::Unspec;

    test_create_socket_pair(domain, typ, protocol)?;
    test_getpeername(domain, typ, protocol)?;
    test_send_receive(domain, typ, protocol)?;
    test_shutdown(domain, typ, protocol)?;

    Ok(())
}
