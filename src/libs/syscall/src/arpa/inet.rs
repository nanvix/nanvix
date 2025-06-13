// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Configuration
//==================================================================================================

#![allow(non_camel_case_types)]

//==================================================================================================
// Imports
//==================================================================================================

use ::core::mem;

//==================================================================================================
// C Interface
//==================================================================================================

/// Used for internet addresses.
pub type in_addr_t = u32;

/// Describes an internet address.
#[derive(Debug)]
#[repr(C, packed)]
pub struct in_addr {
    pub s_addr: in_addr_t,
}
::static_assert::assert_eq_size!(in_addr, in_addr::_SIZE);

impl in_addr {
    /// Size of this structure, used for static assertions.
    const _SIZE: usize = mem::size_of::<in_addr_t>(); // s_addr
}
