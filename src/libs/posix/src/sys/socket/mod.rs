// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Configuration
//==================================================================================================

#![allow(non_camel_case_types)]

//==================================================================================================
// Imports
//==================================================================================================

use crate::{
    ffi::c_uchar,
    netinet::in_::{
        bindings::{
            in_addr,
            sockaddr_in,
        },
        Ipv4Addr,
        SocketAddrV4,
    },
    sys::un::{
        bindings::sockaddr_un,
        SocketAddrUnix,
    },
};
use ::alloc::string::{
    String,
    ToString,
};
use ::core::mem;
use ::num_enum::TryFromPrimitive;
use ::nvx::sys::error::{
    Error,
    ErrorCode,
};

//==================================================================================================
// Modules
//==================================================================================================

pub mod message;

cfg_if::cfg_if! {
    if #[cfg(feature = "syscall")] {
        mod syscall;
        pub use self::syscall::{
            accept,
            bind,
            connect,
            getpeername,
            getsockname,
            listen,
            recv,
            send,
            shutdown,
            socket,
            socketpair,
        };
    }
}

#[cfg(all(feature = "syscall", feature = "staticlib"))]
pub mod bindings;

//==================================================================================================

// Socket address family.
pub mod family {
    /// Unspecified.
    pub const AF_UNSPEC: i32 = 0;
    /// Unix domain sockets.
    pub const AF_UNIX: i32 = 1;
    /// Internet domain sockets for use with IPv4 addresses.
    pub const AF_INET: i32 = 2;
    /// Internet domain sockets for use with IPv6 addresses.
    pub const AF_INET6: i32 = 10;
}

/// Provides sequenced, reliable, bidirectional, connection-mode byte streams.
pub const SOCK_STREAM: i32 = 1;
/// Provides raw network protocol access.
pub const SOCK_RAW: i32 = 3;
/// Provides datagrams, which are connectionless-mode, unreliable messages of fixed maximum length.
pub const SOCK_DGRAM: i32 = 2;
/// Provides sequenced, reliable, bidirectional, connection-mode transmission paths for records.
pub const SOCK_SEQPACKET: i32 = 5;

/// Disables further receive operations.
pub const SHUT_RD: i32 = 0;
/// Disables further send operations.
pub const SHUT_WR: i32 = 1;
/// Disables further send and receive operations.
pub const SHUT_RDWR: i32 = 2;

/// IP Protocol Numbers (https://www.iana.org/assignments/protocol-numbers/protocol-numbers.xhtml).
mod ipproto {
    /// Unspecified IP protocol.
    pub const IPPROTO_IP: i32 = 0;
    /// Transmission Control Protocol.
    pub const IPPROTO_TCP: i32 = 6;
    /// User Datagram Protocol.
    pub const IPPROTO_UDP: i32 = 17;
}

/// Peeks at an incoming message.
pub const MSG_PEEK: i32 = 0x2;
/// Requests out-of-band data.
pub const MSG_OOB: i32 = 0x1;
/// Requests to block until the full amount of data can be returned.
pub const MSG_WAITALL: i32 = 0x100;
/// Terminates a record.
pub const MSG_EOR: i32 = 0x8;
/// Requests not to send SIGPIPE on errors.
pub const MSG_NOSIGNAL: i32 = 0x4000;

/// Used for socket length.
pub type socklen_t = u32;

/// Used for socket address family.
pub type sa_family_t = u8;

/// Describes the address of a socket.
#[derive(Debug, Clone, PartialEq, Eq)]
#[repr(C, packed)]
pub struct sockaddr {
    // Total length.
    pub sa_len: c_uchar,
    /// Address family.
    pub sa_family: sa_family_t,
    /// Address data.
    pub sa_data: [u8; 14],
}
::nvx::sys::static_assert_size!(sockaddr, 16);

/// Describes protocol family of a socket.
#[repr(i32)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, TryFromPrimitive)]
pub enum AddressFamily {
    /// Internet domain sockets for use with IPv4 addresses.
    Inet = family::AF_INET,
    /// Internet domain sockets for use with IPv6 addresses.
    Inet6 = family::AF_INET6,
    /// Unix domain sockets.
    Unix = family::AF_UNIX,
    /// Unspecified.
    Unspec = family::AF_UNSPEC,
}

/// Describes communication protocol of a socket.
#[repr(i32)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, TryFromPrimitive)]
pub enum Protocol {
    /// Internet Protocol.
    Ip = ipproto::IPPROTO_IP,
    /// Transmission Control Protocol.
    Tcp = ipproto::IPPROTO_TCP,
    /// User Datagram Protocol.
    Udp = ipproto::IPPROTO_UDP,
}

