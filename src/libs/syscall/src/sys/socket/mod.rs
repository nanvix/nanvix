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
    netinet::in_::{
        Ipv4Addr,
        SocketAddrV4,
    },
    sys::{
        socket::socket_address_family::{
            AF_INET,
            AF_UNIX,
        },
        un::SocketAddrUnix,
    },
};
use ::alloc::string::{
    String,
    ToString,
};
use ::core::{
    cmp::min,
    mem,
};
use ::sys::error::{
    Error,
    ErrorCode,
};
use ::sysapi::{
    ffi::c_uchar,
    netinet_in::{
        in_addr,
        sockaddr_in,
    },
    sys_socket::{
        sockaddr,
        socket_address_family,
        socket_shutdown_how::{
            SHUT_RD,
            SHUT_RDWR,
            SHUT_WR,
        },
    },
    sys_un::{
        sockaddr_un,
        SUNPATHLEN,
    },
};
pub use ::sysapi::{
    netinet_in::message_flags::{
        MSG_OOB,
        MSG_PEEK,
    },
    sys_socket::{
        linger,
        sa_family_t,
        sockaddr_storage,
        socket_types::{
            SOCK_DGRAM,
            SOCK_RAW,
            SOCK_SEQPACKET,
            SOCK_STREAM,
        },
        socklen_t,
        sockopt_option_names::{
            SO_BROADCAST,
            SO_ERROR,
            SO_KEEPALIVE,
            SO_LINGER,
            SO_RCVBUF,
            SO_RCVTIMEO,
            SO_REUSEADDR,
            SO_SNDBUF,
            SO_SNDTIMEO,
            SO_TYPE,
        },
        SOL_SOCKET,
    },
};

cfg_if::cfg_if! {
    if #[cfg(feature = "syscall")] {
        pub use self::bindings::socketpair::socketpair;
    }
}

//==================================================================================================
// Modules
//==================================================================================================

pub mod family;
pub mod message;

cfg_if::cfg_if! {
    if #[cfg(feature = "syscall")] {
        pub mod syscall;
        pub mod bindings;
    }
}

//==================================================================================================

/// Describes protocol family of a socket.
#[repr(i32)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AddressFamily {
    /// Internet domain sockets for use with IPv4 addresses.
    Inet = socket_address_family::AF_INET,
    /// Internet domain sockets for use with IPv6 addresses.
    Inet6 = socket_address_family::AF_INET6,
    /// Unix domain sockets.
    Unix = socket_address_family::AF_UNIX,
    /// Unspecified.
    Unspec = socket_address_family::AF_UNSPEC,
}

impl From<AddressFamily> for i32 {
    fn from(family: AddressFamily) -> i32 {
        family as i32
    }
}

impl TryFrom<i32> for AddressFamily {
    type Error = Error;

    fn try_from(family: i32) -> Result<Self, Self::Error> {
        match family {
            socket_address_family::AF_INET => Ok(AddressFamily::Inet),
            socket_address_family::AF_INET6 => Ok(AddressFamily::Inet6),
            socket_address_family::AF_UNIX => Ok(AddressFamily::Unix),
            socket_address_family::AF_UNSPEC => Ok(AddressFamily::Unspec),
            _unsupported_family => Err(Error::new(
                ErrorCode::AddressFamilyNotSupported,
                "socket address family not supported",
            )),
        }
    }
}

/// Describes communication semantics of a socket.
#[repr(i32)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SocketType {
    /// Provides sequenced, reliable, bidirectional, connection-mode byte streams.
    Stream = self::SOCK_STREAM,
    /// Provides raw network protocol access.
    Raw = self::SOCK_RAW,
    /// Provides datagrams, which are connectionless-mode, unreliable messages of fixed maximum length.
    Datagram = self::SOCK_DGRAM,
    /// Provides sequenced, reliable, bidirectional, connection-mode transmission paths for records.
    SeqPacket = self::SOCK_SEQPACKET,
}

impl TryFrom<i32> for SocketType {
    type Error = Error;

    fn try_from(socket_type: i32) -> Result<Self, Self::Error> {
        match socket_type {
            x if x == self::SOCK_STREAM => Ok(SocketType::Stream),
            x if x == self::SOCK_RAW => Ok(SocketType::Raw),
            x if x == self::SOCK_DGRAM => Ok(SocketType::Datagram),
            x if x == self::SOCK_SEQPACKET => Ok(SocketType::SeqPacket),
            _unsupported_socket_type => {
                Err(Error::new(ErrorCode::BadProtocolType, "socket type not supported"))
            },
        }
    }
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
                Err(Error::new(ErrorCode::OperationNotSupportedOnSocket, reason))
            },
        }
    }
}

impl TryFrom<&SocketAddrV4> for sockaddr_in {
    type Error = Error;

