// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Configuration
//==================================================================================================

#![allow(non_camel_case_types)]

//==================================================================================================
// Imports
//==================================================================================================

use crate::ffi::{
    c_int,
    c_uchar,
    c_void,
};
use ::core::{
    fmt,
    mem::size_of,
};

#[cfg(target_pointer_width = "32")]
use crate::sys_types::{
    msghdr,
    size_t,
    ssize_t,
};

//==================================================================================================
// Types
//==================================================================================================

/// Used for socket address family.
pub type sa_family_t = u8;

/// Used for socket length.
pub type socklen_t = u32;

//==================================================================================================
// Constants
//==================================================================================================

/// Socket address family.
pub mod socket_address_family {
    use crate::ffi::c_int;

    /// Unspecified.
    pub const AF_UNSPEC: c_int = 0;
    /// Unix domain sockets.
    pub const AF_UNIX: c_int = 1;
    /// Internet domain sockets for use with IPv4 addresses.
    pub const AF_INET: c_int = 2;
    /// Internet domain sockets for use with IPv6 addresses.
    pub const AF_INET6: c_int = 10;
}

/// Socket option names to be used with `setsockopt()` and `getsockopt()`.
pub mod sockopt_option_names {
    use crate::ffi::c_int;

    /// Debugging information is being recorded.
    pub const SO_DEBUG: c_int = 0x0001;
    /// Socket is accepting connections.
    pub const SO_ACCEPTCONN: c_int = 0x0002;
    /// Reuse of local addresses is supported.
    pub const SO_REUSEADDR: c_int = 0x0004;
    /// Connections are kept alive with periodic messages.
    pub const SO_KEEPALIVE: c_int = 0x0008;
    /// Bypass normal routing.
    pub const SO_DONTROUTE: c_int = 0x0010;
    /// Transmission of broadcast messages is supported.
    pub const SO_BROADCAST: c_int = 0x0020;
    /// Socket lingers on close.
    pub const SO_LINGER: c_int = 0x0080;
    /// Out-of-band data is transmitted in line.
    pub const SO_OOBINLINE: c_int = 0x0100;
    /// Send buffer size.
    pub const SO_SNDBUF: c_int = 0x1001;
    /// Receive buffer size.
    pub const SO_RCVBUF: c_int = 0x1002;
    /// Send "low water mark".
    pub const SO_SNDLOWAT: c_int = 0x1003;
    /// Receive "low water mark".
    pub const SO_RCVLOWAT: c_int = 0x1004;
    /// Send timeout.
    pub const SO_SNDTIMEO: c_int = 0x1005;
    /// Receive timeout.
    pub const SO_RCVTIMEO: c_int = 0x1006;
    /// Socket error status.
    pub const SO_ERROR: c_int = 0x1007;
    /// Socket type.
    pub const SO_TYPE: c_int = 0x1008;
    /// Socket protocol.
    pub const SO_PROTOCOL: c_int = 0x1016;
    /// Socket domain.
    pub const SO_DOMAIN: c_int = 0x1019;
}

/// Socket types.
pub mod socket_types {
    use crate::ffi::c_int;

    /// Provides sequenced, reliable, bidirectional, connection-mode byte streams.
    pub const SOCK_STREAM: c_int = 1;
    /// Provides raw network protocol access.
    pub const SOCK_RAW: c_int = 3;
    /// Provides datagrams, which are connectionless-mode, unreliable messages of fixed maximum length.
    pub const SOCK_DGRAM: c_int = 2;
    /// Provides sequenced, reliable, bidirectional, connection-mode transmission paths for records.
    pub const SOCK_SEQPACKET: c_int = 5;
}

/// Socket shutdown values to be used with `shutdown()`.
pub mod socket_shutdown_how {
    use crate::ffi::c_int;

    /// Disables further receive operations.
    pub const SHUT_RD: c_int = 0;
    /// Disables further send operations.
    pub const SHUT_WR: c_int = 1;
    /// Disables further send and receive operations.
    pub const SHUT_RDWR: c_int = 2;
}

/// Maximum number of connections that can be queued for acceptance.
pub const SOMAXCONN: c_int = 128;

/// Options for socket level.
pub const SOL_SOCKET: c_int = 0xffff;

//==================================================================================================
// Structures
//==================================================================================================

