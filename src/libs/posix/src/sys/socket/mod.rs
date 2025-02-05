// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Configuration
//==================================================================================================

#![allow(non_camel_case_types)]

//==================================================================================================
// Imports
//==================================================================================================

use crate::netinet::in_::{
    in_addr,
    sockaddr_in,
};
use ::core::mem;

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

/// Internet domain sockets for use with IPv4 addresses.
pub const AF_INET: sa_family_t = 2;
/// Internet domain sockets for use with IPv6 addresses.
pub const AF_INET6: sa_family_t = 10;
/// Unix domain sockets.
pub const AF_UNIX: sa_family_t = 1;
/// Unspecified.
pub const AF_UNSPEC: sa_family_t = 0;

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
pub type sa_family_t = u16;

/// Describes the address of a socket.
#[derive(Debug, Clone, PartialEq, Eq)]
#[repr(C, packed)]
pub struct sockaddr {
    /// Address family.
    pub sa_family: sa_family_t,
    /// Address data.
    pub sa_data: [u8; 14],
}
::nvx::sys::static_assert_size!(sockaddr, 16);

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

/// Represents an IPv4 address.
#[repr(C, packed)]
#[derive(Default, Debug, Clone, Copy, PartialEq, Eq)]
pub struct Ipv4Addr {
    /// Address.
    octets: [u8; 4],
}
::nvx::sys::static_assert_size!(Ipv4Addr, 4);

/// Represents an IPv4 socket address.
#[repr(C, packed)]
#[derive(Default, Debug, Clone, Copy, PartialEq, Eq)]
pub struct SocketAddrV4 {
    /// IPv4 address.
    addr: Ipv4Addr,
    /// Port number.
    port: u16,
}
::nvx::sys::static_assert_size!(SocketAddrV4, 6);

impl From<SocketAddrV4> for sockaddr_in {
    fn from(addr: SocketAddrV4) -> Self {
        Self {
            sin_family: AF_INET,
            sin_port: addr.port.to_be(),
            sin_addr: in_addr {
                s_addr: u32::from_be_bytes(addr.addr.octets).to_be(),
            },
            sin_zero: [0; 8],
        }
    }
}

impl From<sockaddr_in> for SocketAddrV4 {
    fn from(addr: sockaddr_in) -> Self {
        Self {
            addr: Ipv4Addr {
                octets: u32::from_be(addr.sin_addr.s_addr).to_be_bytes(),
            },
            port: u16::from_be(addr.sin_port),
        }
    }
}

impl From<SocketAddrV4> for sockaddr {
    fn from(addr: SocketAddrV4) -> Self {
        let mut sa_data: [u8; 14] = [0u8; 14];
        sa_data[0..2].copy_from_slice(&addr.port.to_be_bytes());
        sa_data[2..6].copy_from_slice(&addr.addr.octets);
        Self {
            sa_family: AF_INET,
            sa_data,
        }
    }
}

impl From<sockaddr> for SocketAddrV4 {
    fn from(addr: sockaddr) -> Self {
        let port: u16 = u16::from_be_bytes([addr.sa_data[0], addr.sa_data[1]]);
        let octets: [u8; 4] = addr.sa_data[2..6].try_into().unwrap();
        Self {
            addr: Ipv4Addr { octets },
            port,
        }
    }
}

/// Represents an IPv6 address.
#[repr(C, packed)]
#[derive(Default, Debug, Clone, Copy, PartialEq, Eq)]
pub struct Ipv6Addr {
    /// Address.
    octets: [u8; 16],
}
::nvx::sys::static_assert_size!(Ipv6Addr, 16);

/// Represents an IPv6 socket address.
#[derive(Default, Debug, Clone, Copy, PartialEq, Eq)]
pub struct SocketAddrV6 {
    /// IPv6 address.
    addr: Ipv6Addr,
    /// Port number.
    port: u16,
    /// Flow information.
    flowinfo: u32,
    /// Scope ID.
    scope_id: u32,
}

/// Represents a Unix socket address.
#[repr(C, packed)]
#[derive(Default, Debug, Clone, Copy, PartialEq, Eq)]
pub struct SocketAddrUnix;
::nvx::sys::static_assert_size!(SocketAddrUnix, 0);