/// Describes communication semantics of a socket.
#[repr(i32)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, TryFromPrimitive)]
pub enum SocketType {
    /// Provides sequenced, reliable, bidirectional, connection-mode byte streams.
    Stream = SOCK_STREAM,
    /// Provides raw network protocol access.
    Raw = SOCK_RAW,
    /// Provides datagrams, which are connectionless-mode, unreliable messages of fixed maximum length.
    Datagram = SOCK_DGRAM,
    /// Provides sequenced, reliable, bidirectional, connection-mode transmission paths for records.
    SeqPacket = SOCK_SEQPACKET,
}

/// Describes how a socket should be shutdown.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Shutdown {
    /// Disallows further receive operations.
    Read,
    /// Disallows further send operations.
    Write,
    /// Disallows further send and receive operations.
    ReadWrite,
}

impl TryFrom<i32> for Shutdown {
    type Error = Error;

    fn try_from(how: i32) -> Result<Self, Self::Error> {
        match how {
            SHUT_RD => Ok(Shutdown::Read),
            SHUT_WR => Ok(Shutdown::Write),
            SHUT_RDWR => Ok(Shutdown::ReadWrite),
            _unsupported_how => {
                let reason: &str = "unsupported shutdown operation";
                Err(Error::new(ErrorCode::OperationNotSupported, reason))
            },
        }
    }
}

impl TryFrom<&SocketAddrV4> for sockaddr_in {
    type Error = Error;

    fn try_from(addr: &SocketAddrV4) -> Result<Self, Self::Error> {
        Ok(Self {
            sin_len: mem::size_of::<sockaddr_in>() as u8,
            sin_family: family::AF_INET.try_into().map_err(|_| {
                Error::new(ErrorCode::ValueOutOfRange, "failed to convert socket address family")
            })?,
            sin_port: addr.port().to_be(),
            sin_addr: in_addr {
                s_addr: u32::from_be_bytes(addr.addr().octets()).to_be(),
            },
            sin_zero: [0; 8],
        })
    }
}

impl From<&sockaddr_in> for SocketAddrV4 {
    fn from(addr: &sockaddr_in) -> Self {
        SocketAddrV4::new(
            Ipv4Addr::new(u32::from_be(addr.sin_addr.s_addr).to_be_bytes()),
            u16::from_be(addr.sin_port),
        )
    }
}

impl TryFrom<&SocketAddrV4> for sockaddr {
    type Error = Error;

    fn try_from(addr: &SocketAddrV4) -> Result<Self, Self::Error> {
        let mut sa_data: [u8; 14] = [0u8; 14];
        sa_data[0..2].copy_from_slice(&addr.port().to_be_bytes());
        sa_data[2..6].copy_from_slice(&addr.addr().octets());
        Ok(Self {
            sa_len: mem::size_of::<sockaddr_in>() as c_uchar,
            sa_family: family::AF_INET.try_into().map_err(|_| {
                Error::new(ErrorCode::ValueOutOfRange, "failed to convert socket address family")
            })?,
            sa_data,
        })
    }
}

impl From<&sockaddr> for SocketAddrV4 {
    fn from(addr: &sockaddr) -> Self {
        SocketAddrV4::new(
            Ipv4Addr::new(addr.sa_data[2..6].try_into().unwrap()),
            u16::from_be_bytes([addr.sa_data[0], addr.sa_data[1]]),
        )
    }
}

impl TryFrom<&SocketAddrUnix> for sockaddr_un {
    type Error = Error;

    fn try_from(addr: &SocketAddrUnix) -> Result<Self, Self::Error> {
        let mut sun_path: [u8; 104] = [0u8; 104];
        let path: &str = addr.path();
        let path: &[u8] = path.as_bytes();
        if path.len() > sun_path.len() {
            return Err(Error::new(ErrorCode::NameTooLong, "path is too long"));
        }
        sun_path[0..path.len()].copy_from_slice(path);
        Ok(Self {
            sun_family: family::AF_UNIX.try_into().map_err(|_| {
                Error::new(ErrorCode::ValueOutOfRange, "failed to convert socket address family")
            })?,
            sun_path,
        })
    }
}

impl TryFrom<&SocketAddrUnix> for sockaddr {
    type Error = Error;

