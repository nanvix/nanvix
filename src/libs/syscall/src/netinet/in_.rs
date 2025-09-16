// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Imports
//==================================================================================================

use ::alloc::vec::Vec;
use ::core::str::FromStr;
use ::sys::error::{
    Error,
    ErrorCode,
};
use ::sysapi::netinet_in::sockopt_levels::{
    IPPROTO_IP,
    IPPROTO_TCP,
    IPPROTO_UDP,
};

//==================================================================================================
// Modules
//==================================================================================================

pub mod bindings;

//==================================================================================================
// Rust Interface
//==================================================================================================

/// Describes communication protocol of a socket.
#[repr(i32)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Protocol {
    /// Internet Protocol.
    Ip = IPPROTO_IP,
    /// Transmission Control Protocol.
    Tcp = IPPROTO_TCP,
    /// User Datagram Protocol.
    Udp = IPPROTO_UDP,
}

impl TryFrom<i32> for Protocol {
    type Error = Error;

    fn try_from(proto: i32) -> Result<Self, Self::Error> {
        match proto {
            IPPROTO_IP => Ok(Protocol::Ip),
            IPPROTO_TCP => Ok(Protocol::Tcp),
            IPPROTO_UDP => Ok(Protocol::Udp),
            _ => Err(Error::new(ErrorCode::ProtocolNotSupported, "protocol not supported")),
        }
    }
}

/// Represents an IPv4 address.
#[derive(Default, Debug, Clone, Copy, PartialEq, Eq)]
pub struct Ipv4Addr {
    /// Address.
    octets: [u8; 4],
}

impl Ipv4Addr {
    /// Creates a new IPv4 address.
    pub fn new(octets: [u8; 4]) -> Self {
        Ipv4Addr { octets }
    }

    /// Returns the octets of the target IPv4 address.
    pub fn octets(&self) -> [u8; 4] {
        self.octets
    }
}

/// Represents an IPv4 socket address.
#[derive(Default, Debug, Clone, Copy, PartialEq, Eq)]
pub struct SocketAddrV4 {
    /// IPv4 address.
    addr: Ipv4Addr,
    /// Port number.
    port: u16,
}

impl SocketAddrV4 {
    /// Creates a new IPv4 socket address.
    pub fn new(addr: Ipv4Addr, port: u16) -> Self {
        SocketAddrV4 { addr, port }
    }

    /// Returns the IP address of the target IPv4 socket address.
    pub fn addr(&self) -> Ipv4Addr {
        self.addr
    }

    /// Returns the port number of the target IPv4 socket address.
    pub fn port(&self) -> u16 {
        self.port
    }
}

impl FromStr for SocketAddrV4 {
    type Err = Error;

    fn from_str(sockaddr: &str) -> Result<Self, Self::Err> {
        let mut parts = sockaddr.split(':');
        let addr: &str = match parts.next() {
            Some(addr) => addr,
            None => return Err(Error::new(ErrorCode::InvalidArgument, "invalid socket address")),
        };
        let port: &str = match parts.next() {
            Some(port) => port,
            None => return Err(Error::new(ErrorCode::InvalidArgument, "invalid socket address")),
        };
        let port: u16 = match port.parse::<u16>() {
            Ok(port) => port,
            Err(_) => return Err(Error::new(ErrorCode::InvalidArgument, "invalid port number")),
        };
        let octets: Vec<u8> = match addr.split('.').map(|octet| octet.parse::<u8>()).collect() {
            Ok(octets) => octets,
            Err(_) => return Err(Error::new(ErrorCode::InvalidArgument, "invalid ipv4 address")),
        };
        let octets: [u8; 4] = match octets.try_into() {
            Ok(octets) => octets,
            Err(_) => return Err(Error::new(ErrorCode::InvalidArgument, "invalid ipv4 address")),
        };
        Ok(SocketAddrV4 {
            addr: Ipv4Addr { octets },
            port,
        })
    }
}
