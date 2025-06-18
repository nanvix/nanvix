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
        c_uchar,
    },
    sys_socket::{
        sa_family_t,
        sockaddr_storage,
    },
};
use ::core::mem::size_of;

//==================================================================================================
// Constants
//==================================================================================================

/// Size of the `sun_path` field in `sockaddr_un`.
pub const SUNPATHLEN: usize = 14;

//==================================================================================================
// Structures
//==================================================================================================

/// AUNIX domain socket address.
#[repr(C, packed)]
#[derive(Clone, Debug)]
pub struct sockaddr_un {
    /// Total length.
    pub sun_len: c_uchar,
    /// Address family.
    pub sun_family: sa_family_t,
    /// Path.
    pub sun_path: [c_char; SUNPATHLEN],
}
::static_assert::assert_eq_size!(sockaddr_un, sockaddr_un::_SIZE);
::static_assert::assert_eq_size!(sockaddr_un, size_of::<sockaddr_storage>());

impl sockaddr_un {
    /// Size of this structure, used for static assertions.
    pub const _SIZE: usize = size_of::<c_uchar>() + // sun_len
            size_of::<sa_family_t>() + // sun_family
            SUNPATHLEN*size_of::<c_char>(); // sun_path
}
