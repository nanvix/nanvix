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
use ::core::mem;

//==================================================================================================
// Constants
//==================================================================================================

/// Size of the `sun_path` field in [`sockaddr_un`].
pub const SUNPATHLEN: usize = 104;

//==================================================================================================
// Structures
//==================================================================================================

/// Describes a UNIX domain socket address.
#[repr(C, packed)]
pub struct sockaddr_un {
    /// Address family.
    pub sun_family: sa_family_t,
    /// Path.
    pub sun_path: [u8; SUNPATHLEN],
}
::nvx::sys::static_assert_size!(sockaddr_un, sockaddr_un::SIZE);

impl sockaddr_un {
    /// Size of the structure.
    pub const SIZE: usize = mem::size_of::<sa_family_t>() + SUNPATHLEN;
}
