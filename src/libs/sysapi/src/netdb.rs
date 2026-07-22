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
    ffi::{
        c_char,
        c_int,
    },
    sys_socket::{
        sockaddr,
        socklen_t,
    },
};
use ::core::mem;

//==================================================================================================
// Constants
//==================================================================================================

/// Returns socket addresses suitable for binding a passive socket.
pub const AI_PASSIVE: c_int = 0x0001;
/// Requests the canonical name of the host.
pub const AI_CANONNAME: c_int = 0x0002;
/// Interprets the node only as a numeric address string.
pub const AI_NUMERICHOST: c_int = 0x0004;
/// Interprets the service only as a numeric port string.
pub const AI_NUMERICSERV: c_int = 0x0008;
/// Returns IPv4-mapped IPv6 addresses when no IPv6 addresses are found.
pub const AI_V4MAPPED: c_int = 0x0800;
/// Returns both IPv4 and IPv6 addresses with [`AI_V4MAPPED`].
pub const AI_ALL: c_int = 0x0100;
/// Returns addresses only for configured address families.
pub const AI_ADDRCONFIG: c_int = 0x0400;

/// Requests a numeric host string from `getnameinfo()`.
pub const NI_NUMERICHOST: c_int = 0x0001;
/// Requests a numeric service string from `getnameinfo()`.
pub const NI_NUMERICSERV: c_int = 0x0002;
/// Requests only the node name portion of a fully qualified domain name.
pub const NI_NOFQDN: c_int = 0x0004;
/// Reports an error when the host name cannot be resolved.
pub const NI_NAMEREQD: c_int = 0x0008;
/// Indicates that the service is a datagram service.
pub const NI_DGRAM: c_int = 0x0010;
/// Maximum host-name buffer length used by `getnameinfo()`.
pub const NI_MAXHOST: c_int = 1025;
/// Maximum service-name buffer length used by `getnameinfo()`.
pub const NI_MAXSERV: c_int = 32;

/// Invalid flags were supplied to `getaddrinfo()`.
pub const EAI_BADFLAGS: c_int = -1;
/// A name does not resolve for the supplied parameters.
pub const EAI_NONAME: c_int = -2;
/// A temporary failure occurred in name resolution.
pub const EAI_AGAIN: c_int = -3;
/// A non-recoverable failure occurred in name resolution.
pub const EAI_FAIL: c_int = -4;
/// The requested address family is not supported.
pub const EAI_FAMILY: c_int = -6;
/// The requested socket type is not supported.
pub const EAI_SOCKTYPE: c_int = -7;
/// The requested service is not available for the socket type.
pub const EAI_SERVICE: c_int = -8;
/// A memory allocation failed.
pub const EAI_MEMORY: c_int = -10;
/// A system error occurred; details are available through `errno`.
pub const EAI_SYSTEM: c_int = -11;
/// An argument buffer was too small.
pub const EAI_OVERFLOW: c_int = -12;

/// The requested host was not found.
pub const HOST_NOT_FOUND: c_int = 1;
/// A temporary name-resolution failure occurred.
pub const TRY_AGAIN: c_int = 2;
/// A non-recoverable name-resolution failure occurred.
pub const NO_RECOVERY: c_int = 3;
/// The host exists but has no address data.
pub const NO_DATA: c_int = 4;
/// Alias for [`NO_DATA`].
pub const NO_ADDRESS: c_int = NO_DATA;

//==================================================================================================
// Structures
//==================================================================================================

/// A host entry in the network database.
#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct hostent {
    /// Official host name.
    pub h_name: *const c_char,
    /// Null-terminated list of aliases.
    pub h_aliases: *const *const c_char,
    /// Host address family.
    pub h_addrtype: c_int,
    /// Length of each host address.
    pub h_length: c_int,
    /// Null-terminated list of host addresses.
    pub h_addr_list: *const *const c_char,
}

/// A network entry in the network database.
#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct netent {
    /// Official network name.
    pub n_name: *const c_char,
    /// Null-terminated list of aliases.
    pub n_aliases: *const *const c_char,
    /// Network address family.
    pub n_addrtype: c_int,
    /// Network number.
    pub n_net: u32,
}

/// A service entry in the network database.
#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct servent {
    /// Official service name.
    pub s_name: *const c_char,
    /// Null-terminated list of aliases.
    pub s_aliases: *const *const c_char,
    /// Service port number in network byte order.
    pub s_port: c_int,
    /// Protocol used by the service.
    pub s_proto: *const c_char,
}

/// A protocol entry in the network database.
#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct protoent {
    /// Official protocol name.
    pub p_name: *const c_char,
    /// Null-terminated list of aliases.
    pub p_aliases: *const *const c_char,
    /// Protocol number.
    pub p_proto: c_int,
}

/// An address information in the network database.
#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct addrinfo {
    /// Input flags controlling address resolution.
    pub ai_flags: c_int,
    /// Address family.
    pub ai_family: c_int,
    /// Socket type.
    pub ai_socktype: c_int,
    /// Protocol number.
    pub ai_protocol: c_int,
    /// Length of the socket address.
    pub ai_addrlen: socklen_t,
    /// Canonical host name, when requested.
    pub ai_canonname: *const c_char,
    /// Resolved socket address.
    pub ai_addr: *const sockaddr,
    /// Next result in the linked list.
    pub ai_next: *mut Self,
}

