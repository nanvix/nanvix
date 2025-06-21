// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Configuration
//==================================================================================================

#![allow(non_camel_case_types)]

//==================================================================================================
// Modules
//==================================================================================================

use ::sysapi::ffi::c_char;

cfg_if::cfg_if! {
    if #[cfg(feature = "syscall")] {
        mod syscall;
        pub use self::syscall::{
            uname,
        };
    }
}

//==================================================================================================
// Constants
//==================================================================================================

/// Length for c-style strings ins [`utsname`].
const UTSNAME_LENGTH: usize = 64;

//==================================================================================================
// Structures
//==================================================================================================

#[repr(C, packed)]
#[derive(Debug, Clone)]
pub struct utsname {
    /// Name of this implementation of the operating system.
    pub sysname: [c_char; UTSNAME_LENGTH],
    /// Name of this node within the communications network to which this node is attached, if any.
    pub nodename: [c_char; UTSNAME_LENGTH],
    /// Current release level of this implementation.
    pub release: [c_char; UTSNAME_LENGTH],
    /// Current version level of this release.
    pub version: [c_char; UTSNAME_LENGTH],
    /// Name of the hardware type on which the system is running.
    pub machine: [c_char; UTSNAME_LENGTH],
}
::static_assert::assert_eq_size!(utsname, utsname::_SIZE);

impl utsname {
    // Size of `sysname` field, used for static size assertions.
    const _SYSNAME_SIZE: usize = UTSNAME_LENGTH;
    // Size of `nodename` field, used for static size assertions.
    const _NODENAME_SIZE: usize = UTSNAME_LENGTH;
    // Size of `release` field, used for static size assertions.
    const _RELEASE_SIZE: usize = UTSNAME_LENGTH;
    // Size of `version` field, used for static size assertions.
    const _VERSION_SIZE: usize = UTSNAME_LENGTH;
    // Size of `machine` field, used for static size assertions.
    const _MACHINE_SIZE: usize = UTSNAME_LENGTH;

    // Size of this structure, used for static size assertions.
    const _SIZE: usize = Self::_SYSNAME_SIZE
        + Self::_NODENAME_SIZE
        + Self::_RELEASE_SIZE
        + Self::_VERSION_SIZE
        + Self::_MACHINE_SIZE;
}