/// A structure large enough to hold any socket address.
#[derive(Debug, Clone, PartialEq, Eq)]
#[repr(C, packed)]
pub struct sockaddr_storage {
    /// Total length.
    pub ss_len: c_uchar,
    /// Address family.
    pub ss_family: sa_family_t,
    /// Address data.
    pub ss_data: [u8; 14],
}
::static_assert::assert_eq_size!(sockaddr_storage, sockaddr_storage::_SIZE);

impl sockaddr_storage {
    /// Size of this structure, used in static assertions.
    const _SIZE: usize = size_of::<c_uchar>() + // ss_len
        size_of::<sa_family_t>() + // ss_family
        size_of::<[u8; 14]>(); // ss_data
}

/// Describes the address of a socket.
#[derive(Copy, Clone, PartialEq, Eq)]
#[repr(C, packed)]
pub struct sockaddr {
    /// Total length.
    pub sa_len: c_uchar,
    /// Address family.
    pub sa_family: sa_family_t,
    /// Address data.
    pub sa_data: [u8; 14],
}
::static_assert::assert_eq_size!(sockaddr, sockaddr::_SIZE);
::static_assert::assert_eq_size!(sockaddr, size_of::<sockaddr_storage>());

impl sockaddr {
    /// Size of this structure, used in static assertions.
    const _SIZE: usize = size_of::<c_uchar>() + // sa_len
        size_of::<sa_family_t>() + // sa_family
        size_of::<[u8; 14]>(); // sa_data
}

impl fmt::Debug for sockaddr {
    fn fmt(&self, f: &mut ::core::fmt::Formatter<'_>) -> ::core::fmt::Result {
        f.debug_struct("sockaddr")
            .field("sa_len", &self.sa_len)
            .field("sa_family", &self.sa_family)
            .field("sa_data", &self.sa_data)
            .finish()
    }
}

#[repr(C, packed)]
#[derive(Debug, Clone, Copy)]
pub struct linger {
    /// Indicates whether linger option is enabled.
    pub l_onoff: c_int,
    ///Linger  time, in seconds.
    pub l_linger: c_int,
}

//==================================================================================================
// Function Prototypes
//==================================================================================================

unsafe extern "C" {
    pub fn accept(sockfd: c_int, sockaddr: *mut sockaddr, len: *mut socklen_t) -> c_int;
    pub fn bind(sockfd: c_int, sockaddr: *const sockaddr, len: socklen_t) -> c_int;
    pub fn connect(sockfd: c_int, sockaddr: *const sockaddr, len: socklen_t) -> c_int;
    pub fn getpeername(sockfd: c_int, sockaddr: *mut sockaddr, len: *mut socklen_t) -> c_int;
    pub fn getsockname(sockfd: c_int, sockaddr: *mut sockaddr, len: *mut socklen_t) -> c_int;
    pub fn getsockopt(
        sockfd: c_int,
        level: c_int,
        optname: c_int,
        optval: *mut c_void,
        optlen: *mut socklen_t,
    ) -> c_int;
    pub fn listen(sockfd: c_int, backlog: c_int) -> c_int;
    #[cfg(target_pointer_width = "32")]
    pub fn recv(sockfd: c_int, buf: *mut c_void, len: size_t, flags: c_int) -> ssize_t;
    #[cfg(target_pointer_width = "32")]
    pub fn recvfrom(
        sockfd: c_int,
        buf: *mut c_void,
        len: size_t,
        flags: c_int,
        sockaddr: *mut sockaddr,
        socklen: *mut socklen_t,
    ) -> ssize_t;
    #[cfg(target_pointer_width = "32")]
    pub fn recvmsg(sockfd: c_int, msg: *mut msghdr, flags: c_int) -> ssize_t;
    #[cfg(target_pointer_width = "32")]
    pub fn sendmsg(sockfd: c_int, msg: *const msghdr, flags: c_int) -> ssize_t;
    #[cfg(target_pointer_width = "32")]
    pub fn sendto(
        sockfd: c_int,
        buf: *const c_void,
        len: size_t,
        flags: c_int,
        sockaddr: *const sockaddr,
        addrlen: socklen_t,
    ) -> ssize_t;
    #[cfg(target_pointer_width = "32")]
    pub fn setsockopt(
        sockfd: c_int,
        level: c_int,
        optname: c_int,
        optval: *const c_void,
        optlen: socklen_t,
    ) -> c_int;
    pub fn socket(domain: c_int, typ: c_int, protocol: c_int) -> c_int;
    pub fn shutdown(sockfd: c_int, how: c_int) -> c_int;
    pub fn socketpair(domain: c_int, typ: c_int, protocol: c_int, socket_fds: *mut c_int) -> c_int;
}
