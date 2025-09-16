// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Configuration
//==================================================================================================

#![allow(non_camel_case_types)]

//==================================================================================================
// Imports
//==================================================================================================

use crate::sys_socket::{
    sa_family_t,
    sockaddr_storage,
};
use ::core::mem::size_of;

//==================================================================================================
// Types
//==================================================================================================

/// Used for internet addresses.
pub type in_addr_t = u32;

/// Used for internet ports.
pub type in_port_t = u16;

//==================================================================================================
// Constants
//==================================================================================================

/// Socket option levels to be used with the `setsockopt()` and `getsockopt()` functions.
pub mod sockopt_levels {
    use crate::ffi::c_int;

    /// Unspecified IP protocol.
    pub const IPPROTO_IP: c_int = 0;
    /// Control message protocol.
    pub const IPPROTO_ICMP: c_int = 1;
    /// Transmission Control Protocol.
    pub const IPPROTO_TCP: c_int = 6;
    /// User Datagram Protocol.
    pub const IPPROTO_UDP: c_int = 17;
    /// Internet Protocol version 6.
    pub const IPPROTO_IPV6: c_int = 41;
    /// Raw IP Packet Protocol.
    pub const IPPROTO_RAW: c_int = 255;
}

/// IP option names to be used with `setsockopt()` and `getsockopt()`.
pub mod ip_option_names {
    use crate::ffi::c_int;

    /// buf/ip_opts; set/get IP options
    pub const IP_OPTIONS: c_int = 1;
    /// int; header is included with data
    pub const IP_HDRINCL: c_int = 2;
    /// int; IP type of service and precede.
    pub const IP_TOS: c_int = 3;
    /// int; IP time to live
    pub const IP_TTL: c_int = 4;
    /// bool; receive all IP opts w/dgram
    pub const IP_RECVOPTS: c_int = 5;
    /// bool; receive IP opts for response
    pub const IP_RECVRETOPTS: c_int = 6;
    /// bool; receive IP dst addr w/dgram
    pub const IP_RECVDSTADDR: c_int = 7;
    /// cmsg_type to set src addr
    pub const IP_SENDSRCADDR: c_int = IP_RECVDSTADDR;
    /// ip_opts; set/get IP options
    pub const IP_RETOPTS: c_int = 8;
    /// struct in_addr *or* struct ip_mreqn; set/get IP multicast i/f
    pub const IP_MULTICAST_IF: c_int = 9;
    /// u_char; set/get IP multicast ttl
    pub const IP_MULTICAST_TTL: c_int = 10;
    /// u_char; set/get IP multicast loopback
    pub const IP_MULTICAST_LOOP: c_int = 11;
    /// ip_mreq; add an IP group membership
    pub const IP_ADD_MEMBERSHIP: c_int = 12;
    /// ip_mreq; drop an IP group membership
    pub const IP_DROP_MEMBERSHIP: c_int = 13;
    /// set/get IP mcast virt. iface
    pub const IP_MULTICAST_VIF: c_int = 14;
    /// enable RSVP in kernel
    pub const IP_RSVP_ON: c_int = 15;
    /// disable RSVP in kernel
    pub const IP_RSVP_OFF: c_int = 16;
    /// set RSVP per-vif socket
    pub const IP_RSVP_VIF_ON: c_int = 17;
    /// unset RSVP per-vif socket
    pub const IP_RSVP_VIF_OFF: c_int = 18;
    /// int; range to choose for unspec port
    pub const IP_PORTRANGE: c_int = 19;
    /// bool; receive reception if w/dgram
    pub const IP_RECVIF: c_int = 20;
    /// int; set/get security policy
    pub const IP_IPSEC_POLICY: c_int = 21;
    /// bool: send all-ones broadcast
    pub const IP_ONESBCAST: c_int = 23;
    /// bool: allow bind to any address
    pub const IP_BINDANY: c_int = 24;
    /// bool: allow multiple listeners on a tuple
    pub const IP_BINDMULTI: c_int = 25;
    /// int; set RSS listen bucket
    pub const IP_RSS_LISTEN_BUCKET: c_int = 26;
    /// bool: receive IP dst addr/port w/dgram
    pub const IP_ORIGDSTADDR: c_int = 27;
    /// bool: receive IP dst addr/port w/dgram
    pub const IP_RECVORIGDSTADDR: c_int = IP_ORIGDSTADDR;
}

/// IPv6 socket option names for use with `getsockopt()` and `setsockopt()` at the IPPROTO_IPV6
/// level.
pub mod sockopt_ipv6 {
    use crate::ffi::c_int;

    /// bool: receive hop limit w/dgram
    pub const IPV6_RECVHOPLIMIT: c_int = 37;
}