    fn try_from(addr: &SocketAddrUnix) -> Result<Self, Self::Error> {
        let mut sa_data: [u8; 14] = [0u8; 14];
        let path: &str = addr.path();
        let path: &[u8] = path.as_bytes();
        if path.len() > sa_data.len() {
            return Err(Error::new(ErrorCode::NameTooLong, "path is too long"));
        }
        sa_data[0..path.len()].copy_from_slice(path);
        Ok(Self {
            sa_len: mem::size_of::<sockaddr>() as c_uchar,
            sa_family: family::AF_UNIX.try_into().map_err(|_| {
                Error::new(ErrorCode::ValueOutOfRange, "failed to convert socket address family")
            })?,
            sa_data,
        })
    }
}

impl TryFrom<&sockaddr_un> for SocketAddrUnix {
    type Error = Error;

    fn try_from(addr: &sockaddr_un) -> Result<Self, Self::Error> {
        let path: String = String::from_utf8(addr.sun_path.to_vec())
            .map_err(|_| {
                Error::new(ErrorCode::InvalidArgument, "failed to convert socket address path")
            })?
            .trim_end_matches('\0')
            .to_string();
        Ok(SocketAddrUnix::new(&path))
    }
}

impl TryFrom<&sockaddr> for SocketAddrUnix {
    type Error = Error;

    fn try_from(addr: &sockaddr) -> Result<Self, Self::Error> {
        let path: String = String::from_utf8(addr.sa_data.to_vec())
            .map_err(|_| {
                Error::new(ErrorCode::InvalidArgument, "failed to convert socket address path")
            })?
            .trim_end_matches('\0')
            .to_string();
        Ok(SocketAddrUnix::new(&path))
    }
}

/// Represents a socket address.
#[repr(C)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SocketAddr {
    /// IPv4 socket address.
    V4(SocketAddrV4),
    /// Unix socket address.
    Unix(SocketAddrUnix),
}

impl TryFrom<&sockaddr_in> for SocketAddr {
    type Error = Error;

    fn try_from(addr: &sockaddr_in) -> Result<Self, Self::Error> {
        Ok(SocketAddr::V4(SocketAddrV4::from(addr)))
    }
}

impl TryFrom<&sockaddr> for SocketAddr {
    type Error = Error;

    fn try_from(addr: &sockaddr) -> Result<Self, Self::Error> {
        match addr.sa_family as i32 {
            family::AF_INET => Ok(SocketAddr::V4(SocketAddrV4::from(addr))),
            family::AF_UNIX => Ok(SocketAddr::Unix(SocketAddrUnix::try_from(addr)?)),
            _unsupported => {
                let reason: &str = "unsupported socket address family";
                Err(Error::new(ErrorCode::AddressFamilyNotSupported, reason))
            },
        }
    }
}

impl TryFrom<&SocketAddr> for sockaddr {
    type Error = Error;

    fn try_from(addr: &SocketAddr) -> Result<Self, Self::Error> {
        match addr {
            SocketAddr::V4(addr) => addr.try_into(),
            SocketAddr::Unix(addr) => addr.try_into(),
        }
    }
}

impl TryFrom<SocketAddr> for (sockaddr, socklen_t) {
    type Error = Error;

    fn try_from(addr: SocketAddr) -> Result<(sockaddr, socklen_t), Self::Error> {
        let len: socklen_t = match addr {
            SocketAddr::V4(_) => mem::size_of::<sockaddr>() as socklen_t,
            SocketAddr::Unix(_) => mem::size_of::<sockaddr>() as socklen_t,
        };
        Ok((sockaddr::try_from(&addr)?, len))
    }
}

#[cfg(test)]
mod test {

    use super::*;

    /// Tests conversion from `SocketAddrV4` to `sockaddr_in`.
    #[test]
    fn test_ipv4_socket_addr_conversion() {
        let expected_addr: SocketAddrV4 = SocketAddrV4::new(Ipv4Addr::new([192, 168, 1, 1]), 80);
        let test_addr: sockaddr_in = sockaddr_in::try_from(&expected_addr)
            .expect("conversion from socket addrress should succeed");
        assert_eq!(expected_addr, SocketAddrV4::from(&test_addr));
    }

    /// Tets conversion from `sockaddr_in` to `SocketAddrV4`.
    #[test]
    fn test_ipv4_sockaddr_conversion() {
        let test_addr: sockaddr_in = sockaddr_in {
            sin_len: mem::size_of::<sockaddr_in>() as u8,
            sin_family: family::AF_INET
                .try_into()
                .expect("converting address family should succeed"),
            sin_port: 80u16.to_be(),
            sin_addr: in_addr {
                s_addr: u32::from_be_bytes([192, 168, 1, 1]).to_be(),
            },
            sin_zero: [0; 8],
        };
        let expected_addr: SocketAddrV4 = SocketAddrV4::new(Ipv4Addr::new([192, 168, 1, 1]), 80);
        assert_eq!(expected_addr, SocketAddrV4::from(&test_addr));
    }

