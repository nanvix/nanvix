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
    c_char,
    c_uint,
};

//==================================================================================================
// Constants
//==================================================================================================

/// Maximum network interface name length, including the null terminator.
pub const IF_NAMESIZE: usize = 16;
/// Alias for [`IF_NAMESIZE`].
pub const IFNAMSIZ: usize = IF_NAMESIZE;

//==================================================================================================
// Structures
//==================================================================================================

/// Associates a network interface index with its name.
#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct if_nameindex {
    /// Network interface index.
    pub if_index: c_uint,
    /// Null-terminated network interface name.
    pub if_name: *mut c_char,
}
