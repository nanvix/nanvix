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
// Structures
//==================================================================================================

/// A host entry in the network database.
#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct hostent {
    pub h_name: *const c_char,
    pub h_aliases: *const *const c_char,
    pub h_addrtype: c_int,
    pub h_length: c_int,
    pub h_addr_list: *const *const c_char,
}

/// A network entry in the network database.
#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct netent {
    pub n_name: *const c_char,
    pub n_aliases: *const *const c_char,
    pub n_addrtype: c_int,
    pub n_net: u32,
}

/// A service entry in the network database.
#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct servent {
    pub s_name: *const c_char,
    pub s_aliases: *const *const c_char,
    pub s_port: c_int,
    pub s_proto: *const c_char,
}

/// A protocol entry in the network database.
#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct protoent {
    pub p_name: *const c_char,
    pub p_aliases: *const *const c_char,
    pub p_proto: c_int,
}

/// An address information in the network database.
#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct addrinfo {
    pub ai_flags: c_int,
    pub ai_family: c_int,
    pub ai_socktype: c_int,
    pub ai_protocol: c_int,
    pub ai_addrlen: socklen_t,
    pub ai_canonname: *const c_char,
    pub ai_addr: *const sockaddr,
    pub ai_next: *mut addrinfo,
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