    /// Tests conversion from `SocketAddrV4` to `sockaddr`.
    #[test]
    fn test_ipv4_socket_addr_into_sockaddr() {
        let expected_addr: SocketAddrV4 = SocketAddrV4::new(Ipv4Addr::new([192, 168, 1, 1]), 80);
        let test_addr: sockaddr =
            sockaddr::try_from(&expected_addr).expect("socket address conversion should succeed");
        assert_eq!(expected_addr, SocketAddrV4::from(&test_addr));
    }

    /// Tests conversion from `sockaddr` to `SocketAddrV4`.
    #[test]
    fn test_ipv4_sockaddr_into_socket_addr() {
        let test_addr: sockaddr = sockaddr {
            sa_len: mem::size_of::<sockaddr_in>() as u8,
            sa_family: family::AF_INET
                .try_into()
                .expect("converting address family should succeed"),
            sa_data: [0, 80, 192, 168, 1, 1, 0, 0, 0, 0, 0, 0, 0, 0],
        };
        let expected_addr: SocketAddrV4 = SocketAddrV4::new(Ipv4Addr::new([192, 168, 1, 1]), 80);
        assert_eq!(expected_addr, SocketAddrV4::from(&test_addr));
    }

    /// Tests conversion from `SocketAddr` to `sockaddr`.
    #[test]
    fn test_socket_addr_into_sockaddr() {
        let expected_addr: SocketAddrV4 = SocketAddrV4::new(Ipv4Addr::new([192, 168, 1, 1]), 80);
        let test_addr: sockaddr =
            sockaddr::try_from(&SocketAddr::V4(expected_addr)).expect("conversion should succeed");
        assert_eq!(expected_addr, SocketAddrV4::from(&test_addr));
    }

    /// Tests conversion for `SockAddrUnix` to `sockaddr_un`.
    #[test]
    fn test_unix_socket_addr_conversion() {
        let expected_addr: SocketAddrUnix = SocketAddrUnix::new("/tmp/socket");
        let test_addr: sockaddr_un = sockaddr_un::try_from(&expected_addr)
            .expect("conversion from socket address should succeed");
        assert_eq!(expected_addr, SocketAddrUnix::try_from(&test_addr).unwrap());
    }

    /// Tests conversion from `sockaddr_un` to `SocketAddrUnix`.
    #[test]
    fn test_unix_sockaddr_conversion() {
        let test_addr = sockaddr_un {
            sun_family: family::AF_UNIX
                .try_into()
                .expect("converting address family should succeed"),
            sun_path: {
                let mut path = [0; 104];
                let bytes = "/tmp/socket".as_bytes();
                path[..bytes.len()].copy_from_slice(bytes);
                path
            },
        };
        let expected_addr: SocketAddrUnix = SocketAddrUnix::new("/tmp/socket");
        assert_eq!(expected_addr, SocketAddrUnix::try_from(&test_addr).unwrap());
    }

    /// Tests conversion from `SocketAddrUnix` to `sockaddr`.
    #[test]
    fn test_unix_socket_addr_into_sockaddr() {
        let expected_addr: SocketAddrUnix = SocketAddrUnix::new("/tmp/socket");
        let test_addr: sockaddr =
            sockaddr::try_from(&expected_addr).expect("socket address conversion should succeed");
        assert_eq!(expected_addr, SocketAddrUnix::try_from(&test_addr).unwrap());
    }

    /// Tests conversion from `sockaddr` to `SocketAddrUnix`.
    #[test]
    fn test_unix_sockaddr_into_socket_addr() {
        let test_addr = sockaddr {
            sa_len: mem::size_of::<sockaddr_un>() as u8,
            sa_family: family::AF_UNIX as u8,
            sa_data: {
                let mut data = [0; 14];
                let bytes = "/tmp/socket".as_bytes();
                data[..bytes.len()].copy_from_slice(bytes);
                data
            },
        };
        let expected_addr: SocketAddrUnix = SocketAddrUnix::new("/tmp/socket");
        assert_eq!(expected_addr, SocketAddrUnix::try_from(&test_addr).unwrap());
    }
}
