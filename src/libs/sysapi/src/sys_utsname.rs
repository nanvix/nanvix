// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Configuration
//==================================================================================================

#![allow(non_camel_case_types)]

//==================================================================================================
// Imports
//==================================================================================================

use crate::ffi::c_char;

//==================================================================================================
// Constants
//==================================================================================================

/// Length of each field in [`utsname`].
pub const UTSNAME_LENGTH: usize = 64;

//==================================================================================================
// Structures
//==================================================================================================

/// Identifies the operating system and machine.
#[derive(Debug, Clone)]
#[repr(C)]
pub struct utsname {
    /// Operating system name.
    pub sysname: [c_char; UTSNAME_LENGTH],
    /// Network node name.
    pub nodename: [c_char; UTSNAME_LENGTH],
    /// Operating system release.
    pub release: [c_char; UTSNAME_LENGTH],
    /// Operating system version.
    pub version: [c_char; UTSNAME_LENGTH],
    /// Hardware identifier.
    pub machine: [c_char; UTSNAME_LENGTH],
}

::static_assert::assert_eq_size!(utsname, 5 * UTSNAME_LENGTH);
::static_assert::assert_eq_align!(utsname, ::core::mem::align_of::<c_char>());
