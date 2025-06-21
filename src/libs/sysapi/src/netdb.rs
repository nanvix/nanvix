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

//==================================================================================================
// Structures
//==================================================================================================

/// A host entry in the network database.
#[derive(Debug, Clone, Copy)]
#[repr(C, packed)]
pub struct hostent {
    pub h_name: *const c_char,
    pub h_aliases: *const *const c_char,
    pub h_addrtype: c_int,
    pub h_length: c_int,
    pub h_addr_list: *const *const c_char,
}

/// A network entry in the network database.
#[derive(Debug, Clone, Copy)]
#[repr(C, packed)]
pub struct netent {
    pub n_name: *const c_char,
    pub n_aliases: *const *const c_char,
    pub n_addrtype: c_int,
    pub n_net: u32,
}

/// A service entry in the network database.
#[derive(Debug, Clone, Copy)]
#[repr(C, packed)]
pub struct servent {
    pub s_name: *const c_char,
    pub s_aliases: *const *const c_char,
    pub s_port: c_int,
    pub s_proto: *const c_char,
}

/// A protocol entry in the network database.
#[derive(Debug, Clone, Copy)]
#[repr(C, packed)]
pub struct protoent {
    pub p_name: *const c_char,
    pub p_aliases: *const *const c_char,
    pub p_proto: c_int,
}

/// An address information in the network database.
#[derive(Debug, Clone, Copy)]
#[repr(C, packed)]
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