/// Represents a socket address.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SocketAddr {
    /// IPv4 socket address.
    V4(SocketAddrV4),
    /// IPv6 socket address.
    V6(SocketAddrV6),
    /// Unix socket address.
    Unix(SocketAddrUnix),
}
::nvx::sys::static_assert_size!(SocketAddr, 32);

impl From<sockaddr> for SocketAddr {
    fn from(addr: sockaddr) -> Self {
        match addr.sa_family {
            AF_INET => SocketAddr::V4(SocketAddrV4::from(addr)),
            AF_INET6 => unimplemented!(),
            AF_UNIX => Self::Unix(SocketAddrUnix),
            _ => unimplemented!(),
        }
    }
}

impl From<SocketAddr> for sockaddr {
    fn from(addr: SocketAddr) -> Self {
        match addr {
            SocketAddr::V4(addr) => addr.into(),
            SocketAddr::V6(_) => unimplemented!(),
            SocketAddr::Unix(_) => sockaddr {
                sa_family: AF_UNIX,
                sa_data: [0; 14],
            },
        }
    }
}

impl From<SocketAddr> for (sockaddr, socklen_t) {
    fn from(addr: SocketAddr) -> (sockaddr, socklen_t) {
        let len: socklen_t = match addr {
            SocketAddr::V4(_) => mem::size_of::<sockaddr_in>() as socklen_t,
            SocketAddr::V6(_) => unimplemented!(),
            SocketAddr::Unix(_) => 0,
        };
        (addr.into(), len)
    }
}

#[cfg(test)]
mod test {

    use super::*;

    /// Tests conversion from `SocketAddrV4` to `sockaddr_in`.
    #[test]
    fn test_ipv4_socket_addr_conversion() {
        let expected_addr: SocketAddrV4 = SocketAddrV4 {
            addr: Ipv4Addr {
                octets: [192, 168, 1, 1],
            },
            port: 80,
        };
        let test_addr: sockaddr_in = expected_addr.into();
        assert_eq!(expected_addr, SocketAddrV4::from(test_addr));
    }

    /// Tets conversion from `sockaddr_in` to `SocketAddrV4`.
    #[test]
    fn test_ipv4_sockaddr_conversion() {
        let test_addr: sockaddr_in = sockaddr_in {
            sin_family: AF_INET,
            sin_port: 80u16.to_be(),
            sin_addr: in_addr {
                s_addr: u32::from_be_bytes([192, 168, 1, 1]).to_be(),
            },
            sin_zero: [0; 8],
        };
        let expected_addr: SocketAddrV4 = SocketAddrV4 {
            addr: Ipv4Addr {
                octets: [192, 168, 1, 1],
            },
            port: 80,
        };
        assert_eq!(expected_addr, SocketAddrV4::from(test_addr));
    }

    /// Tests conversion from `SocketAddrV4` to `sockaddr`.
    #[test]
    fn test_ipv4_socket_addr_into_sockaddr() {
        let expected_addr: SocketAddrV4 = SocketAddrV4 {
            addr: Ipv4Addr {
                octets: [192, 168, 1, 1],
            },
            port: 80,
        };
        let test_addr: sockaddr = expected_addr.into();
        assert_eq!(expected_addr, SocketAddrV4::from(test_addr));
    }

    /// Tests conversion from `sockaddr` to `SocketAddrV4`.
    #[test]
    fn test_ipv4_sockaddr_into_socket_addr() {
        let test_addr: sockaddr = sockaddr {
            sa_family: AF_INET,
            sa_data: [0, 80, 192, 168, 1, 1, 0, 0, 0, 0, 0, 0, 0, 0],
        };
        let expected_addr: SocketAddrV4 = SocketAddrV4 {
            addr: Ipv4Addr {
                octets: [192, 168, 1, 1],
            },
            port: 80,
        };
        assert_eq!(expected_addr, SocketAddrV4::from(test_addr));
    }

    /// Tests conversion from `SocketAddr` to `sockaddr`.
    #[test]
    fn test_socket_addr_into_sockaddr() {
        let expected_addr: SocketAddrV4 = SocketAddrV4 {
            addr: Ipv4Addr {
                octets: [192, 168, 1, 1],
            },
            port: 80,
        };
        let test_addr: sockaddr = SocketAddr::V4(expected_addr).into();
        assert_eq!(expected_addr, SocketAddrV4::from(test_addr));
    }
}