//==================================================================================================
// Layout Assertions
//==================================================================================================

const fn align_to(value: usize, alignment: usize) -> usize {
    value.div_ceil(alignment) * alignment
}

::static_assert::assert_eq_size!(
    hostent,
    3 * mem::size_of::<*const c_char>() + 2 * mem::size_of::<c_int>()
);
::static_assert::assert_eq_align!(hostent, mem::align_of::<*const c_char>());
::static_assert::assert_eq!(mem::offset_of!(hostent, h_name) == 0);
::static_assert::assert_eq!(mem::offset_of!(hostent, h_aliases) == mem::size_of::<*const c_char>());
::static_assert::assert_eq!(
    mem::offset_of!(hostent, h_addrtype) == 2 * mem::size_of::<*const c_char>()
);
::static_assert::assert_eq!(
    mem::offset_of!(hostent, h_length)
        == 2 * mem::size_of::<*const c_char>() + mem::size_of::<c_int>()
);
::static_assert::assert_eq!(
    mem::offset_of!(hostent, h_addr_list)
        == 2 * mem::size_of::<*const c_char>() + 2 * mem::size_of::<c_int>()
);

::static_assert::assert_eq_size!(
    netent,
    2 * mem::size_of::<*const c_char>() + mem::size_of::<c_int>() + mem::size_of::<u32>()
);
::static_assert::assert_eq_align!(netent, mem::align_of::<*const c_char>());
::static_assert::assert_eq!(mem::offset_of!(netent, n_name) == 0);
::static_assert::assert_eq!(mem::offset_of!(netent, n_aliases) == mem::size_of::<*const c_char>());
::static_assert::assert_eq!(
    mem::offset_of!(netent, n_addrtype) == 2 * mem::size_of::<*const c_char>()
);
::static_assert::assert_eq!(
    mem::offset_of!(netent, n_net) == 2 * mem::size_of::<*const c_char>() + mem::size_of::<c_int>()
);

const SERVENT_S_PROTO_OFFSET: usize = align_to(
    2 * mem::size_of::<*const c_char>() + mem::size_of::<c_int>(),
    mem::align_of::<*const c_char>(),
);
::static_assert::assert_eq_size!(servent, SERVENT_S_PROTO_OFFSET + mem::size_of::<*const c_char>());
::static_assert::assert_eq_align!(servent, mem::align_of::<*const c_char>());
::static_assert::assert_eq!(mem::offset_of!(servent, s_name) == 0);
::static_assert::assert_eq!(mem::offset_of!(servent, s_aliases) == mem::size_of::<*const c_char>());
::static_assert::assert_eq!(
    mem::offset_of!(servent, s_port) == 2 * mem::size_of::<*const c_char>()
);
::static_assert::assert_eq!(mem::offset_of!(servent, s_proto) == SERVENT_S_PROTO_OFFSET);

::static_assert::assert_eq_size!(
    protoent,
    align_to(
        2 * mem::size_of::<*const c_char>() + mem::size_of::<c_int>(),
        mem::align_of::<*const c_char>(),
    )
);
::static_assert::assert_eq_align!(protoent, mem::align_of::<*const c_char>());
::static_assert::assert_eq!(mem::offset_of!(protoent, p_name) == 0);
::static_assert::assert_eq!(
    mem::offset_of!(protoent, p_aliases) == mem::size_of::<*const c_char>()
);
::static_assert::assert_eq!(
    mem::offset_of!(protoent, p_proto) == 2 * mem::size_of::<*const c_char>()
);

const ADDRINFO_AI_CANONNAME_OFFSET: usize = align_to(
    4 * mem::size_of::<c_int>() + mem::size_of::<socklen_t>(),
    mem::align_of::<*const c_char>(),
);
::static_assert::assert_eq_size!(
    addrinfo,
    ADDRINFO_AI_CANONNAME_OFFSET + 3 * mem::size_of::<*const c_char>()
);
::static_assert::assert_eq_align!(addrinfo, mem::align_of::<*const c_char>());
::static_assert::assert_eq!(mem::offset_of!(addrinfo, ai_flags) == 0);
::static_assert::assert_eq!(mem::offset_of!(addrinfo, ai_family) == mem::size_of::<c_int>());
::static_assert::assert_eq!(mem::offset_of!(addrinfo, ai_socktype) == 2 * mem::size_of::<c_int>());
::static_assert::assert_eq!(mem::offset_of!(addrinfo, ai_protocol) == 3 * mem::size_of::<c_int>());
::static_assert::assert_eq!(mem::offset_of!(addrinfo, ai_addrlen) == 4 * mem::size_of::<c_int>());
::static_assert::assert_eq!(
    mem::offset_of!(addrinfo, ai_canonname) == ADDRINFO_AI_CANONNAME_OFFSET
);
::static_assert::assert_eq!(
    mem::offset_of!(addrinfo, ai_addr)
        == ADDRINFO_AI_CANONNAME_OFFSET + mem::size_of::<*const c_char>()
);
::static_assert::assert_eq!(
    mem::offset_of!(addrinfo, ai_next)
        == ADDRINFO_AI_CANONNAME_OFFSET + 2 * mem::size_of::<*const c_char>()
);
