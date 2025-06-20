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
    c_short,
    c_uint,
};

//==================================================================================================
// Types
//==================================================================================================

// Used for the number of file descriptors.
pub type nfds_t = c_uint;

//==================================================================================================
// Structures
//==================================================================================================

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct pollfd {
    /// The following descriptor being polled.
    pub fd: c_int,
    /// Input event flags.
    pub events: c_short,
    /// Output event flags.
    pub revents: c_short,
}
::static_assert::assert_eq_size!(pollfd, pollfd::_SIZE);

impl pollfd {
    /// Size of the `pollfd` structure.
    pub const _SIZE: usize = ::core::mem::size_of::<c_int>() + // fd
        ::core::mem::size_of::<c_short>() + // events
        ::core::mem::size_of::<c_short>(); // revents
}
