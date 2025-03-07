// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Configuration
//==================================================================================================

#![allow(non_camel_case_types)]

//==================================================================================================
// Imports
//==================================================================================================

use crate::sys::socket::sa_family_t;
use ::alloc::vec::Vec;
use ::core::{
    mem,
    str::FromStr,
};
use ::nvx::sys::error::{
    Error,
    ErrorCode,
};

//==================================================================================================
// C Interface
//==================================================================================================

/// C Bindings for `netinet/in.h`.
pub mod bindings {
    use super::*;

    /// Used for internet ports.
    pub type in_port_t = u16;

    /// Used for internet addresses.
    pub type in_addr_t = u32;

    /// Describes an internet address.
    #[repr(C, packed)]
    pub struct in_addr {
        pub s_addr: in_addr_t,
    }
    ::nvx::sys::static_assert_size!(in_addr, in_addr::_SIZE);

    impl in_addr {
        /// Size of this structure, used for static assertions.
        const _SIZE: usize = mem::size_of::<in_addr_t>(); // s_addr
    }

    /// Describes an internet socket address.
    #[repr(C, packed)]
    pub struct sockaddr_in {
        /// Socket address length.
        pub sin_len: u8,
        /// Address family.
        pub sin_family: sa_family_t,
        /// Port number.
        pub sin_port: in_port_t,
        /// Internet address.
        pub sin_addr: in_addr,
        /// Padding.
        pub sin_zero: [u8; 8],
    }
    ::nvx::sys::static_assert_size!(sockaddr_in, sockaddr_in::_SIZE);

    impl sockaddr_in {
        /// Size of this structure, used for static assertions.
        const _SIZE: usize = mem::size_of::<u8>() + // sin_len
                        mem::size_of::<sa_family_t>() + // sin_family
                        mem::size_of::<in_port_t>() + // sin_port
                        mem::size_of::<in_addr>() + // sin_addr
                        mem::size_of::<[u8; 8]>(); // sin_zero
    }
}

//==================================================================================================
// Rust Interface
//==================================================================================================

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
