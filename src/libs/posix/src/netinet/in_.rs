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

//==================================================================================================

/// Used for internet ports.
pub type in_port_t = u16;

/// Used for internet addresses.
pub type in_addr_t = u32;

/// Describes an internet address.
pub struct in_addr {
    pub s_addr: in_addr_t,
}

/// Describes an internet socket address.
#[repr(C, packed)]
pub struct sockaddr_in {
    /// Address family.
    pub sin_family: sa_family_t,
    /// Port number.
    pub sin_port: in_port_t,
    /// Internet address.
    pub sin_addr: in_addr,
    /// Padding.
    pub sin_zero: [u8; 8],
}
::nvx::sys::static_assert_size!(sockaddr_in, 16);