    fn try_from(addr: &SocketAddrV4) -> Result<Self, Self::Error> {
        Ok(Self {
            sin_len: mem::size_of::<sockaddr_in>() as u8,
            sin_family: socket_address_family::AF_INET.try_into().map_err(|_| {
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

impl From<SocketAddrV4> for sockaddr {
    fn from(sockaddr: SocketAddrV4) -> Self {
        Self::from(&sockaddr)
    }
}

impl From<&SocketAddrV4> for sockaddr {
    fn from(sockaddr: &SocketAddrV4) -> Self {
        let mut sa_data: [u8; 14] = [0u8; 14];
        sa_data[0..2].copy_from_slice(&sockaddr.port().to_be_bytes());
        sa_data[2..6].copy_from_slice(&sockaddr.addr().octets());
        sockaddr {
            sa_len: mem::size_of::<sockaddr_in>() as u8,
            sa_family: AF_INET as u8,
            sa_data,
        }
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
        let mut sun_path: [u8; SUNPATHLEN] = [0u8; SUNPATHLEN];
        let path: &str = addr.path();
        let path: &[u8] = path.as_bytes();
        if path.len() > sun_path.len() {
            return Err(Error::new(ErrorCode::NameTooLong, "path is too long"));
        }
        sun_path[0..path.len()].copy_from_slice(path);
        Ok(Self {
            sun_len: mem::size_of::<sockaddr_un>() as u8,
            sun_family: socket_address_family::AF_UNIX.try_into().map_err(|_| {
                Error::new(ErrorCode::ValueOutOfRange, "failed to convert socket address family")
            })?,
            sun_path: sun_path.map(|b| b as i8),
        })
    }
}

impl From<SocketAddrUnix> for sockaddr {
    fn from(sockaddr: SocketAddrUnix) -> Self {
        Self::from(&sockaddr)
    }
}

impl From<&SocketAddrUnix> for sockaddr {
    fn from(sockaddr: &SocketAddrUnix) -> Self {
        let mut sa_data: [u8; 14] = [0u8; 14];
        let path: &str = sockaddr.path();
        let path: &[u8] = path.as_bytes();
        sa_data[0..min(path.len(), 14)].copy_from_slice(path);
        sockaddr {
            sa_len: mem::size_of::<sockaddr>() as c_uchar,
            sa_family: AF_UNIX as u8,
            sa_data,
        }
    }
}

impl TryFrom<&sockaddr_un> for SocketAddrUnix {
    type Error = Error;

    fn try_from(addr: &sockaddr_un) -> Result<Self, Self::Error> {
        let path: String = String::from_utf8(addr.sun_path.iter().map(|&b| b as u8).collect())
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
            socket_address_family::AF_INET => Ok(SocketAddr::V4(SocketAddrV4::from(addr))),
            socket_address_family::AF_UNIX => Ok(SocketAddr::Unix(SocketAddrUnix::try_from(addr)?)),
            _unsupported => {
                let reason: &str = "unsupported socket address family";
                Err(Error::new(ErrorCode::AddressFamilyNotSupported, reason))
            },
        }
    }
}

impl From<SocketAddr> for sockaddr {
    fn from(addr: SocketAddr) -> Self {
        (&addr).into()
    }
}

impl From<&SocketAddr> for sockaddr {
    fn from(addr: &SocketAddr) -> Self {
        match addr {
            SocketAddr::V4(addr) => sockaddr::from(addr),
            SocketAddr::Unix(addr) => sockaddr::from(addr),
        }
    }
}

impl From<SocketAddr> for (sockaddr, self::socklen_t) {
    fn from(addr: SocketAddr) -> (sockaddr, self::socklen_t) {
        (&addr).into()
    }
}

impl From<&SocketAddr> for (sockaddr, self::socklen_t) {
    fn from(addr: &SocketAddr) -> (sockaddr, self::socklen_t) {
        match addr {
            SocketAddr::V4(sockaddr) => {
                (sockaddr::from(sockaddr), mem::size_of::<sockaddr>() as self::socklen_t)
            },
            SocketAddr::Unix(sockaddr) => {
                (sockaddr::from(sockaddr), mem::size_of::<sockaddr>() as self::socklen_t)
            },
        }
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
            .expect("conversion from socket address should succeed");
        assert_eq!(expected_addr, SocketAddrV4::from(&test_addr));
    }

    /// Tets conversion from `sockaddr_in` to `SocketAddrV4`.
    #[test]
    fn test_ipv4_sockaddr_conversion() {
        let test_addr: sockaddr_in = sockaddr_in {
            sin_len: mem::size_of::<sockaddr_in>() as u8,
            sin_family: socket_address_family::AF_INET
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
            sa_len: mem::size_of::<sockaddr>() as u8,
            sa_family: socket_address_family::AF_INET
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
        let test_addr: sockaddr_un = sockaddr_un {
            sun_len: mem::size_of::<sockaddr_un>() as u8,
            sun_family: socket_address_family::AF_UNIX
                .try_into()
                .expect("converting address family should succeed"),
            sun_path: {
                let mut path: [u8; SUNPATHLEN] = [0; SUNPATHLEN];
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
        let test_addr: sockaddr = sockaddr {
            sa_len: mem::size_of::<sockaddr>() as u8,
            sa_family: socket_address_family::AF_UNIX as u8,
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