/// Socket creation flags to be used with `socket()`, `socketpair()`, and `accept4()`.
pub mod socket_flags {
    use crate::ffi::c_int;

    /// Creates a socket file descriptor with the FD_CLOEXEC flag set.
    pub const SOCK_CLOEXEC: c_int = 0x10000000;
    /// Creates a socket file descriptor with the O_NONBLOCK flag set.
    pub const SOCK_NONBLOCK: c_int = 0x20000000;
    /// Creates a socket file descriptor with the O_CLOFORK flag set.
    pub const SOCK_CLOFORK: c_int = 0x40000000;
}

/// Values for use  for the `msg_flags` field in the `msghdr` structure, or the flags parameter in
/// `recv()`, `recvfrom()`, `recvmsg()`, `send()`, `sendmsg()`, or `sendto()`.
pub mod message_flags {
    use crate::ffi::c_int;

    /// Requests out-of-band data.
    pub const MSG_OOB: c_int = 0x1;
    /// Peeks at an incoming message.
    pub const MSG_PEEK: c_int = 0x2;
    /// Send without using routing tables.
    pub const MSG_DONTROUTE: c_int = 0x4;
    /// Terminates a record.
    pub const MSG_EOR: c_int = 0x8;
    /// Normal data truncated.
    pub const MSG_TRUNC: c_int = 0x10;
    /// Control data truncated.
    pub const MSG_CTRUNC: c_int = 0x20;
    /// Requests to block until the full amount of data can be returned.
    pub const MSG_WAITALL: c_int = 0x40;
    /// Requests not to send SIGPIPE on errors.
    pub const MSG_NOSIGNAL: c_int = 0x20000;
    /// Atomically set the close-on-exec flag for the file descriptor.
    pub const MSG_CMSG_CLOEXEC: c_int = 0x40000;
    /// Atomically set the close-on-fork flag for the file descriptor.
    pub const MSG_CMSG_CLOFORK: c_int = 0x80000;
}

//==================================================================================================
// Structures
//==================================================================================================

/// An internet address for IPv4.
#[derive(Debug, Clone, Copy)]
#[repr(C, packed)]
pub struct in_addr {
    /// Internet address version 4.
    pub s_addr: in_addr_t,
}
::static_assert::assert_eq_size!(in_addr, in_addr::_SIZE);

impl in_addr {
    // Size of this structure, used for static assertions.
    const _SIZE: usize = size_of::<in_addr_t>(); // s_addr
}

/// An internet socket address.
#[repr(C, packed)]
#[derive(Debug, Clone, Copy)]
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
::static_assert::assert_eq_size!(sockaddr_in, sockaddr_in::_SIZE);
::static_assert::assert_eq_size!(sockaddr_in, size_of::<sockaddr_storage>());

impl sockaddr_in {
    /// Size of this structure, used for static assertions.
    const _SIZE: usize = size_of::<u8>() + // sin_len
                        size_of::<sa_family_t>() + // sin_family
                        size_of::<in_port_t>() + // sin_port
                        size_of::<in_addr>() + // sin_addr
                        size_of::<[u8; 8]>(); // sin_zero
}

/// An internet socket address for IPv6.
#[repr(C, packed)]
#[derive(Debug, Clone, Copy)]
pub struct in6_addr {
    /// Internet address version 6.
    pub s6_addr: [u8; 16],
}
::static_assert::assert_eq_size!(in6_addr, in6_addr::_SIZE);

impl in6_addr {
    /// Size of this structure, used for static assertions.
    const _SIZE: usize = size_of::<[u8; 16]>();
}

#[repr(C, packed)]
#[derive(Debug, Clone, Copy)]
pub struct sockaddr_in6 {
    /// Socket address length.
    pub sin6_len: u8,
    /// Address family.
    pub sin6_family: sa_family_t,
    /// Port number.
    pub sin6_port: in_port_t,
    /// IPv6 traffic class and flow information.
    pub sin6_flowinfo: u32,
    /// IPv6 address.
    pub sin6_addr: in6_addr,
    /// Set of interfaces for a scope.
    pub sin6_scope_id: u32,
}
::static_assert::assert_eq_size!(sockaddr_in6, sockaddr_in6::_SIZE);

impl sockaddr_in6 {
    /// Size of this structure, used for static assertions.
    const _SIZE: usize = size_of::<u8>() + // sin6_len
                        size_of::<sa_family_t>() + // sin6_family
                        size_of::<in_port_t>() + // sin6_port
                        size_of::<u32>() + // sin6_flowinfo
                        size_of::<in6_addr>() + // sin6_addr
                        size_of::<u32>(); // sin6_scope_id
}
