// Copyright(c) The Maintainers of Nanvix.
// Licensed under the MIT License.

//==================================================================================================
// Configuration
//==================================================================================================

#![allow(non_camel_case_types)]

//==================================================================================================
// Imports
//==================================================================================================

use crate::ffi::c_ulong;

//==================================================================================================
// Types
//==================================================================================================

/// Used for resource limit values.
pub type rlim_t = c_ulong;

//===================================================================================================
// Structures
//===================================================================================================

///
/// # Description
///
#[derive(Debug, Clone, Copy)]
#[repr(C, packed)]
pub struct rlimit {
    /// Soft limit.
    pub rlim_cur: rlim_t,
    /// Hard limit.
    pub rlim_max: rlim_t,
}
::static_assert::assert_eq_size!(rlimit, rlimit::_SIZE);

impl rlimit {
    /// Size of `rlim_cur` field, used for static size assertions.
    pub const _RLIM_CUR_SIZE: usize = core::mem::size_of::<rlim_t>();
    /// Size of `rlim_max` field, used for static size assertions.
    pub const _RLIM_MAX_SIZE: usize = core::mem::size_of::<rlim_t>();
    /// Size of the `rlimit` structure, used for static size assertions.
    pub const _SIZE: usize = Self::_RLIM_CUR_SIZE + Self::_RLIM_MAX_SIZE;
}
